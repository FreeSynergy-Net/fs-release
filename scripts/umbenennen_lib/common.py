"""Shared helpers of scripts/umbenennen: names, the file lists (git ls-files), text reading, the `keep` list (L2)."""
import json
import os
import re
import subprocess

NAME = re.compile(r"fs-[a-z0-9]+(?:-[a-z0-9]+)*")
SKIP_DIRS = {".git", "target", "node_modules", "site", ".fs-release"}
BEREICHE = ("Typ", "Vertrag", "Capability-ID", "PROGRAM", "MIME", "Seitenpfad", "sonstiges")
MAX_BYTES = 2_000_000


class BadInput(Exception):
    pass


def words(old):
    """The old name, also as the start of a longer name (`fs-wiki-core`), but not inside another word (`fs-wikipedia`)."""
    return re.compile(r"(?<![\w-])" + re.escape(old) + r"(?![a-z0-9])")


def crate_re(old):
    return re.compile(r"(?<![\w-])" + re.escape(old) + r"(?:-[a-z0-9]+)*(?![\w-])")


def snake(name):
    return name.replace("-", "_")


def kinds(old, line):
    """Which forms of the old name a line carries: `name` (fs-x), `snake` (fs_x, Rust paths) and `env` (FS_X_ROOT)."""
    found = []
    if words(old).search(line):
        found.append("name")
    s = snake(old)
    if s in line and re.search(r"(?<![A-Za-z0-9_])" + re.escape(s) + r"(?![a-z0-9])", line):
        found.append("snake")
    u = s.upper()
    if u in line and re.search(r"(?<![A-Za-z0-9_])" + re.escape(u) + r"(?![A-Z0-9])", line):
        found.append("env")
    return found


def git(root, *args):
    try:
        return subprocess.run(["git", "-C", root, *args], capture_output=True, timeout=300, check=False)
    except (OSError, subprocess.TimeoutExpired):
        return None


def is_git(root):
    return os.path.exists(os.path.join(root, ".git"))


def _clean(paths, root):
    out = []
    for p in paths:
        if not p or any(part in SKIP_DIRS for part in p.split("/")):
            continue
        full = os.path.join(root, p)
        if os.path.isfile(full) and not os.path.islink(full):
            out.append(p)
    return sorted(out)


def file_lists(root):
    """(tracked, untracked) relative paths. In a git checkout: `git ls-files` (the head counts, not what lies around), untracked
    files separately (only a warning). Outside git (fixtures): every file counts as tracked."""
    if is_git(root):
        t = git(root, "ls-files", "-z")
        u = git(root, "ls-files", "-z", "--others", "--exclude-standard")
        if t is not None and t.returncode == 0:
            untracked = u.stdout.decode("utf-8", "replace").split("\0") if u is not None and u.returncode == 0 else []
            return _clean(t.stdout.decode("utf-8", "replace").split("\0"), root), _clean(untracked, root)
    found = []
    for base, dirs, names in os.walk(root):
        dirs[:] = [d for d in dirs if d not in SKIP_DIRS]
        found += [os.path.relpath(os.path.join(base, n), root) for n in names]
    return _clean(found, root), []


def text_of(path):
    try:
        if os.path.getsize(path) > MAX_BYTES:
            return None
        with open(path, encoding="utf-8") as f:
            return f.read()
    except (OSError, UnicodeDecodeError):
        return None


def lines_of(rel, text):
    """(number, line, haystack). The haystack is what `keep` patterns search: `<file> [<toml section>] <line>`."""
    section = ""
    toml = rel.endswith(".toml")
    for n, line in enumerate(text.split("\n"), 1):
        if toml:
            m = re.match(r"\s*(\[\[?[^\]]+\]\]?)\s*(#.*)?$", line)
            if m:
                section = m.group(1)
            yield n, line, f"{rel} {section} {line.strip()}"
        else:
            yield n, line, f"{rel} {line.strip()}"


def read_retired(path):
    pairs = []
    if path and os.path.exists(path):
        for line in open(path, encoding="utf-8"):
            line = line.split("#", 1)[0].strip()
            if "->" in line:
                a, b = (x.strip() for x in line.split("->", 1))
                pairs.append((a, b))
    return pairs


def toml_ids(text):
    """The catalog entries: `id = "..."` (the catalog names its packages by `id`; `name` is the display name)."""
    return re.findall(r'^\s*id\s*=\s*"([^"]+)"', text or "", re.M)


def load_keep(arg):
    """`--keep`: a JSON list of {pattern, grund, bereich} (the text itself or a file). Returns the list with compiled patterns."""
    if not arg:
        return []
    try:
        raw = arg if arg.lstrip().startswith("[") else open(arg, encoding="utf-8").read()
        data = json.loads(raw)
    except (OSError, ValueError) as e:
        raise BadInput(f"--keep is not readable JSON ({e})")
    if not isinstance(data, list):
        raise BadInput("--keep must be a JSON list of {pattern, grund, bereich}")
    out = []
    for i, e in enumerate(data):
        if not isinstance(e, dict) or not all(isinstance(e.get(k), str) and e.get(k).strip() for k in ("pattern", "grund", "bereich")):
            raise BadInput(f"--keep entry {i} needs the non-empty strings pattern, grund, bereich")
        if e["bereich"] not in BEREICHE:
            raise BadInput(f"--keep entry {i}: bereich {e['bereich']!r} is not one of {', '.join(BEREICHE)}")
        try:
            rx = re.compile(e["pattern"])
        except re.error as err:
            raise BadInput(f"--keep entry {i}: pattern is not a regular expression ({err})")
        out.append({"pattern": e["pattern"], "grund": e["grund"], "bereich": e["bereich"], "re": rx})
    return out


def keep_for(keep, hay):
    """Index of the first `keep` entry whose pattern is found in the haystack, else None."""
    for i, k in enumerate(keep):
        if k["re"].search(hay):
            return i
    return None
