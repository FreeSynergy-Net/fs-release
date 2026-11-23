#!/usr/bin/env python3
"""Point every dependency on the fs-components git repo at `tag = "<tag>"` (rev/branch are removed).

    scripts/set_components_tag.py <tag> <repo root> [fs-repo-name]      (name default fs-components, e.g. fs-interfaces) exit 0: changed or already on the tag; 3: no such dependency; 1: could not
    scripts/set_components_tag.py --move <fs-name> <tag> <repo root>    (e.g. --move fs-types v0.0.1: a dependency named <fs-name> or with `package = "<fs-name>"` is moved from the
                                                                       repo fs-interfaces to github.com/FreeSynergy-Net/<fs-name> at the tag; same exit codes)

Pattern: none (helper of bump-components.sh).
The files are edited line by line (comments and layout stay), the parser is the judge: after the edit every
dependency whose `git` is the fs-components repo, under any name or `package =`, in `dependencies`,
`dev-`/`build-dependencies`, `target.*` and `workspace.dependencies`, must have exactly `tag = <tag>` and no rev/branch.
Handled forms: inline tables, `[dependencies.name]` tables (also `[target.'cfg(..)'.dependencies.name]`).
A `[patch]`/`[replace]` on the fs-components repo is an error (exit 1, nothing written): never set silently there.
Anything else fails the check and exits 1, nothing is written then.
V1+ (ADR 0092/0093): with the repo fs-interfaces a dependency named `fs-types` (or `package = "fs-types"`) is NEVER touched, whatever its git URL (it lived in the
repo fs-interfaces up to v0.0.26; the lift of fs-interfaces must not drag it to the interfaces tag). `--move` is the only way to change it: the entry gets
`git = "https://github.com/FreeSynergy-Net/<fs-name>"` and `tag = <tag>` (rev/branch removed); afterwards the parser checks that no such dependency points at fs-interfaces
any more. A dependency of that name with a git URL that is neither fs-interfaces nor <fs-name> is an error (exit 1, nothing written): it is never guessed.
Entries without `git` (`fs-types.workspace = true`, path, registry) are left alone.
"""
import re, sys, tomllib
from pathlib import Path

REPO = "fs-components"                                      # `set_components_tag.py <tag> <root> fs-interfaces` switches it
REPO_URL = re.compile(r"/fs-components(\.git)?/?$", re.I)
PROTECT = None                                              # a dependency name the lift of REPO never touches (fs-types with fs-interfaces)
MOVE = None                                                 # --move <name>: the dependency that changes its repo
OLD_HOME = "fs-interfaces"                                  # where a moved dependency came from
HOME = "https://github.com/FreeSynergy-Net"
SKIP = {"target", ".git", "node_modules"}
VERSION_KEYS = ("rev", "branch", "tag")

def cargo_tomls(root: Path):
    """Every Cargo.toml below root, but not inside target/, .git/ or nested worktrees (.wt-*)."""
    for f in sorted(root.rglob("Cargo.toml")):
        parts = f.relative_to(root).parts[:-1]
        if any(p in SKIP or p.startswith(".wt-") for p in parts): continue
        yield f

def url_of(name: str):
    return re.compile(rf"/{name}(\.git)?/?$", re.I)

def named(name: str, pkg, lib) -> bool:
    """The dependency is `lib`: its key, or its `package =` when it has one."""
    return lib is not None and (pkg or name) == lib

def target(name: str, pkg, url) -> str | None:
    """The git URL this entry is set to, or None when the lift leaves it alone. Normal mode: the repo's own URL (kept), except the PROTECTed name;
    --move mode: only the moved name, from fs-interfaces or the new repo, to the new repo."""
    if not isinstance(url, str): return None
    if MOVE:
        old_or_new = any(re.fullmatch(rf"https?://github\.com/FreeSynergy-Net/{n}(\.git)?/?", url, re.I) for n in (OLD_HOME, MOVE))
        return f"{HOME}/{MOVE}" if named(name, pkg, MOVE) and old_or_new else None
    return url if REPO_URL.search(url) and not named(name, pkg, PROTECT) else None

def is_components(name: str, spec) -> bool:
    return isinstance(spec, dict) and target(name, spec.get("package"), spec.get("git")) is not None

def dep_tables(doc):
    for k, v in doc.items():
        if k in ("dependencies", "dev-dependencies", "build-dependencies") and isinstance(v, dict): yield v
    for t in (doc.get("target") or {}).values():
        if isinstance(t, dict): yield from dep_tables(t)
    if isinstance(doc.get("workspace"), dict): yield from dep_tables(doc["workspace"])

