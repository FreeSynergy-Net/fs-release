#!/usr/bin/env bash
# Pattern: Strategy-by-presence is turned around on purpose: the existing actions/sign-files/sign.sh says "not signed" and exits 0 when the key is
# missing. Here a missing key, a missing approval or a missing signature is RED. This is the `sign` job of ADR 0078 (the only job with the key).
#   GPG_PRIVATE_KEY=... [GPG_PASSPHRASE=...] site-signieren.sh <dist-dir> <approval> <approval.asc> <ro-keyring> <allowed-fingerprints> <run-id> \
#       <release-pubkey.asc> <approved-src>
# <approved-src> is the git checkout of the approved folder state. Order (the key is only loaded in step 5, by sign.sh):
#   1. key present   2. approval check (site-freigabe-pruefen.sh, WITHOUT the key in its environment: it unpacks an untrusted tarball)
#   3. REBUILD from <approved-src> (also without the key) and compare byte for byte with <dist-dir>: the delivered tarball is only what the
#      approved folder produces   4. artefacts equal their SHA256SUMS.txt; the approval and its signature join the release as proof
#   5. detached .asc for every file (sign.sh detached)   6. site-signaturen-pruefen.sh: every file has a verifying .asc made by exactly the release key.
set -euo pipefail
here=$(cd "$(dirname "$0")" && pwd)
red() { echo "RED: $*" >&2; exit 1; }
[ "$#" -eq 8 ] || { echo "usage: site-signieren.sh <dist-dir> <approval> <approval.asc> <ro-keyring> <allowed-fingerprints> <run-id> <release-pubkey.asc> <approved-src>" >&2; exit 2; }
dist=$1 appr=$2 sig=$3 ring=$4 allowed=$5 run=$6 pub=$7 src=$8
[ -n "${GPG_PRIVATE_KEY:-}" ] || red "GPG_PRIVATE_KEY is not set: nothing is signed, and that is not green"
[ -s "$pub" ] || red "no public release key $pub"
[ -d "$src" ] || red "no approved source folder $src"
nokey=(env -u GPG_PRIVATE_KEY -u GPG_PASSPHRASE)
"${nokey[@]}" "$here/site-freigabe-pruefen.sh" "$dist" "$appr" "$sig" "$ring" "$allowed" "$run" || red "approval check failed"
name=$(sed -n -E 's/^name: (.*)$/\1/p' "$appr"); version=$(sed -n -E 's/^version: (.*)$/\1/p' "$appr"); mode=$(sed -n -E 's/^mode: (.*)$/\1/p' "$appr")
popt=(); [ "$mode" != passthrough ] || popt=(--passthrough)
hold=$(mktemp -d); trap 'rm -rf "$hold"' EXIT
"${nokey[@]}" "$here/site-bauen.sh" "${popt[@]}" build "$src" "$hold/again" "$name" "$version" >/dev/null || red "rebuild from the approved source failed"
diff -r "$dist" "$hold/again" >/dev/null || red "the delivered files differ from a rebuild of the approved source (not the approved state)"
(cd "$dist" && sha256sum --check --strict --quiet SHA256SUMS.txt) || red "artefacts differ from SHA256SUMS.txt"
cp -- "$appr" "$dist/approval.txt"; cp -- "$sig" "$dist/approval.txt.asc"
# approval.txt.asc is the approver's signature, kept as it is; the release signature covers every other file, approval.txt included
sign="$here/../actions/sign-files/sign.sh"
[ -f "$sign" ] || red "no $sign"
mv "$dist/approval.txt.asc" "$hold/approval-approver.asc"
bash "$sign" detached "$dist"
mv "$hold/approval-approver.asc" "$dist/approval.txt.approver.asc"
"$here/site-signaturen-pruefen.sh" "$dist" "$pub" "$name-$version"
