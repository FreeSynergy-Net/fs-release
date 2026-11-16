#!/usr/bin/env bash
# Pattern: Strategy-by-presence is NOT used here - this job holds no key at all (ADR 0078: job `build` without keys, then `sign`).
# Builds the artefacts of one site from an approved folder state, deterministically (same folder -> same bytes).
#   site-bauen.sh [--passthrough] check <src>                            refuse a folder that breaks the rules (see below), every problem named, exit 1
#   site-bauen.sh [--passthrough] build <src> <outdir> <name> <version>  check, then write into <outdir> (must not exist or be empty):
#       <name>-<version>-site.tar.gz      the folder, sorted, owner 0, mtime SITE_MTIME, modes 0644/0755, gzip without name/time
#       <name>-<version>-manifest.json    name, version, mode, tree_sha256, tar_sha256, file list with sha256 and size, tar and gzip version
#       <name>-<version>.spdx.json        SPDX 2.3 file list (the site has no packages; the project's cargo-sbom action does not apply)
#       SHA256SUMS.txt                    the three files above
#   (the manifest also records the site: schema, id, directory, path_prefix and names of its site.toml, schema 1 or 2)
#   and prints `tree-sha256=<hash>` and `tar-sha256=<hash>` (the approval names both; the tarball hash is the bytes). A second build into a
#   temp folder must be byte-identical (the comparison is mutation-tested in test-site.sh).
# SITE_MTIME (epoch, required) is the release time: every entry of the tarball gets it as mtime, so the web server's ETag (size + mtime) changes with each
# release (a fixed 0 would give an unchanged ETag for a same-size file). CI takes the commit time of the approved source; the manifest records it.
# Rules of a folder: allowlist of file types, no dot files, safe names, an index.html at the root, no secret-looking text; mode static: also no
# script and no external resource (parsed, not grepped); mode --passthrough (a person's own HTML with JavaScript, explicit): content rules off,
# a log line per file on stdout, the web server's CSP is the second wall. Details: scripts/site-pruefen.py.
set -euo pipefail
here=$(cd "$(dirname "$0")" && pwd)
# shellcheck source=site-lib.sh
. "$here/site-lib.sh"
die() { echo "error: $*" >&2; exit 2; }
mode=static; popt=()
if [ "${1:-}" = "--passthrough" ]; then mode=passthrough; popt=(--passthrough); shift; fi

check_src() { python3 -I "$here/site-pruefen.py" "${popt[@]}" "$1"; }

build_into() { # src out name version
  local src=$1 out=$2 name=$3 version=$4 tar="$2/$3-$4-site.tar.gz"
  mkdir -p "$out"
  (cd "$src" && tar --sort=name --mtime=@"$SITE_MTIME" --owner=0 --group=0 --numeric-owner --mode='u=rwX,go=rX' --format=gnu -cf - .) | gzip -n -9 > "$tar"
  tar_check "$tar" >/dev/null || die "the tarball just built does not pass the tarball check"
  python3 -I - "$src" "$out" "$name" "$version" "$mode" "$SITE_MTIME" "$(tar_tree_hash "$tar")" "$(sha256sum "$tar" | cut -d' ' -f1)" \
    "$(tar --version | head -1)" "$(gzip --version 2>&1 | head -1)" "$here" <<'PY'
import hashlib, json, os, sys
src, out, name, version, mode, mtime, tree, tarsum, tarv, gzv, here = sys.argv[1:12]
sys.path.insert(0, here)
import site_manifest
sm = site_manifest.load(os.path.join(src, "site.toml"))  # the folder check has passed: the file is valid (schema 1 reads as id main, prefix /)
files = []
for root, _, names in os.walk(src):
    for n in names:
        p = os.path.join(root, n); rel = os.path.relpath(p, src)
        data = open(p, "rb").read()
        files.append({"path": rel, "sha256": hashlib.sha256(data).hexdigest(), "size": len(data)})
files.sort(key=lambda f: f["path"].encode())
man = {"name": name, "version": version, "mode": mode, "mtime": int(mtime), "tree_sha256": tree, "tar": f"{name}-{version}-site.tar.gz", "tar_sha256": tarsum,
       "tools": {"tar": tarv, "gzip": gzv}, "files": files,
       "site": {"schema": sm["schema"], "id": sm["id"], "dir": sm["dir"], "path_prefix": sm["path_prefix"], "names": sm["names"]}}
open(f"{out}/{name}-{version}-manifest.json", "w").write(json.dumps(man, indent=2, sort_keys=True) + "\n")
spdx = {"spdxVersion": "SPDX-2.3", "dataLicense": "CC0-1.0", "SPDXID": "SPDXRef-DOCUMENT", "name": f"{name}-{version}",
        "documentNamespace": f"urn:fs-release:site-spdx:{name}:{version}:{tree}",
        "creationInfo": {"created": "1970-01-01T00:00:00Z", "creators": ["Tool: fs-release site-bauen"]},
        "files": [{"SPDXID": f"SPDXRef-File-{i}", "fileName": "./" + f["path"], "checksums": [{"algorithm": "SHA256", "checksumValue": f["sha256"]}],
                   "licenseConcluded": "NOASSERTION", "copyrightText": "NOASSERTION"} for i, f in enumerate(files)]}
open(f"{out}/{name}-{version}.spdx.json", "w").write(json.dumps(spdx, indent=2, sort_keys=True) + "\n")
PY
  (cd "$out" && sha256sum -- "$name-$version-site.tar.gz" "$name-$version-manifest.json" "$name-$version.spdx.json" > SHA256SUMS.txt)
}

cmd=${1:-}
case $cmd in
  check) check_src "${2:?usage: site-bauen.sh [--passthrough] check <src>}" && echo "folder ok ($mode)" ;;
  build)
    src=${2:?usage: site-bauen.sh [--passthrough] build <src> <outdir> <name> <version>}; out=${3:?outdir}; name=${4:?name}; version=${5:?version}
    [[ $name =~ ^[a-z0-9][a-z0-9.-]*$ ]] || die "name: lower-case letters, digits, dots, dashes"
    [[ $version =~ ^[0-9]+(\.[0-9]+)*([-+][A-Za-z0-9.]+)?$ ]] || die "version: like 0.0.1"
    if [ -e "$out" ] && [ -n "$(ls -A "$out")" ]; then die "$out exists and is not empty"; fi
    [[ ${SITE_MTIME:-} =~ ^[0-9]{10}$ ]] || die "SITE_MTIME (the release time as epoch seconds, e.g. the commit time of the approved source) is not set or not an epoch"
    check_src "$src" || exit 1
    build_into "$src" "$out" "$name" "$version"
    chk=$(mktemp -d); trap 'rm -rf "$chk"' EXIT
    build_into "$src" "$chk/again" "$name" "$version"
    if ! diff -r "$out" "$chk/again" >/dev/null; then echo "error: the second build differs from the first (not reproducible)" >&2; exit 1; fi
    echo "tree-sha256=$(tree_hash "$src")"
    echo "tar-sha256=$(sha256sum "$out/$name-$version-site.tar.gz" | cut -d' ' -f1)"
    echo "mode=$mode"
    echo "built $(find "$out" -type f | wc -l) files in $out" ;;
  *) die "usage: site-bauen.sh [--passthrough] check <src> | build <src> <outdir> <name> <version>" ;;
esac
