# Pattern: none (script of the pin cascade).
"""Cargo.toml reading and editing of the pin-kaskade scripts: which dependencies come from a FreeSynergy git repo, with `tag` or `rev`.
Cargo.lock is NEVER edited here (the lift makes it, by cargo)."""
import os
import re
import tomllib

ORG = "FreeSynergy"
GIT = re.compile(r"^(?:git\+)?https?://github\.com/([^/]+)/([^/?#]+?)(?:\.git)?/?(?:[?#].*)?$")
# crate -> owner repo (the repo that carries the crate), for the wrong-source test; most repos carry the crate of their own name
OWNER_EXACT = {
    "fs-types": "fs-types", "fs-conformance": "fs-types", "fs-interfaces": "fs-interfaces", "fs-objects": "fs-objects",
    "fs-rpc": "fs-rpc", "fs-jscalendar": "fs-jscalendar", "fs-people-directory": "fs-people", "fs-admin-frontend": "fs-components",
}
OWNER_PREFIX = (("fs-components-", "fs-components"), ("fs-tasks-", "fs-tasks"), ("fs-people-", "fs-people"))
SECTIONS = ("dependencies", "dev-dependencies", "build-dependencies")


def owner_of_crate(crate):
    if crate in OWNER_EXACT:
        return OWNER_EXACT[crate]
    for pre, repo in OWNER_PREFIX:
        if crate.startswith(pre):
            return repo
    return None


def cargo_files(root):
    out = []
    for d, dirs, files in os.walk(root):
        dirs[:] = [x for x in dirs if x not in ("target", ".git")]
        if "Cargo.toml" in files:
            out.append(os.path.join(d, "Cargo.toml"))
    return sorted(out)


def _tables(doc):
    for s in SECTIONS:
        yield s, doc.get(s, {})
    yield "workspace.dependencies", doc.get("workspace", {}).get("dependencies", {})
    for tname, t in doc.get("target", {}).items():
        for s in SECTIONS:
            yield f"target.{tname}.{s}", t.get(s, {})


def git_deps(text):
    """Every dependency with a git URL, as dicts {key, crate, owner, repo, tag, rev, section}. owner is the GitHub org, repo the repo name."""
    out = []
    for section, table in _tables(tomllib.loads(text)):
        for key, spec in table.items():
            if not isinstance(spec, dict) or "git" not in spec:
                continue
            m = GIT.match(spec["git"])
            if not m:
                continue
            out.append({"key": key, "crate": spec.get("package", key), "owner": m.group(1), "repo": m.group(2),
                        "tag": spec.get("tag"), "rev": spec.get("rev"), "section": section})
    return out


def fremdquellen(path):
    """The set of known foreign sources `OWNER/NAME` of the file (lines `OWNER/NAME  # why`; `#` lines and empty lines are skipped)."""
    out = set()
    with open(path, encoding="utf-8") as f:
        for raw in f:
            line = raw.split("#", 1)[0].strip()
            if line:
                out.add(line.lower())
    return out


def is_fremd(dep, fremd):
    return f"{dep['owner']}/{dep['repo']}".lower() in fremd


def set_rev_to_tag(text, key, repo, rev, tag):
    """`key = { git = ".../repo", rev = "<rev>" ... }` becomes `tag = "<tag>"` on the dependency line(s) of that key; returns (new text, changes)."""
    n = 0
    lines = text.split("\n")
    for i, line in enumerate(lines):
        if re.match(rf"^\s*{re.escape(key)}\s*=\s*\{{", line) and f"/{repo}" in line and re.search(rf'rev\s*=\s*"{re.escape(rev)}"', line):
            lines[i] = re.sub(rf'rev\s*=\s*"{re.escape(rev)}"', f'tag = "{tag}"', line)
            n += 1
    return "\n".join(lines), n
