#!/usr/bin/env python3
"""Which workspace crates must be TESTED for the files a branch changed (CI cost on `unreviewed/*` branches; fmt, clippy, lock and rules always stay full).

    python3 -I scripts/affected_crates.py <cargo workspace dir> <base rev>       files = `git diff --name-only --no-renames <base>...HEAD`
    python3 -I scripts/affected_crates.py <cargo workspace dir> --files <file>   files = the lines of <file> (paths relative to the git top level; for tests)

    ... --empty-none     (last argument) no changed file at all prints `NONE <reason>` instead of `FULL` (the landing-head runs of scripts/lib_gates_list.sh skip then)

Prints ONE line on stdout: `PACKAGES a b c` (the crates that own a changed file plus every workspace crate that depends on one of them, dev-dependencies
included, transitively), or `FULL <reason>` (test the whole workspace). Exit 0 for both. Any error (no such rev, cargo metadata fails, odd name) is
`FULL`-worthy: it exits 2 and the caller (scripts/gates.sh) then tests the whole workspace. The rule is conservative: a changed file that no crate owns
(Cargo.lock, the workspace manifest, toolchain/cargo config, scripts, workflows of a virtual workspace, anything outside the workspace dir) means FULL.
"""
import json
import os
import re
import subprocess
import sys

GLOBAL = {"Cargo.lock", "Cargo.toml", "rust-toolchain", "rust-toolchain.toml", "RUST_VERSION", "clippy.toml", "rustfmt.toml", ".rustfmt.toml"}
NAME = re.compile(r"^[A-Za-z0-9_-]+$")


def run(cmd, cwd):
    return subprocess.run(cmd, cwd=cwd, check=True, capture_output=True, text=True).stdout


def changed_files(root, arg):
    """Paths relative to the workspace dir; None = a changed file outside it (FULL)."""
    if arg[0] == "--files":
        prefix, lines = "", open(arg[1], encoding="utf-8").read().splitlines()
    else:
        prefix = run(["git", "rev-parse", "--show-prefix"], root).strip()
        lines = run(["git", "diff", "--name-only", "--no-renames", f"{arg[0]}...HEAD"], root).splitlines()
    files = []
    for p in (l for l in lines if l):
        if prefix and not p.startswith(prefix):
            return None
        files.append(p[len(prefix):])
    return files


def main():
    if len(sys.argv) < 3:
        print(__doc__)
        return 2
    args = sys.argv[2:]
    empty_none = args[-1] == "--empty-none"
    if empty_none:
        args = args[:-1]
    root = os.path.realpath(sys.argv[1])
    files = changed_files(root, args)
    if files is None:
        print("FULL a changed file lies outside the workspace directory")
        return 0
    if not files and empty_none:
        print("NONE no changed file against the base")
        return 0
    if not files:
        print("FULL no changed file against the base (nothing to narrow)")
        return 0
    meta = json.loads(run(["cargo", "metadata", "--format-version", "1", "--locked"], root))
    ws = set(meta["workspace_members"])
    pkgs = {p["id"]: p for p in meta["packages"] if p["id"] in ws}
    wroot = os.path.realpath(meta["workspace_root"])
    dirs = {i: os.path.relpath(os.path.dirname(os.path.realpath(p["manifest_path"])), wroot) for i, p in pkgs.items()}
    dirs = {i: ("" if d == "." else d + "/") for i, d in dirs.items()}
    hit = set()
    for f in files:
        if f in GLOBAL or f.startswith(".cargo/"):
            print(f"FULL {f} affects every crate")
            return 0
        owners = [(len(d), i) for i, d in dirs.items() if f.startswith(d)]
        if not owners:
            print(f"FULL {f} belongs to no crate")
            return 0
        hit.add(max(owners)[1])
    rev = {i: set() for i in pkgs}
    for node in meta["resolve"]["nodes"]:
        if node["id"] in pkgs:
            for d in node["deps"]:
                if d["pkg"] in pkgs:
                    rev[d["pkg"]].add(node["id"])
    todo, seen = list(hit), set(hit)
    while todo:
        for user in rev[todo.pop()]:
            if user not in seen:
                seen.add(user)
                todo.append(user)
    names = sorted(pkgs[i]["name"] for i in seen)
    if not all(NAME.match(n) for n in names):
        print("odd crate name", file=sys.stderr)
        return 2
    if len(seen) == len(pkgs):
        print("FULL every crate is affected")
    else:
        print("PACKAGES " + " ".join(names))
    return 0


if __name__ == "__main__":
    try:
        sys.exit(main())
    except (subprocess.CalledProcessError, OSError, KeyError, ValueError) as e:
        print(f"affected_crates: {e}", file=sys.stderr)
        sys.exit(2)
