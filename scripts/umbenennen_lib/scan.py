"""The read-only plan of `dry-run` and `after`: every hit of the old name, sorted into categories, then split by `keep` and `allow`."""
import fnmatch
import os
import re

from .common import crate_re, file_lists, keep_for, kinds, lines_of, text_of, toml_ids

CATS = ("crates", "ci", "pins", "catalog", "catalogfiles", "docs", "source", "lock", "desktop", "outside")
DOC_EXT = (".md", ".yml", ".yaml")


def _hit(cat, repo, rel, n, line, hay, **more):
    return {"cat": cat, "repo": repo, "file": rel, "line": n, "text": line.strip()[:200], "hay": hay, **more}


def _pin(old, line):
    return f"FreeSynergy/{old}" in line or re.match(rf"\s*{re.escape(old)}(-[a-z0-9]+)*\s*=", line)


def _repo_file(a, repo, rel, text, hits):
    base = os.path.basename(rel)
    ci = rel.startswith(".github/")
    for n, line, hay in lines_of(rel, text):
        if not kinds(a.old, line):
            continue
        if base == "Cargo.toml" and not ci:
            m = re.match(r'\s*name\s*=\s*"([^"]+)"', line)
            if m and crate_re(a.old).fullmatch(m.group(1)):
                hits.append(_hit("crates", repo, rel, n, line, hay, old=m.group(1), new=a.new + m.group(1)[len(a.old):]))
                continue
            if _pin(a.old, line):
                hits.append(_hit("pins", repo, rel, n, line, hay))
                continue
        cat = "ci" if ci else "lock" if base == "Cargo.lock" else "desktop" if rel.endswith(".desktop") else "source"
        hits.append(_hit(cat, repo, rel, n, line, hay))


def _untracked(a, root, rels, warn):
    for rel in rels:
        text = text_of(os.path.join(root, rel))
        if not text or not any(kinds(a.old, ln) for ln in text.split("\n")):
            continue
        shown = [(n, ln) for n, ln, _ in lines_of(rel, text) if kinds(a.old, ln)]
        for n, ln in shown[:3]:
            warn.append({"file": rel, "repo": root, "line": n, "text": ln.strip()[:200]})


def scan(a):
    """All hits (list of dicts with `cat` and `hay`) and the untracked warnings. Nothing is written."""
    hits, untracked = [], []
    for repo in a.repos:
        tracked, other = file_lists(repo)
        for rel in tracked:
            text = text_of(os.path.join(repo, rel))
            if text is None or os.path.basename(rel) == "retired-names.txt" or not any(k in text for k in (a.old, a.old.replace("-", "_"), a.old.replace("-", "_").upper())):
                continue
            _repo_file(a, repo, rel, text, hits)
        _untracked(a, repo, other, untracked)
    if a.catalog:
        tracked, other = file_lists(a.catalog)
        for rel in tracked:
            text = text_of(os.path.join(a.catalog, rel))
            if text is None:
                continue
            for n, line, hay in lines_of(rel, text):
                if rel == "packages.toml" and re.match(rf'\s*id\s*=\s*"{re.escape(a.old)}"', line):
                    hits.append(_hit("catalog", a.catalog, rel, n, line, hay, entry=a.old,
                                     action=f'add replaces = ["{a.old}"] to the entry {a.new}'))
                elif kinds(a.old, line) and not re.match(r"\s*replaces\s*=", line):
                    hits.append(_hit("catalogfiles", a.catalog, rel, n, line, hay))
        _untracked(a, a.catalog, other, untracked)
    if a.docs:
        tracked, other = file_lists(a.docs)
        for rel in tracked:
            text = text_of(os.path.join(a.docs, rel)) if rel.endswith(DOC_EXT) else None
            for n, line, hay in lines_of(rel, text or ""):
                if kinds(a.old, line):
                    hits.append(_hit("docs", a.docs, rel, n, line, hay))
        _untracked(a, a.docs, [r for r in other if r.endswith(DOC_EXT)], untracked)
    return hits, untracked


def split(hits, keep, allow):
    """Open hits per category, the hits kept on purpose (with grund and bereich), the allowed ones, and the stale keep patterns."""
    plan = {c: [] for c in CATS}
    kept, allowed, used = [], [], set()
    for h in hits:
        hay = h.pop("hay")
        i = keep_for(keep, hay)
        if i is not None:
            used.add(i)
            k = keep[i]
            kept.append({"category": h["cat"], "file": h["file"], "line": h["line"], "text": h["text"], "repo": h["repo"],
                         "pattern": k["pattern"], "grund": k["grund"], "bereich": k["bereich"]})
        elif any(fnmatch.fnmatch(h["file"], g) for g in allow):
            allowed.append({"category": h["cat"], "file": h["file"], "line": h["line"], "repo": h["repo"]})
        else:
            plan[h.pop("cat")].append(h)
    stale = [k["pattern"] for i, k in enumerate(keep) if i not in used]
    return plan, kept, allowed, stale
