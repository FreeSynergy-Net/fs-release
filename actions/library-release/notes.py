#!/usr/bin/env python3
"""Library mode of the release: check tag = workspace version = non-empty CHANGELOG section and write the notes of that section.

usage: python3 -I notes.py <working directory> <tag> [<notes file>]     (tag from $TAG / $GITHUB_REF_NAME when not given: use `-`)

Checks (every failure is one `::error::` line and exit 1; nothing is written then):
  1. the tag is `vMAJOR.MINOR.PATCH[-pre][+build]` (SemVer, as in the plan of binary releases);
  2. `Cargo.toml` of the working directory has `[workspace.package] version` (or `[package] version`) and it equals the tag version;
  3. CHANGELOG.md has the section `## X.Y.Z` or `## <name> X.Y.Z` and it is not empty (not only blank lines).
The notes are the text of that section (without the heading). Pure library: no binaries, no packages, no signatures, no secrets.
"""
import os
import pathlib
import re
import sys
import tomllib

SEMVER = re.compile(
    r"^(0|[1-9]\d*)\.(0|[1-9]\d*)\.(0|[1-9]\d*)"
    r"(?:-((?:0|[1-9]\d*|\d*[a-zA-Z-][0-9a-zA-Z-]*)(?:\.(?:0|[1-9]\d*|\d*[a-zA-Z-][0-9a-zA-Z-]*))*))?"
    r"(?:\+([0-9a-zA-Z-]+(?:\.[0-9a-zA-Z-]+)*))?$"
)


def fail(msg):
    print(f"::error::{msg}")
    sys.exit(1)


def workspace_version(workdir):
    path = pathlib.Path(workdir) / "Cargo.toml"
    try:
        data = tomllib.loads(path.read_text(encoding="utf-8"))
    except (OSError, tomllib.TOMLDecodeError) as e:
        fail(f"{path}: cannot be read ({e})")
    v = data.get("workspace", {}).get("package", {}).get("version") or data.get("package", {}).get("version")
    if not isinstance(v, str) or not v:
        fail(f"{path}: no [workspace.package] version (or [package] version)")
    return v


def section(changelog, version):
    """The text of `## <version>` or `## <name> <version>` up to the next `## ` heading."""
    head = re.compile(rf"^## (?:\S+ )?{re.escape(version)}$")
    out, on = [], False
    for line in changelog.splitlines():
        if on and line.startswith("## "):
            break
        if on:
            out.append(line)
        elif head.match(line):
            on = True
    return "\n".join(out).strip("\n") if on else None


def main(argv):
    if len(argv) not in (2, 3):
        print("usage: notes.py <working directory> <tag|-> [<notes file>]", file=sys.stderr)
        return 2
    workdir, tag = argv[0], argv[1]
    if tag == "-":
        tag = os.environ.get("TAG") or os.environ.get("GITHUB_REF_NAME", "")
    if not tag.startswith("v") or not SEMVER.match(tag[1:]):
        fail(f"tag '{tag}' is not a SemVer release tag (expected vX.Y.Z[-pre][+build])")
    version = tag[1:]
    have = workspace_version(workdir)
    if have != version:
        fail(f"tag version {version} differs from the workspace version {have} in Cargo.toml")
    path = pathlib.Path(workdir) / "CHANGELOG.md"
    try:
        text = path.read_text(encoding="utf-8")
    except OSError as e:
        fail(f"{path}: cannot be read ({e})")
    notes = section(text, version)
    if notes is None or not notes.strip():
        fail(f"CHANGELOG.md has no non-empty section '## {version}' (or '## <name> {version}')")
    if len(argv) == 3:
        pathlib.Path(argv[2]).write_text(notes + "\n", encoding="utf-8")
    print(f"library release {tag}: version and CHANGELOG section agree ({len(notes.splitlines())} lines of notes)")
    return 0


if __name__ == "__main__":
    sys.exit(main(sys.argv[1:]))
