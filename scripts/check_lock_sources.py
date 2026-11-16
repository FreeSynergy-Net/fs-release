#!/usr/bin/env python3
"""Lock check: every shared library appears in exactly ONE source in a Cargo.lock (ADR 0092 rule 2, ADR 0093 rule 3).

    python3 -I scripts/check_lock_sources.py [--config FILE] [--only-from] [--print-strict] <Cargo.lock | repo root | ->   (`-` = lock on stdin)

Pattern: none (checker; called by scripts/gates.sh, so every repo's `check` gate runs it; self test scripts/test_check_lock_sources.sh).
The libraries are listed in scripts/lock-sources.toml (`libraries`). Two [[package]] entries with the same name (two versions,
or the same version from two git URLs/tags/commits) are an error: the message names both sources and the packages that pull
each one (a package without a source is the workspace member itself). Exit codes: 0 green (or nothing to check: no Cargo.lock),
1 violation, 2 usage/read error. A repo root is searched for every Cargo.lock outside target/ and .git.
V1+ addition (`--only-from`, config `only_from_enabled`, env FS_LOCK_ONLY_FROM=1; default OFF): a library listed in `[only_from]`
(fs-types) must come from that repo only, also when it appears once.
Whether a violation is a warning or red is decided by the caller (scripts/gates.sh, switch `strict`), not here.
"""
import os, re, sys, tomllib
from pathlib import Path

CONFIG = Path(__file__).resolve().parent / "lock-sources.toml"
DEP = re.compile(r"^(\S+)(?: (\S+))?(?: \((.*)\))?$")

def norm(url: str) -> str:
    return re.sub(r"(\.git)?/*$", "", url.strip().lower())

def repo_of(source: str) -> str:
    """git+https://host/org/repo.git?tag=x#sha -> https://host/org/repo; registry and path sources -> the source itself."""
    if not source.startswith("git+"):
        return source or "(path)"
    return norm(re.split(r"[?#]", source[4:], maxsplit=1)[0])

def load_config(path: Path, repo_root: Path | None, only_from: bool) -> dict:
    cfg = tomllib.loads(path.read_text(encoding="utf-8"))
    if repo_root is not None and (repo_root / ".fs-lock-sources.toml").is_file():
        cfg.update(tomllib.loads((repo_root / ".fs-lock-sources.toml").read_text(encoding="utf-8")))
    cfg["only_from_on"] = only_from or bool(cfg.get("only_from_enabled")) or os.environ.get("FS_LOCK_ONLY_FROM") == "1"
    return cfg

def pullers(pkgs: list, name: str, entry: dict) -> list:
    """Who depends on this entry: `name`, `name version` or `name version (source)` in a dependency list."""
    out = []
    for p in pkgs:
        for d in p.get("dependencies", []):
            m = DEP.match(d)
            if not m or m.group(1) != name:
                continue
            ver, src = m.group(2), m.group(3)
            if (ver is None or ver == entry["version"]) and (src is None or src == entry.get("source", "")):
                out.append(p["name"] + " " + p["version"] + ("" if p.get("source") else " (workspace member)"))
    return sorted(set(out)) or ["(nobody in this lock: a root or a stale entry)"]

def check_lock(text: str, label: str, cfg: dict) -> list:
    pkgs = tomllib.loads(text).get("package", [])
    errors = []
    for lib in cfg["libraries"]:
        entries = [p for p in pkgs if p["name"] == lib]
        if len(entries) > 1:
            lines = [f"ERROR {label}: `{lib}` has {len(entries)} sources in the lock, it must have exactly ONE:"]
            for e in entries:
                lines.append(f"    {lib} {e['version']} from {e.get('source') or '(path)'}\n      pulled by: " + ", ".join(pullers(pkgs, lib, e)))
            lines.append("  fix: pin every dependent to the same tag of the same repo (`cargo update -p " + lib + "` after aligning the pins).")
            errors.append("\n".join(lines))
        want = cfg.get("only_from", {}).get(lib)
        if cfg["only_from_on"] and want:
            for e in entries:
                if repo_of(e.get("source", "")) != norm(want):
                    errors.append(f"ERROR {label}: `{lib}` {e['version']} comes from {e.get('source') or '(path)'}, V1+ allows only {want}\n"
                                  f"      pulled by: " + ", ".join(pullers(pkgs, lib, e)))
    return errors

def locks_of(arg: str):
    if arg == "-":
        yield "<stdin>", sys.stdin.read(), None
        return
    p = Path(arg)
    if p.is_file():
        yield str(p), p.read_text(encoding="utf-8"), p.parent
    elif p.is_dir():
        for d, dirs, files in os.walk(p):
            dirs[:] = [x for x in dirs if x not in ("target", ".git")]
            if "Cargo.lock" in files:
                f = Path(d) / "Cargo.lock"
                yield str(f), f.read_text(encoding="utf-8"), p
    else:
        raise OSError(f"cannot read {arg}")

def main(argv: list) -> int:
    cfg_path, only_from, target, print_strict = CONFIG, False, None, False
    it = iter(argv)
    for a in it:
        if a == "--print-strict":
            print_strict = True
        elif a == "--config":
            cfg_path = Path(next(it, ""))
        elif a == "--only-from":
            only_from = True
        elif target is None:
            target = a
        else:
            print(__doc__, file=sys.stderr)
            return 2
    if target is None:
        print(__doc__, file=sys.stderr)
        return 2
    if print_strict:  # for scripts/gates-lock.sh: the switch of this repo (its .fs-lock-sources.toml, env FS_LOCK_STRICT=1)
        try:
            cfg = load_config(cfg_path, Path(target) if Path(target).is_dir() else None, False)
        except (OSError, tomllib.TOMLDecodeError) as e:
            print(f"ERROR cannot read the configuration: {e}", file=sys.stderr)
            return 2
        print("true" if cfg.get("strict") or os.environ.get("FS_LOCK_STRICT") == "1" else "false")
        return 0
    try:
        bad = []
        for label, text, root in locks_of(target):
            bad += check_lock(text, label, load_config(cfg_path, root, only_from))
    except (OSError, tomllib.TOMLDecodeError, KeyError) as e:
        print(f"ERROR cannot check {target}: {e}", file=sys.stderr)
        return 2
    for b in bad:
        print(b)
    return 1 if bad else 0

if __name__ == "__main__":
    sys.exit(main(sys.argv[1:]))