def components_deps(doc):
    return [(n, s) for t in dep_tables(doc) for n, s in t.items() if is_components(n, s)]

def protected_deps(doc):
    """Entries the lift must leave exactly as they are (the PROTECTed name, any URL)."""
    return {n: s for t in dep_tables(doc) for n, s in t.items() if isinstance(s, dict) and named(n, s.get("package"), PROTECT)}

def foreign_moved(doc):
    """--move: dependencies of the moved name with a git URL that is neither fs-interfaces nor the new repo (never guessed)."""
    return [(n, s["git"]) for t in dep_tables(doc) for n, s in t.items()
            if isinstance(s, dict) and named(n, s.get("package"), MOVE) and isinstance(s.get("git"), str) and target(n, s.get("package"), s["git"]) is None]

def _git_val(line: str):
    return re.search(r"\bgit\s*=\s*(\"([^\"]*)\"|'([^']*)')", line)

def _pkg(text: str):
    m = re.search(r"\bpackage\s*=\s*[\"']([^\"']+)[\"']", text)
    return m.group(1) if m else None

def _key(line: str):
    m = re.match(r"\s*(?:\"([^\"]+)\"|'([^']+)'|([A-Za-z0-9_-]+))\s*=", line)
    return next((g for g in m.groups() if g), None) if m else None

def _inline(line: str, tag: str) -> str:
    """`name = { git = "…/fs-components", rev = "x" }` -> tag = "<tag>" right after git (the URL is replaced under --move)."""
    g = _git_val(line)
    new = target(_key(line) or "", _pkg(line), g.group(2) or g.group(3) if g else None)
    if new is None: return line
    val = r"(\"[^\"]*\"|'[^']*')"
    line = re.sub(rf"\b(rev|branch|tag)\s*=\s*{val}\s*,\s*", "", line)       # `{ tag = "v1", git = .. }`: key followed by comma
    line = re.sub(rf",\s*\b(rev|branch|tag)\s*=\s*{val}", "", line)          # key at the end
    return re.sub(r"\bgit\s*=\s*(\"[^\"]*\"|'[^']*')", lambda m: f'git = "{new}", tag = "{tag}"', line, count=1)

def _mentions(*parts) -> bool:
    return any(n in " ".join(str(p) for p in parts).lower() for n in {REPO, MOVE} - {None})

def patch_hits(doc):
    """[patch.<url>] / [replace] entries naming fs-components in url, key or value (path, git, anything): never edited, reported.
    Also a non-empty `paths` (cargo config)."""
    hits = [f"[patch.{src}] {n}" for src, e in (doc.get("patch") or {}).items() if isinstance(e, dict)
            for n, s in e.items() if _mentions(src, n, s)]
    hits += [f"[replace] {n}" for n, s in (doc.get("replace") or {}).items() if _mentions(n, s)]
    return hits + (["paths"] if doc.get("paths") else [])

def config_files(root: Path):
    """`.cargo/config(.toml)` below root (same skips) and the files they `include`."""
    seen = set()
    def load(f):
        f = f.resolve()
        if f in seen or not f.is_file(): return
        seen.add(f)
        try: doc = tomllib.loads(f.read_text(encoding="utf-8"))
        except tomllib.TOMLDecodeError: yield f, None; return
        yield f, doc
        inc = doc.get("include") or []
        for i in ([inc] if isinstance(inc, (str, dict)) else inc):
            p = i.get("path") if isinstance(i, dict) else i
            if isinstance(p, str): yield from load(f.parent / p)
    for n in ("config.toml", "config"):
        for f in sorted(root.rglob(f".cargo/{n}")):
            parts = f.relative_to(root).parts[:-2]
            if any(p in SKIP or p.startswith(".wt-") for p in parts): continue
            yield from load(f)

