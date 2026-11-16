"""The rules of `dry-run`, `after` and `retire`: names, clashes, the catalog entry, the retired list."""
import os
import re

from .common import NAME, read_retired, text_of, toml_ids
from .outside import world_dir


def name_problems(a, check_retired=True):
    problems = []
    for label, n in (("old", a.old), ("new", a.new)):
        if not NAME.fullmatch(n):
            problems.append(f"red: the {label} name {n!r} is not of the form fs-<x>")
    if a.old == a.new:
        problems.append("red: old and new name are equal")
    retired = read_retired(a.retired) if check_retired else []
    for r_old, r_new in retired:
        if a.new == r_old:
            problems.append(f"red: the name {a.new!r} was given up ({r_old} -> {r_new}) and is never given out again (ADR 0080)")
        if a.old == r_old:
            problems.append(f"red: {a.old!r} is already retired ({r_old} -> {r_new})")
    return problems


def clashes(a):
    problems = []
    dirs = [os.path.normpath(r) for r in a.repos]
    world = world_dir(a)
    if world and os.path.isdir(os.path.join(world, a.new)):
        dirs.append(os.path.join(world, a.new))
    for d in dirs:
        if os.path.basename(d) == a.new:
            problems.append(f"red: a checkout named {a.new!r} already exists ({d})")
            break
    if a.catalog and a.new in toml_ids(text_of(os.path.join(a.catalog, "packages.toml"))):
        problems.append(f"red: the catalog already has an entry {a.new!r}")
    return problems


def catalog_replaces_ok(a):
    path = os.path.join(a.catalog, "packages.toml")
    text = text_of(path)
    if text is None:
        return [f"red: {path} is not readable"]
    for block in re.split(r"^\s*\[\[?[^\]]+\]\]?\s*$", text, flags=re.M):
        if re.search(rf'^\s*id\s*=\s*"{re.escape(a.new)}"', block, re.M):
            if re.search(rf'^\s*replaces\s*=\s*\[[^\]]*"{re.escape(a.old)}"', block, re.M):
                return []
            return [f'red: the catalog entry {a.new!r} has no replaces = ["{a.old}"]']
    return [f"red: the catalog has no entry {a.new!r}"]


def pair_is_clean(a, retired):
    """The pair old -> new is on the list, it is the ONLY entry for the old name, and the new name was not given up since (ADR 0080)."""
    if (a.old, a.new) not in retired:
        return False
    if sum(1 for r_old, _ in retired if r_old == a.old) != 1:
        return False
    return not any(r_old == a.new for r_old, _ in retired)


LABEL = {"crates": "crate {old!r} still named in", "ci": "the old name is still in", "pins": "pin to the old name in",
         "docs": "old name still in the docs", "source": "old name still in the source", "lock": "old name still in the lock",
         "desktop": "old name still in the desktop file", "catalogfiles": "old name still in the catalog file",
         "outside": "old name still in the repo (origin/main)"}


def keep_problems(stale):
    return [f"red: keep pattern {p!r} matches no hit of the old name (a stale entry)" for p in stale]


def after_problems(a, plan, stale):
    problems = []
    for cat, label in LABEL.items():
        for h in plan[cat]:
            where = f"{h['repo']}/{h['file']}:{h['line']}" if cat in ("pins", "docs", "source", "lock", "desktop", "catalogfiles", "outside") \
                else f"{h['repo']}/{h['file']}"
            if cat == "docs":
                where = f"{h['file']}:{h['line']}"
            problems.append(f"red: {label.format(old=h.get('old'))} {where}")
    problems += keep_problems(stale)
    if a.catalog:
        problems += catalog_replaces_ok(a)
    retired = read_retired(a.retired)
    if (a.old, a.new) not in retired:
        problems.append(f"red: {a.old} -> {a.new} is not on the retired list (the old name must never be given out again)")
    elif not pair_is_clean(a, retired):
        problems.append(f"red: the retired list is inconsistent for {a.old} -> {a.new} (another entry for the old name, or the new name was given up since)")
    return problems


def retire(a):
    # The same pair again is a no-op (rc 0): the flow loops back to `umbau` after a red gate or a NO and runs `sperre` again.
    # Any other entry for the old name (old -> something else) or for the new name as an old one stays red - also for the same pair when the
    # new name was given up in the meantime (name recycling, ADR 0080) or the list has a second entry for the old name.
    if a.retired and pair_is_clean(a, read_retired(a.retired)) and not name_problems(a, check_retired=False):
        print(f"already retired, same pair: {a.old} -> {a.new}")
        return 0
    problems = name_problems(a)
    if problems or not a.retired:
        print("\n".join(problems) or "bad input: --retired is required")
        return 1 if problems else 2
    with open(a.retired, "a", encoding="utf-8") as f:
        f.write(f"{a.old} -> {a.new}  # renamed, the old name is never given out again (ADR 0080)\n")
    print(f"retired: {a.old} -> {a.new}")
    return 0
