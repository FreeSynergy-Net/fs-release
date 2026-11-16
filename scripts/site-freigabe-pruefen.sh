#!/usr/bin/env bash
# Pattern: none. The check of an approval ("Freigabe") before signing. Deterministic, no AI, no network. A missing input, a missing result
# or an unreadable value is RED (exit 1 or 2), never green; only the last line `freigabe ok` with exit 0 means yes.
#   site-freigabe-pruefen.sh <dist-dir> <approval.txt> <approval.txt.asc> <ro-keyring> <allowed-fingerprints> <run-id> [now-epoch]
# <approval.txt>   lines `key: value`: gate (site-signieren), name, version, tree-sha256, run-id, expires (UTC `2026-11-17T12:00:00Z`)
# <ro-keyring>     a read-only OpenPGP keyring (gpg --no-default-keyring --keyring) that holds the approver's public key, NOT the user's keyring
# <allowed-...>    file with one 40-hex primary fingerprint per line: only these may approve (the sender is the signature, not a field)
# What it proves: the approval is signed by an allowed key, names exactly this folder state (hash recomputed FROM THE TARBALL, not from the
# manifest), this name and version, THIS run (nonce), and has not expired (and is not valid for more than 72 hours).
# Run it from the pinned main checkout, never from an agent worktree: whoever changes this script changes the check.
set -uo pipefail
here=$(cd "$(dirname "$0")" && pwd)
# shellcheck source=site-lib.sh
. "$here/site-lib.sh"
red() { echo "RED: $*" >&2; exit 1; }
[ "$#" -ge 6 ] || { echo "usage: site-freigabe-pruefen.sh <dist-dir> <approval> <approval.asc> <keyring> <allowed-fingerprints> <run-id> [now-epoch]" >&2; exit 2; }
dist=$1 appr=$2 sig=$3 ring=$4 allowed=$5 run=$6 now=${7:-$(date -u +%s)}
for f in "$appr" "$sig" "$ring" "$allowed"; do [ -s "$f" ] || red "missing or empty input: $f"; done
[ -d "$dist" ] || red "no dist directory $dist"
[[ $run =~ ^[A-Za-z0-9._-]{4,64}$ ]] || red "run-id is not a plain token"
[[ $now =~ ^[0-9]+$ ]] || red "now is not an epoch"
grep -q -E '^[0-9A-F]{40}$' "$allowed" || red "no valid fingerprint line in $allowed"
if grep -v -E '^([0-9A-F]{40}|#.*|)$' "$allowed" | grep -q .; then red "$allowed holds a line that is neither a fingerprint nor a comment"; fi

# 1. the signature: GOODSIG and VALIDSIG, nothing bad, primary fingerprint in the list
st=$(gpg --batch --no-default-keyring --keyring "$ring" --status-fd 1 --verify "$sig" "$appr" 2>/dev/null) || red "signature does not verify"
grep -q '^\[GNUPG:\] GOODSIG ' <<<"$st" || red "no GOODSIG"
if grep -q -E '^\[GNUPG:\] (BADSIG|ERRSIG|EXPSIG|EXPKEYSIG|REVKEYSIG|NO_PUBKEY)' <<<"$st"; then red "signature is bad, expired, revoked or from an unknown key"; fi
fpr=$(awk '$2=="VALIDSIG"{print $12; exit}' <<<"$st")
[[ $fpr =~ ^[0-9A-F]{40}$ ]] || red "no VALIDSIG primary fingerprint"
grep -q -x "$fpr" "$allowed" || red "signer $fpr is not on the list of allowed approvers"

# 2. the content of the approval: exact keys, each exactly once
val() { local n; n=$(grep -c -E "^$1: " "$appr"); [ "$n" = 1 ] || red "approval: key '$1' must appear exactly once (found $n)"; sed -n -E "s/^$1: (.*)$/\1/p" "$appr"; }
a_gate=$(val gate) || exit 1; a_name=$(val name) || exit 1; a_version=$(val version) || exit 1; a_tree=$(val tree-sha256) || exit 1
a_run=$(val run-id) || exit 1; a_exp=$(val expires) || exit 1
[ "$a_gate" = "site-signieren" ] || red "approval is for another gate"
[[ $a_name =~ ^[a-z0-9][a-z0-9.-]*$ && $a_version =~ ^[0-9][0-9A-Za-z.+-]*$ ]] || red "name or version of the approval is not plain"
[ "$a_run" = "$run" ] || red "approval is for another run ($a_run, expected $run)"
[[ $a_tree =~ ^[0-9a-f]{64}$ ]] || red "tree-sha256 is not a sha256"
exp=$(date -u -d "$a_exp" +%s 2>/dev/null) || red "expires is not a date: $a_exp"
[[ $a_exp =~ ^[0-9]{4}-[0-9]{2}-[0-9]{2}T[0-9]{2}:[0-9]{2}:[0-9]{2}Z$ ]] || red "expires must be UTC like 2026-11-17T12:00:00Z"
[ "$exp" -gt "$now" ] || red "approval expired at $a_exp"
[ "$exp" -le $((now + 72 * 3600)) ] || red "approval is valid for more than 72 hours"

# 3. the object: the tarball in dist, its hash recomputed, listed in the checked sums, equal to the approved tree hash
tar="$dist/$a_name-$a_version-site.tar.gz"
[ -f "$tar" ] || red "no tarball $tar"
(cd "$dist" && sha256sum --check --strict --quiet SHA256SUMS.txt) 2>/dev/null || red "SHA256SUMS.txt does not match the files"
grep -q -F "$a_name-$a_version-site.tar.gz" "$dist/SHA256SUMS.txt" || red "tarball not listed in SHA256SUMS.txt"
real=$(tar_tree_hash "$tar") || red "tarball cannot be read"
[ "$real" = "$a_tree" ] || red "tarball content hash $real differs from the approved $a_tree"
man="$dist/$a_name-$a_version-manifest.json"
[ -f "$man" ] || red "no manifest"
python3 -I -c 'import json,sys; m=json.load(open(sys.argv[1])); sys.exit(0 if m["tree_sha256"]==sys.argv[2] else 1)' "$man" "$real" || red "manifest names another tree hash"
echo "approver $fpr, $a_name $a_version, tree $real, run $run"
echo "freigabe ok"