def edit(text: str, tag: str) -> str:
    out, section, skip, head = [], [], [False], [""]           # section: lines of the current [table] (header excluded); head: its header
    def flush():
        body = "".join(section)
        g = _git_val(next((l for l in section if re.match(r"\s*git\s*=", l)), ""))
        name = head[0].rstrip().rstrip("]").rsplit(".", 1)[-1].strip().strip("\"'")
        new = None if skip[0] or not g else target(name, _pkg(body), g.group(2) or g.group(3))
        if new is not None:
            kept = [l for l in section if not re.match(r"\s*(rev|branch|tag)\s*=", l)]
            i = next(i for i, l in enumerate(kept) if re.match(r"\s*git\s*=", l))
            kept[i:i + 1] = [f'git = "{new}"\n', f'tag = "{tag}"\n']
            section[:] = kept
        out.extend(section); section.clear()
    for line in text.splitlines(keepends=True):
        if re.match(r"\s*\[", line):
            flush(); out.append(line); head[0] = line.strip()
            skip[0] = bool(re.match(r"\s*\[+\s*(patch|replace)\b", line))      # never touch [patch]/[replace] sections
        else:
            section.append(_inline(line, tag) if "=" in line and "{" in line and not skip[0] else line)
    flush()
    return "".join(out)

def bad_deps(ndoc, tag: str) -> list:
    """What did not end up as wanted: tag, no rev/branch; under --move also the new git URL and nothing left on fs-interfaces."""
    bad = [n for n, s in components_deps(ndoc) if s.get("tag") != tag or "rev" in s or "branch" in s]
    if MOVE:
        bad += [n for n, s in components_deps(ndoc) if s["git"] != f"{HOME}/{MOVE}"]
        bad += [f"{n} (still on {OLD_HOME})" for t in dep_tables(ndoc) for n, s in t.items()
                if isinstance(s, dict) and named(n, s.get("package"), MOVE) and isinstance(s.get("git"), str) and url_of(OLD_HOME).search(s["git"])]
    return bad

def main(argv):
    global REPO, REPO_URL, PROTECT, MOVE
    if argv[1:2] == ["--move"]:
        if len(argv) != 5 or not re.fullmatch(r"fs-[a-z][a-z0-9-]*", argv[2]): print(__doc__, file=sys.stderr); return 2
        MOVE, tag, root, REPO = argv[2], argv[3], Path(argv[4]), OLD_HOME
        REPO_URL = url_of(REPO)
    else:
        if len(argv) not in (3, 4): print(__doc__, file=sys.stderr); return 2
        tag, root = argv[1], Path(argv[2])
        if len(argv) == 4:
            if not re.fullmatch(r"fs-[a-z][a-z0-9-]*", argv[3]): print(f"not a repo name: {argv[3]}", file=sys.stderr); return 2
            REPO = argv[3]; REPO_URL = url_of(REPO)
        PROTECT = "fs-types" if REPO == "fs-interfaces" else None
    found, writes = 0, {}
    for f, doc in config_files(root):
        if doc is None: print(f"{f}: not valid TOML", file=sys.stderr); return 1
        if patch_hits(doc):
            print(f"{f}: cargo config overrides {REPO} ({', '.join(patch_hits(doc))}): it hides the tag; remove it by hand", file=sys.stderr); return 1
    for f in cargo_tomls(root):
        text = f.read_text(encoding="utf-8")
        try: doc = tomllib.loads(text)
        except tomllib.TOMLDecodeError as e: print(f"{f}: not valid TOML ({e})", file=sys.stderr); return 1
        if patch_hits(doc):
            print(f"{f}: [patch]/[replace] on {REPO} ({', '.join(patch_hits(doc))}): a patch hides the tag; remove it by hand", file=sys.stderr); return 1
        if MOVE and foreign_moved(doc):
            print(f"{f}: {MOVE} has a git URL that is neither {OLD_HOME} nor {MOVE}: {foreign_moved(doc)}; not guessing, edit by hand", file=sys.stderr); return 1
        if not components_deps(doc): continue
        found += len(components_deps(doc))
        new = edit(text, tag)
        try: ndoc = tomllib.loads(new)
        except tomllib.TOMLDecodeError as e: print(f"{f}: edit broke the file ({e})", file=sys.stderr); return 1
        bad = bad_deps(ndoc, tag)
        if len(components_deps(ndoc)) != len(components_deps(doc)): bad.append("(a dependency lost its git)")
        if protected_deps(ndoc) != protected_deps(doc): bad.append(f"({PROTECT} was touched)")
        if bad:
            print(f"{f}: cannot set the tag on: {', '.join(bad)} (form not handled; edit by hand)", file=sys.stderr); return 1
        if new != text: writes[f] = new
    if not found:
        print(f"no dependency on the {MOVE or REPO} git repo found", file=sys.stderr); return 3
    for f, new in writes.items(): f.write_text(new, encoding="utf-8")
    print(f"changed {len(writes)} file(s), {found} dependencies" if writes else f"already on {tag}")
    return 0

if __name__ == "__main__":
    sys.exit(main(sys.argv))
