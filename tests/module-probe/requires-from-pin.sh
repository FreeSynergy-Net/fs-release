#!/usr/bin/env bash
# Pattern: none (test caller). Release-path dry run: fs-blueprint is built from EXACTLY the commit that release.yml pins for `_release-linux.yml` (the tag run builds it from job.workflow_sha =
# that commit), packages a module whose commands.json requires `person`, and the RPM is read back in a CONTAINER (never `rpm` on the host: SELinux lesson). Red unless `Requires: fs-module-person`.
# Usage: tests/module-probe/requires-from-pin.sh [sha]   (default: the pin in .github/workflows/release.yml). Needs git, cargo, podman or docker.
set -euo pipefail
root=$(cd "$(dirname "$0")/../.." && pwd); here=$root/tests/module-probe
sha=${1:-$(sed -n 's#.*/_release-linux\.yml@\([0-9a-f]\{40\}\).*#\1#p' "$root/.github/workflows/release.yml" | head -1)}
[[ $sha =~ ^[0-9a-f]{40}$ ]] || { echo "requires-from-pin: no pin of _release-linux.yml found" >&2; exit 2; }
ct=$(command -v podman || command -v docker) || { echo "requires-from-pin: neither podman nor docker" >&2; exit 2; }
w=$(mktemp -d "${REQUIRES_PIN_TMP:-${TMPDIR:-/tmp}}/requires-pin.XXXXXX"); trap 'rm -rf "$w"' EXIT
mkdir "$w/src" "$w/mod" "$w/out"; git -C "$root" archive "$sha" | tar -x -C "$w/src"
cp -r "$here/catalog" "$here/metainfo.xml" "$here"/LICENSE-* "$w/mod/"; printf '\177ELF' >"$w/mod/service"
printf '{"module":"planning","requires":[{"module":"person","required":true}]}' >"$w/mod/catalog/commands.json"
cargo build --locked --manifest-path "$w/src/Cargo.toml" -p fs-blueprint --bin fs-blueprint
"${CARGO_TARGET_DIR:-$w/src/target}/debug/fs-blueprint" package module "$w/mod" --name planning --pkg-version 0.0.1 --binary "$w/mod/service" --arch x86_64 --out "$w/out"
req=$("$ct" run --rm --network none -v "$w/out:/out:ro,Z" "${PROBE_IMAGE:-registry.fedoraproject.org/fedora:45}" sh -c 'rpm -qp --requires /out/fs-module-planning-*.rpm')
echo "$req"; grep -qx 'fs-module-person' <<<"$req" || { echo "requires-from-pin: RED, no Requires: fs-module-person in the RPM built from $sha" >&2; exit 1; }
echo "requires-from-pin: ok ($sha)"
