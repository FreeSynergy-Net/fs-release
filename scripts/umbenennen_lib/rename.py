"""L1 write mode (scaffold): `git mv` of the crate folders, word-boundary replacement in *.toml, *.rs, *.md, Cargo.lock and the CI files.

Only in a worktree branch (never main/master), idempotent (names already renamed are a no-op), `keep` lines are skipped, no network.
Refuses on a dirty tree (it must be committed or clean first, so the rename is one reviewable diff). `--allow` (globs on the file) skips
those files. Pin URLs (`github.com/<owner>/<old>`) are NOT touched by the normal run: they stay until the GitHub rename has happened;
the separate switch `--pins` (the flow calls it in the step after `github-umbenennen`) replaces only these URLs.
The environment variable form (FS_X_ROOT) is NOT replaced (a decision of its own, see `keep`). The lock is renamed with the crates;
the check `cargo metadata --locked` is the next step of the flow (`lock`), not done here.
"""
import fnmatch
import os
import re

from .common import crate_re, file_lists, git, keep_for, lines_of, snake, text_of, words

EXT = (".toml", ".rs", ".md")


def _branch(repo):
    r = git(repo, "rev-parse", "--abbrev-ref", "HEAD")
    return r.stdout.decode().strip() if r is not None and r.returncode == 0 else ""


def _move_crates(a, repo, out):
    crates = os.path.join(repo, "crates")
    if not os.path.isdir(crates):
        return 0
    bad = 0
    for entry in sorted(os.listdir(crates)):
        if not (os.path.isdir(os.path.join(crates, entry)) and crate_re(a.old).fullmatch(entry)):
            continue
        target = a.new + entry[len(a.old):]
        if os.path.exists(os.path.join(crates, target)):
            out.append(f"red: crates/{target} already exists, crates/{entry} stays")
            bad += 1
            continue
        r = git(repo, "mv", f"crates/{entry}", f"crates/{target}")
        if r is None or r.returncode != 0:
            out.append(f"red: git mv crates/{entry} failed")
            bad += 1
        else:
            out.append(f"moved: crates/{entry} -> crates/{target}")
    return bad


def _pin(a):
    # the two known owners, exactly; the old one stays until part 2 of the address wave is done
    return re.compile(r"github\.com/(?:FreeSynergy-Net|FreeSynergy)/" + re.escape(a.old) + r"(?![\w-])")


def _dirty(repo):
    r = git(repo, "status", "--porcelain")
    return r is None or r.returncode != 0 or bool(r.stdout.strip())


def _transform(a, pin, name, sn, new_snk, line):
    """The normal run: names renamed, pin URLs kept (they follow the GitHub rename)."""
    urls = pin.findall(line)
    work = pin.sub("\0", line)
    work = sn.sub(new_snk, name.sub(a.new, work))
    for u in urls:
        work = work.replace("\0", u, 1)
    return work


def _rewrite(a, repo, keep, out):
    name, snk = words(a.old), snake(a.old)
    new_snk = snake(a.new)
    pin = _pin(a)
    pins_only = getattr(a, "pins", False)
    sn = re.compile(r"(?<![A-Za-z0-9_])" + re.escape(snk) + r"(?![a-z0-9])")
    tracked, _ = file_lists(repo)
    for rel in tracked:
        base = os.path.basename(rel)
        if not (rel.endswith(EXT) or base == "Cargo.lock" or (rel.startswith(".github/") and rel.endswith((".yml", ".yaml")))):
            continue
        if any(fnmatch.fnmatch(rel, g) for g in (a.allow or [])):
            continue
        text = text_of(os.path.join(repo, rel))
        if text is None:
            continue
        lines, changed = text.split("\n"), 0
        for n, line, hay in lines_of(rel, text):
            if keep_for(keep, hay) is not None:
                continue
            if pins_only:
                new_line = pin.sub(lambda m: m.group(0).rsplit("/", 1)[0] + "/" + a.new, line)
            else:
                new_line = _transform(a, pin, name, sn, new_snk, line)
            if new_line != line:
                lines[n - 1] = new_line
                changed += 1
        if changed:
            with open(os.path.join(repo, rel), "w", encoding="utf-8") as f:
                f.write("\n".join(lines))
            out.append(f"changed: {rel} ({changed} lines)")


def rename(a, keep):
    out, bad = [], 0
    for repo in a.repos:
        branch = _branch(repo)
        if branch in ("", "HEAD", "main", "master"):
            out.append(f"red: {repo} is on {branch or 'no branch'!r}; the rename runs only in a worktree branch")
            bad += 1
            continue
        if _dirty(repo):
            out.append(f"red: {repo} has uncommitted changes; commit or clean the tree first")
            bad += 1
            continue
        if not getattr(a, "pins", False):
            bad += _move_crates(a, repo, out)
        _rewrite(a, repo, keep, out)
    print("\n".join(out) or "nothing to do")
    print("red" if bad else "green")
    return 1 if bad else 0
