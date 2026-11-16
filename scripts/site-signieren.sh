#!/usr/bin/env bash
# Pattern: Strategy-by-presence is turned around on purpose: the existing actions/sign-files/sign.sh says "not signed" and exits 0 when the key is
# missing. Here a missing key, a missing approval or a missing signature is RED. This is the `sign` job of ADR 0078 (the only job with the key).
#   GPG_PRIVATE_KEY=... [GPG_PASSPHRASE=...] site-signieren.sh <dist-dir> <approval> <approval.asc> <ro-keyring> <allowed-fingerprints> <run-id> <release-pubkey.asc>
# Steps: 1. key present   2. approval check (site-freigabe-pruefen.sh)   3. artefacts equal their SHA256SUMS.txt   4. approval and its
# signature join the release as proof   5. detached .asc for every file (sign.sh detached), SHA256SUMS.txt included
# 6. every file has an .asc that verifies against <release-pubkey.asc>, made by exactly the fingerprint in that file.
set -euo pipefail
here=$(cd "$(dirname "$0")" && pwd)
red() { echo "RED: $*" >&2; exit 1; }
[ "$#" -eq 7 ] || { echo "usage: site-signieren.sh <dist-dir> <approval> <approval.asc> <ro-keyring> <allowed-fingerprints> <run-id> <release-pubkey.asc>" >&2; exit 2; }
dist=$1 appr=$2 sig=$3 ring=$4 allowed=$5 run=$6 pub=$7
[ -n "${GPG_PRIVATE_KEY:-}" ] || red "GPG_PRIVATE_KEY is not set: nothing is signed, and that is not green"
[ -s "$pub" ] || red "no public release key $pub"
"$here/site-freigabe-pruefen.sh" "$dist" "$appr" "$sig" "$ring" "$allowed" "$run" || red "approval check failed"
(cd "$dist" && sha256sum --check --strict --quiet SHA256SUMS.txt) || red "artefacts differ from SHA256SUMS.txt"
cp -- "$appr" "$dist/approval.txt"; cp -- "$sig" "$dist/approval.txt.asc"
# approval.txt.asc is the approver's signature, kept as it is; the release signature covers every other file, approval.txt included
sign="$here/../actions/sign-files/sign.sh"
[ -f "$sign" ] || red "no $sign"
hold=$(mktemp -d); trap 'rm -rf "$hold"' EXIT
mv "$dist/approval.txt.asc" "$hold/approval-approver.asc"
bash "$sign" detached "$dist"
mv "$hold/approval-approver.asc" "$dist/approval.txt.approver.asc"
# verification with a private keyring made of the public release key only
export GNUPGHOME; GNUPGHOME=$(mktemp -d); trap 'rm -rf "$hold" "$GNUPGHOME"' EXIT
gpg --batch -q --import "$pub"
want=$(gpg --batch --list-keys --with-colons | awk -F: '/^fpr:/{print $10; exit}')
[[ $want =~ ^[0-9A-F]{40}$ ]] || red "cannot read the fingerprint of $pub"
n=0
while IFS= read -r -d '' f; do
  case $f in *.asc) continue ;; esac
  [ -s "$f.asc" ] || red "no signature for $(basename "$f")"
  st=$(gpg --batch --status-fd 1 --verify "$f.asc" "$f" 2>/dev/null) || red "signature of $(basename "$f") does not verify"
  [ "$(awk '$2=="VALIDSIG"{print $12; exit}' <<<"$st")" = "$want" ] || red "signature of $(basename "$f") is not by $want"
  n=$((n + 1))
done < <(find "$dist" -maxdepth 1 -type f -print0)
[ "$n" -ge 5 ] || red "only $n signed files (expected at least tarball, manifest, sbom, sums, approval)"
echo "signed $n files by $want"
