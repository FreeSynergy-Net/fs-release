# Pattern: none (library of scripts/gates-komplett: the Fehlerliste and the partial check `--nur-fehlerliste`).
"""The Fehlerliste (checklist of findings with `nachpruefung:` commands) and the PARTIAL check that runs only those commands.

SECURITY (fail closed): a line of the list is data that a person (or an error text) could edit, so nothing of it reaches a shell and no word is trusted: every command is checked
against an ARGUMENT WHITELIST per program and sub-command (`check_command`); an unknown word, any `=` form, any path, any `-Z...`, `--config`, `--manifest-path`, `--target-dir`,
`-C`, an environment assignment is REFUSED and the item stays open. The list itself is read only from its own folder, as a regular file (no symlink) that the caller owns and that no
one else can write (`check_list_file`). The partial check is NEVER a gate result: its JSON has status `teilpruefung`, red true, no head and no runs, its last output line is
`TEILPRÜFUNG: kein Gate-Ergebnis, volle CI nötig`, and its exit code is never 0 (1 = something is open, 3 = everything ticked, the full CI is still needed).
"""
import datetime
import os
import re
import stat
import subprocess
import tempfile

ITEM = re.compile(r"^- \[( |x)\] (.*)$")
NAME = r"[A-Za-z0-9_][A-Za-z0-9_-]*"  # a crate, bin or test target name
TEST = r"[A-Za-z0-9_]+(?:::[A-Za-z0-9_]+)*"  # a test filter: a name or a module path
SCRIPT = r"\{SCRIPTS\}/[A-Za-z0-9_-]+(?:\.[a-z]+)?"
EXIT_OPEN, EXIT_PART_GREEN = 1, 3
LAST_LINE = "TEILPRÜFUNG: kein Gate-Ergebnis, volle CI nötig"


def utc():
    return datetime.datetime.now(datetime.timezone.utc).strftime("%Y-%m-%dT%H:%M:%SZ")


def parse_list(text):
    """The items of a Fehlerliste: [{start, end, ticked, title, nach: [command, ...]}] (line indexes); only lines `  - nachpruefung: CMD` of an item count."""
    lines, items = text.splitlines(), []
    for i, line in enumerate(lines):
        m = ITEM.match(line)
        if m:
            items.append({"start": i, "ticked": m.group(1) == "x", "title": m.group(2), "nach": []})
    for k, it in enumerate(items):
        it["end"] = items[k + 1]["start"] if k + 1 < len(items) else len(lines)
        for line in lines[it["start"] + 1:it["end"]]:
            if line.startswith("  - nachpruefung: ") and not line.startswith("  - nachpruefung: ("):
                it["nach"].append(line[len("  - nachpruefung: "):])
    return lines, items


def _options(words, allowed):
    """`words` must be options of `allowed`: {option: kind}; kind 'flag' (no value) or 'name' (followed by a crate/bin/test name). Nothing else: no positional word, no `=` form. True or False."""
    i = 0
    while i < len(words):
        kind = allowed.get(words[i])
        if kind == "flag":
            i += 1
        elif kind == "name" and i + 1 < len(words) and re.fullmatch(NAME, words[i + 1]):
            i += 2
        else:
            return False
    return True


def _split(w):
    """(the words before the first `--`, the words after it or None when there is no `--`)."""
    return (w[2:], None) if "--" not in w[2:] else (w[2:w.index("--", 2)], w[w.index("--", 2) + 1:])


def _check_argv(w):
    """True when the argument list `w` (placeholders not yet filled) is one of the allowed forms."""
    if len(w) < 2:
        return False
    if w[0] == "cargo" and w[1] == "test":
        rest, tail = _split(w)
        flags = {"-p": "name", "--package": "name", "--bin": "name", "--test": "name", "--lib": "flag", "--doc": "flag", "--workspace": "flag", "--no-fail-fast": "flag",
                 "--locked": "flag", "--all-features": "flag", "-q": "flag", "--quiet": "flag"}
        # a `--` needs at least one test filter behind it (a bare `--` is refused); filters are names or module paths, never a path
        return _options(rest, flags) and (tail is None or (len(tail) > 0 and all(re.fullmatch(TEST, t) or t in ("--exact", "--nocapture") for t in tail)))
    if w[0] == "cargo" and w[1] == "clippy":
        rest, tail = _split(w)
        flags = {"-p": "name", "--package": "name", "--workspace": "flag", "--all-targets": "flag", "--all-features": "flag", "--locked": "flag", "--keep-going": "flag"}
        # the tail of the gate is MANDATORY and exact: without it clippy would pass what the gate refuses
        return _options(rest, flags) and tail == ["-D", "warnings", "-W", "clippy::pedantic"]
    if w[0] == "cargo" and w[1] == "fmt":
        return _options(w[2:], {"--check": "flag", "--all": "flag", "-p": "name", "--package": "name"})
    if w[0] == "bash":
        return len(w) == 3 and w[1] in ("{SCRIPTS}/gates-lock.sh", "{SCRIPTS}/gates-files.sh", "{SCRIPTS}/licence-gate.sh") and w[2] == "{ROOT}"
    if w[0] == "python3":
        return w == ["python3", "-I", "{SCRIPTS}/check_rust_rules.py", "{ROOT}"]
    return False


