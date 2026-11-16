"""L3: every fs-* checkout beside the given repos is searched on `origin/main` with `git grep` (read only, no checkout, no network)."""
import glob
import os

from .common import git, is_git, kinds


def _common_dir(path):
    r = git(path, "rev-parse", "--path-format=absolute", "--git-common-dir")
    if r is not None and r.returncode == 0:
        return os.path.realpath(r.stdout.decode().strip())
    return os.path.realpath(os.path.join(path, ".git"))


def world_dir(a):
    return a.world or (os.path.dirname(os.path.abspath(a.repos[0])) if a.repos else None)


def outside_hits(a):
    """(hits, warnings): hits on origin/main of the repos that are NOT among --repos/--catalog/--docs (same git folder = same repo)."""
    world = world_dir(a)
    hits, warns = [], []
    if not world:
        return hits, warns
    given = {_common_dir(d) for d in [*a.repos, a.catalog, a.docs] if d and is_git(d)}
    s = a.old.replace("-", "_")
    for d in sorted(glob.glob(os.path.join(world, "fs-*"))):
        if not os.path.isdir(d) or not is_git(d) or _common_dir(d) in given:
            continue
        name = os.path.basename(d)
        ref = git(d, "rev-parse", "--verify", "-q", "origin/main^{commit}")
        if ref is None or ref.returncode != 0:
            warns.append(f"warn: {name} has no origin/main, not searched")
            continue
        r = git(d, "grep", "-n", "-I", "-F", "--no-color", "-e", a.old, "-e", s, "-e", s.upper(), "origin/main", "--")
        if r is None or r.returncode not in (0, 1):
            warns.append(f"warn: git grep failed in {name}")
            continue
        for row in r.stdout.decode("utf-8", "replace").split("\n"):
            parts = row.split(":", 3)
            if len(parts) < 4 or not kinds(a.old, parts[3]):
                continue
            rel, n, line = parts[1], int(parts[2]) if parts[2].isdigit() else 0, parts[3]
            hits.append({"cat": "outside", "repo": d, "file": rel, "line": n, "text": line.strip()[:200],
                         "hay": f"{name} {rel} {line.strip()}"})
    return hits, warns
