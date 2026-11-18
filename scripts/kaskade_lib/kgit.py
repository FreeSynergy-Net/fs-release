# Pattern: none (script of the pin cascade).
"""Read-only git helpers of the pin-kaskade scripts. The remote base is `FS_KASKADE_BASE` (default https://github.com/FreeSynergy),
so a test can point it at local bare repos. Nothing here pushes, tags or deletes; every call is `ls-remote`, `clone` into an OWN folder or `show`."""
import os
import re
import subprocess

BASE_DEFAULT = "https://github.com/FreeSynergy"
ORG = "FreeSynergy"
TAG = re.compile(r"^v[0-9]+\.[0-9]+\.[0-9]+$")
REPO = re.compile(r"^[A-Za-z0-9][A-Za-z0-9_.-]*$")


def base():
    return os.environ.get("FS_KASKADE_BASE", BASE_DEFAULT).rstrip("/")


def url(repo):
    if not REPO.match(repo) or repo in (".", ".."):
        raise ValueError(f"bad repo name: {repo!r}")
    return f"{base()}/{repo}"


def run(args, cwd=None):
    """(rc, stdout) of a git call; stderr is kept in stdout's tail for messages. Never raises on rc != 0."""
    p = subprocess.run(["git", *args], cwd=cwd, capture_output=True, text=True, check=False)
    return p.returncode, p.stdout + ("" if p.returncode == 0 else p.stderr)


def ls_remote(repo, *patterns):
    """{ref: sha} of `git ls-remote <repo> <patterns>`; raises RuntimeError when the remote cannot be read (never an empty guess)."""
    rc, out = run(["ls-remote", url(repo), *patterns])
    if rc != 0:
        raise RuntimeError(f"ls-remote {repo} failed (rc {rc}): {out.strip()}")
    refs = {}
    for line in out.splitlines():
        sha, _, ref = line.partition("\t")
        refs[ref] = sha
    return refs


def tags(repo):
    """{tag: commit sha} (annotated tags peeled) of the remote."""
    out = {}
    refs = ls_remote(repo, "refs/tags/*")
    for ref, sha in refs.items():
        name = ref[len("refs/tags/"):]
        if not name.endswith("^{}"):
            out.setdefault(name, sha)
    for ref, sha in refs.items():
        if ref.endswith("^{}"):
            out[ref[len("refs/tags/"):-3]] = sha
    return out


def main_head(repo):
    refs = ls_remote(repo, "refs/heads/main")
    return refs.get("refs/heads/main")


def clone(repo, dest, ref=None, bare=False):
    """A blobless clone of the remote into `dest` (an own folder). depth 1, or the full history when `bare` (for ancestor checks)."""
    args = ["clone", "-q", "--filter=blob:none"]
    if bare:
        args.append("--bare")
    else:
        args += ["--depth", "1", "--no-checkout"]
    if ref:
        args += ["--branch", ref]
    rc, out = run([*args, url(repo), str(dest)])
    if rc != 0:
        raise RuntimeError(f"clone {repo} {ref or ''} failed (rc {rc}): {out.strip()}")


def show(clone_dir, rev, path):
    rc, out = run(["show", f"{rev}:{path}"], cwd=clone_dir)
    return out if rc == 0 else None


def tree(clone_dir, rev="HEAD"):
    rc, out = run(["ls-tree", "-r", "--name-only", rev], cwd=clone_dir)
    return out.splitlines() if rc == 0 else []


def version_key(tag):
    m = re.match(r"^v([0-9]+)\.([0-9]+)\.([0-9]+)$", tag or "")
    return tuple(int(x) for x in m.groups()) if m else None
