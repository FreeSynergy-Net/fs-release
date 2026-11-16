#!/usr/bin/env bash
# Pattern: Strategy-by-presence is NOT used here - this job has no secrets at all (ADR 0078: job `build` without secrets, then `sign`).
# Builds the artefacts of one site from an approved folder state, deterministically (same folder -> same bytes).
#   site-bauen.sh check <src>                       refuse a folder that breaks the rules (see below), prints every problem, exit 1
#   site-bauen.sh build <src> <outdir> <name> <version>   check, then write into <outdir> (must not exist or be empty):
#       <name>-<version>-site.tar.gz      the folder, sorted, owner 0, mtime 0, gzip without name/time
#       <name>-<version>-manifest.json    name, version, tree_sha256, file list with sha256 and size, tar sha256
#       <name>-<version>.spdx.json        SPDX 2.3 file list (the site has no packages; the project's cargo-sbom action does not apply)
#       SHA256SUMS.txt                    the three files above
#   and prints `tree-sha256=<hash>` (the hash the approval must name). A second build into a temp folder must be byte-identical.
# Rules of a folder: only regular files, safe names, an index.html at the root, no script, no external resource, no secret-looking text.
set -euo pipefail
here=$(cd "$(dirname "$0")" && pwd)
# shellcheck source=site-lib.sh
. "$here/site-lib.sh"
die() { echo "error: $*" >&2; exit 2; }

check_src() {
  local src=$1 bad=0 f
  [ -d "$src" ] || { echo "$src: not a directory" >&2; return 1; }
  [ -f "$src/index.html" ] || { echo "$src: no index.html at the root" >&2; bad=1; }
  if [ -n "$(find "$src" -mindepth 1 ! -type f ! -type d -print -quit)" ]; then echo "$src: contains something that is not a regular file or folder (symlink, device)" >&2; bad=1; fi
  while IFS= read -r f; do
    [[ $f =~ ^[A-Za-z0-9._/-]+$ ]] || { echo "$f: unsafe file name" >&2; bad=1; }
    [[ $f != *..* ]] || { echo "$f: '..' in the name" >&2; bad=1; }
  done < <(cd "$src" && find . -type f -printf '%P\n')
  while IFS= read -r f; do
    if grep -q -i -E '<script|<iframe|<object|<embed|<link[^>]*(href|src)=|src="(https?:)?//|@import|url\((https?:)?//' "$src/$f"; then echo "$f: script or external resource" >&2; bad=1; fi
    if grep -q -E 'BEGIN [A-Z ]*PRIVATE KEY|ghp_[A-Za-z0-9]{20,}|github_pat_|AKIA[0-9A-Z]{16}|(password|passwort|secret|token)[[:space:]]*[=:]' "$src/$f"; then echo "$f: looks like a secret" >&2; bad=1; fi
  done < <(cd "$src" && find . -type f \( -name '*.html' -o -name '*.css' -o -name '*.js' -o -name '*.txt' -o -name '*.xml' -o -name '*.json' -o -name '*.svg' \) -printf '%P\n')
  return "$bad"
}

build_into() { # src out name version
  local src=$1 out=$2 name=$3 version=$4 tar="$2/$3-$4-site.tar.gz"
  mkdir -p "$out"
  (cd "$src" && tar --sort=name --mtime=@0 --owner=0 --group=0 --numeric-owner --mode='u=rwX,go=rX' --format=gnu -cf - .) | gzip -n -9 > "$tar"
  python3 -I - "$src" "$out" "$name" "$version" "$(tar_tree_hash "$tar")" "$(sha256sum "$tar" | cut -d' ' -f1)" <<'PY'
import hashlib, json, os, sys
src, out, name, version, tree, tarsum = sys.argv[1:7]
files = []
for root, _, names in os.walk(src):
    for n in names:
        p = os.path.join(root, n); rel = os.path.relpath(p, src)
        data = open(p, "rb").read()
        files.append({"path": rel, "sha256": hashlib.sha256(data).hexdigest(), "size": len(data)})
files.sort(key=lambda f: f["path"].encode())
man = {"name": name, "version": version, "tree_sha256": tree, "tar": f"{name}-{version}-site.tar.gz", "tar_sha256": tarsum, "files": files}
open(f"{out}/{name}-{version}-manifest.json", "w").write(json.dumps(man, indent=2, sort_keys=True) + "\n")
spdx = {"spdxVersion": "SPDX-2.3", "dataLicense": "CC0-1.0", "SPDXID": "SPDXRef-DOCUMENT", "name": f"{name}-{version}",
        "documentNamespace": f"https://freesynergy.net/spdx/{name}/{version}/{tree}",
        "creationInfo": {"created": "1970-01-01T00:00:00Z", "creators": ["Tool: fs-release site-bauen"]},
        "files": [{"SPDXID": f"SPDXRef-File-{i}", "fileName": "./" + f["path"], "checksums": [{"algorithm": "SHA256", "checksumValue": f["sha256"]}],
                   "licenseConcluded": "NOASSERTION", "copyrightText": "NOASSERTION"} for i, f in enumerate(files)]}
open(f"{out}/{name}-{version}.spdx.json", "w").write(json.dumps(spdx, indent=2, sort_keys=True) + "\n")
PY
  (cd "$out" && sha256sum -- "$name-$version-site.tar.gz" "$name-$version-manifest.json" "$name-$version.spdx.json" > SHA256SUMS.txt)
}

cmd=${1:-}
case $cmd in
  check) check_src "${2:?usage: site-bauen.sh check <src>}" && echo "folder ok" ;;
  build)
    src=${2:?usage: site-bauen.sh build <src> <outdir> <name> <version>}; out=${3:?outdir}; name=${4:?name}; version=${5:?version}
    [[ $name =~ ^[a-z0-9][a-z0-9.-]*$ ]] || die "name: lower-case letters, digits, dots, dashes"
    [[ $version =~ ^[0-9]+(\.[0-9]+)*([-+][A-Za-z0-9.]+)?$ ]] || die "version: like 0.0.1"
    if [ -e "$out" ] && [ -n "$(ls -A "$out")" ]; then die "$out exists and is not empty"; fi
    check_src "$src" || exit 1
    build_into "$src" "$out" "$name" "$version"
    chk=$(mktemp -d); trap 'rm -rf "$chk"' EXIT
    build_into "$src" "$chk/again" "$name" "$version"
    if ! diff -r "$out" "$chk/again" >/dev/null; then echo "error: the second build differs from the first (not reproducible)" >&2; exit 1; fi
    echo "tree-sha256=$(tree_hash "$src")"
    echo "built $(find "$out" -type f | wc -l) files in $out" ;;
  *) die "usage: site-bauen.sh check <src> | build <src> <outdir> <name> <version>" ;;
esac
