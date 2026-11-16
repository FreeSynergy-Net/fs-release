#!/usr/bin/env python3
"""Pattern: none. Allowlist check of a site tarball BEFORE anything is unpacked (ADR 0078: the build job is the less trusted part).

usage: python3 -I site-tar-pruefen.py <tarball.tar.gz>     exit 0 only for a tarball that holds nothing but plain files and folders; prints `mtime=<epoch>`.

Allowed: regular files (mode 0644 or 0755) and folders (mode 0755), owner 0:0 without names, ONE common mtime (the release time, 2020-01-01 .. now+1 day), relative safe names
without `..`, without a dot file, no name twice. Everything else is red and named: symlinks, hard links, devices, fifos, sparse files, set-uid/
set-gid/sticky bits, a folder 0777, absolute or `..` paths, other owners. This is exactly what scripts/site-bauen.sh writes.
"""
import re
import sys
import tarfile
import time

MAX_MEMBERS, MAX_BYTES = 20000, 500 * 1024 * 1024
COMP = re.compile(r"^[A-Za-z0-9_-][A-Za-z0-9._-]*$")  # no leading dot: no dot files, no "." or ".."


def main(path: str) -> int:
    bad: list[str] = []
    seen: set[str] = set()
    total = 0
    mtimes: set[int] = set()
    try:
        tf = tarfile.open(path, "r:gz")
    except (OSError, tarfile.TarError) as e:
        print(f"{path}: not a readable .tar.gz ({e})", file=sys.stderr)
        return 1
    with tf:
        try:
            for i, m in enumerate(tf):
                if i >= MAX_MEMBERS:
                    bad.append("too many entries")
                    break
                name = m.name
                label = repr(name)
                if m.type not in (tarfile.REGTYPE, tarfile.AREGTYPE, tarfile.DIRTYPE):
                    bad.append(f"{label}: not a plain file or folder (type {m.type!r}: link, device, fifo or sparse)")
                    continue
                rel = name[2:] if name.startswith("./") else name
                if name == ".":
                    rel = ""
                if rel.endswith("/"):
                    rel = rel[:-1]
                if name.startswith("/"):
                    bad.append(f"{label}: absolute path")
                comps = rel.split("/") if rel else []
                if any(not COMP.fullmatch(c) for c in comps):
                    bad.append(f"{label}: unsafe path (empty, '..', dot file or odd characters)")
                if rel in seen:
                    bad.append(f"{label}: name twice")
                seen.add(rel)
                perm = m.mode & 0o7777
                want = (0o755,) if m.isdir() else (0o644, 0o755)
                if perm not in want:
                    bad.append(f"{label}: mode {perm:04o} (allowed: {', '.join(f'{w:04o}' for w in want)})")
                if m.uid != 0 or m.gid != 0 or m.uname or m.gname:
                    bad.append(f"{label}: owner {m.uid}:{m.gid} {m.uname!r}/{m.gname!r} (must be 0:0 without names)")
                mtimes.add(int(m.mtime))
                total += m.size
                if total > MAX_BYTES:
                    bad.append("content too large")
                    break
        except (OSError, tarfile.TarError, EOFError) as e:
            bad.append(f"tarball broken ({e})")
    if len(mtimes) != 1:
        bad.append(f"entries have {len(mtimes)} different mtimes (must be exactly one: the release time)")
    elif not 1577836800 <= next(iter(mtimes)) <= time.time() + 86400:
        bad.append(f"mtime {next(iter(mtimes))} is not a release time (2020-01-01 .. now+1 day)")
    for b in bad:
        print(f"{path}: {b}", file=sys.stderr)
    if not bad:
        print(f"mtime={next(iter(mtimes))}")
    return 1 if bad else 0


if __name__ == "__main__":
    if len(sys.argv) != 2:
        print("usage: site-tar-pruefen.py <tarball.tar.gz>", file=sys.stderr)
        sys.exit(2)
    sys.exit(main(sys.argv[1]))
