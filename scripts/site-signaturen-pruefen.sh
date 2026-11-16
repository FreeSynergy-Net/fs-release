#!/usr/bin/env bash
# Pattern: none. Checks the signatures of a signed site folder (step `signatures-check`; also what the person runs before delivering).
#   site-signaturen-pruefen.sh <dist-dir> <release-pubkey.asc> <name-version>        e.g. ... dist release-key.asc example.org-0.0.1
# The folder must hold exactly: the four files of site-bauen, approval.txt, each with its `.asc`, plus approval.txt.approver.asc (the approver's
# own signature, checked by site-freigabe-pruefen.sh against the approver's key, not here). Each `.asc` must verify against the public release key
# (private keyring made of this key only) and be made by exactly its fingerprint. A missing file, a foreign file or a missing signature is RED.
set -uo pipefail
red() { echo "RED: $*" >&2; exit 1; }
[ "$#" -eq 3 ] || { echo "usage: site-signaturen-pruefen.sh <dist-dir> <release-pubkey.asc> <name-version>" >&2; exit 2; }
dist=$1 pub=$2 base=$3
[ -d "$dist" ] || red "no folder $dist"
[ -s "$pub" ] || red "no public release key $pub"
[[ $base =~ ^[a-z0-9][a-z0-9.-]*-[0-9][0-9A-Za-z.+-]*$ ]] || red "name-version is not plain"
files=("$base-manifest.json" "$base-site.tar.gz" "$base.spdx.json" SHA256SUMS.txt approval.txt)
want=$( { for f in "${files[@]}"; do printf '%s\n%s.asc\n' "$f" "$f"; done; echo approval.txt.approver.asc; } | LC_ALL=C sort)
have=$(cd "$dist" && find . -mindepth 1 -printf '%P\t%y\n' | LC_ALL=C sort)
[ "$have" = "$(while IFS= read -r n; do printf '%s\tf\n' "$n"; done <<<"$want")" ] || red "folder must hold exactly: $(tr '\n' ' ' <<<"$want")"
GNUPGHOME=$(mktemp -d); export GNUPGHOME; trap 'gpgconf --kill gpg-agent >/dev/null 2>&1; rm -rf "$GNUPGHOME"' EXIT
gpg --batch -q --import "$pub" 2>/dev/null || red "cannot import $pub"
fp=$(gpg --batch --list-keys --with-colons | awk -F: '/^fpr:/{print $10; exit}')
[[ $fp =~ ^[0-9A-F]{40}$ ]] || red "cannot read the fingerprint of $pub"
n=0
for f in "${files[@]}"; do
  [ -s "$dist/$f.asc" ] || red "no signature for $f"
  st=$(gpg --batch --status-fd 1 --verify "$dist/$f.asc" "$dist/$f" 2>/dev/null) || red "signature of $f does not verify"
  grep -q '^\[GNUPG:\] GOODSIG ' <<<"$st" || red "no GOODSIG for $f"
  [ "$(awk '$2=="VALIDSIG"{print $12; exit}' <<<"$st")" = "$fp" ] || red "signature of $f is not by $fp"
  n=$((n + 1))
done
echo "signed $n files by $fp"