def check_command(cmd, scripts_dir, root):
    """The argv of an allowed command with {ROOT} and {SCRIPTS} filled, or None (refused). No shell ever sees it."""
    w = cmd.split(" ")
    if not w or any(not x for x in w) or not _check_argv(w):
        return None
    return [x.replace("{SCRIPTS}", scripts_dir).replace("{ROOT}", root) for x in w]


def read_list_file(path, list_dir, uid):
    """(text, None) when `path` may be read as the Fehlerliste, else (None, reason): it is named fehlerliste-*.md, lies directly in `list_dir`, is a regular file that belongs to `uid`
    and that nobody else can write. The file is OPENED once (`O_NOFOLLOW`: a symbolic link is refused) and everything is checked on the open file (`fstat`) and read from it:
    nothing can be swapped between the check and the read."""
    if not re.fullmatch(r"fehlerliste-[A-Za-z0-9._-]+\.md", os.path.basename(path)):
        return None, "the file is not named fehlerliste-<branch>.md"
    if os.path.realpath(os.path.dirname(os.path.abspath(path))) != os.path.realpath(list_dir):
        return None, f"the file is not in the list folder {list_dir}"
    try:
        fd = os.open(path, os.O_RDONLY | os.O_NOFOLLOW | getattr(os, "O_CLOEXEC", 0))
    except OSError as e:
        return None, f"cannot open the file (a symbolic link is refused): {e.strerror}"
    try:
        st = os.fstat(fd)
        if not stat.S_ISREG(st.st_mode):
            return None, "the file is no regular file"
        if st.st_uid != uid:
            return None, "the file does not belong to the calling user"
        if st.st_mode & 0o022:
            return None, "the file is writable by others"
        with os.fdopen(fd, "r", encoding="utf-8", closefd=False) as fh:
            return fh.read(), None
    finally:
        os.close(fd)


def run_partial(path, text, repo_dir, scripts_dir, slot, env):
    """Runs the `nachpruefung` commands of the open items of the list `text` (read from `path` by `read_list_file`), ticks the green ones, rewrites the list at `path` through a
    temporary file that is created new in the same folder (`mkstemp`, mode 0600) and renamed. Returns (open count, results)."""
    lines, items = parse_list(text)
    new_lines, results = list(lines), []
    for it in reversed(items):  # from the end: the line indexes of the earlier items stay valid
        if it["ticked"]:
            continue
        reds = []
        for cmd in it["nach"]:
            argv = check_command(cmd, scripts_dir, repo_dir)
            if argv is None:
                reds.append((cmd, None, "command refused: not an allowed command"))
                continue
            with tempfile.TemporaryFile("w+", encoding="utf-8", errors="replace") as fh:
                rc = subprocess.run([slot, *argv], cwd=repo_dir, env=env, stdout=fh, stderr=subprocess.STDOUT, check=False).returncode
                fh.seek(0)
                err = [x for x in fh.read().splitlines() if re.search(r"rc=|FAILED|ERROR|errors|^error", x) and "rc=0" not in x][:6]
            if rc != 0:
                reds.append((cmd, rc, " // ".join(err)))
        body = [x for x in new_lines[it["start"] + 1:it["end"]] if not x.startswith("  - nachgeprueft:") and not x.startswith("  - letzte pruefung:")]
        if it["nach"] and not reds:
            head, add = f"- [x] {it['title']}", [f"  - nachgeprueft: {utc()}"]
            results.append({"item": it["title"], "gruen": True})
        else:
            head = f"- [ ] {it['title']}"
            add = [f"  - letzte pruefung: {utc()} " + ("; ".join(f"rc {rc}: {cmd}: {e}" for cmd, rc, e in reds) if reds else "no local check (only the full run)")]
            results.append({"item": it["title"], "gruen": False})
        new_lines[it["start"]:it["end"]] = [head, *body, *add]
    fd, tmpfile = tempfile.mkstemp(dir=os.path.dirname(os.path.abspath(path)), prefix=".fehlerliste-", suffix=".tmp")
    with os.fdopen(fd, "w", encoding="utf-8") as fh:
        fh.write("\n".join(new_lines).rstrip("\n") + "\n")
    os.replace(tmpfile, path)
    still = sum(1 for it in parse_list("\n".join(new_lines))[1] if not it["ticked"])
    return still, results[::-1]
