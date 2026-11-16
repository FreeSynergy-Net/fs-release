#!/usr/bin/env bash
# Pattern: none. The check of an approval ("Freigabe") before signing. Deterministic, no AI, no network. A missing input, a missing result
# or an unreadable value is RED (exit 1 or 2), never green; only the last line `freigabe ok` with exit 0 means yes.
#   site-freigabe-pruefen.sh <dist-dir> <approval.txt> <approval.txt.asc> <ro-keyring> <allowed-fingerprints> <run-id> [now-epoch]
# <approval.txt>   lines `key: value`: gate (site-signieren), name, version, mode (static|passthrough), tree-sha256, tar-sha256, run-id,
#                  expires (UTC `2026-11-17T12:00:00Z`). tar-sha256 is the sha256 of the TARBALL FILE (the bytes), tree-sha256 of its content.
# <ro-keyring>     a read-only OpenPGP keyring (gpg --no-default-keyring --keyring) that holds the approver's public key, NOT the user's keyring
# <allowed-...>    file with one 40-hex fingerprint of a SIGNING (SUB)KEY per line (the fingerprint gpg reports in VALIDSIG field 1, not the primary
#                  key's): only these may approve. A software subkey under the same primary key does NOT count unless it is listed. The sender
#                  is the signature, not a field.
# What it proves: the approval is signed by a listed key, names exactly this tarball (bytes recomputed from dist) and its content, this name,
# version and mode, THIS run (nonce), and has not expired; it is valid for 72 hours from the SIGNATURE time (not from the check time).
# dist holds exactly the four files of site-bauen and nothing else; the tarball passes the allowlist check (only plain files and folders, no
# link, device, special mode, 0777 folder, absolute or `..` path, other owner) BEFORE it is unpacked.
# Where the pieces come from (documented in templates/site/README.md): this script, the allowed list and the keyring come from the pinned main
# (commit recorded in the run); a change of the list is a commit the person signs. In CI the run id is the id of the earlier build run
# (`<GITHUB_RUN_ID>-<GITHUB_RUN_ATTEMPT>`), never a value an agent types in: with GITHUB_ACTIONS=true the format is enforced.
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
if [ "${GITHUB_ACTIONS:-}" = true ]; then
  [[ $run =~ ^[0-9]+-[0-9]+$ ]] || red "in CI the run id must be <run id>-<attempt> of the earlier build run"
  [ "$run" != "${GITHUB_RUN_ID:-x}-${GITHUB_RUN_ATTEMPT:-x}" ] || red "the run id must be the one of the EARLIER build run, not of this signing run"
fi
[[ $now =~ ^[0-9]+$ ]] || red "now is not an epoch"
grep -q -E '^[0-9A-F]{40}$' "$allowed" || red "no valid fingerprint line in $allowed"
if grep -v -E '^([0-9A-F]{40}|#.*|)$' "$allowed" | grep -q .; then red "$allowed holds a line that is neither a fingerprint nor a comment"; fi

# 1. the signature: GOODSIG and VALIDSIG, nothing bad, the SIGNING (sub)key fingerprint (VALIDSIG field 1) is in the list
st=$(gpg --batch --no-default-keyring --keyring "$ring" --status-fd 1 --verify "$sig" "$appr" 2>/dev/null) || red "signature does not verify"
grep -q '^\[GNUPG:\] GOODSIG ' <<<"$st" || red "no GOODSIG"
if grep -q -E '^\[GNUPG:\] (BADSIG|ERRSIG|EXPSIG|EXPKEYSIG|REVKEYSIG|NO_PUBKEY)' <<<"$st"; then red "signature is bad, expired, revoked or from an unknown key"; fi
fpr=$(awk '$2=="VALIDSIG"{print $3; exit}' <<<"$st")
sigtime=$(awk '$2=="VALIDSIG"{print $5; exit}' <<<"$st")
[[ $fpr =~ ^[0-9A-F]{40}$ ]] || red "no VALIDSIG signing-key fingerprint"
[[ $sigtime =~ ^[0-9]+$ ]] || red "no VALIDSIG signature time"
grep -q -x "$fpr" "$allowed" || red "signing key $fpr is not on the list of allowed approval keys"
[ "$sigtime" -le $((now + 300)) ] || red "the signature is dated in the future"

# 2. the content of the approval: exact keys, each exactly once
val() { local n; n=$(grep -c -E "^$1: " "$appr"); [ "$n" = 1 ] || red "approval: key '$1' must appear exactly once (found $n)"; sed -n -E "s/^$1: (.*)$/\1/p" "$appr"; }
a_gate=$(val gate) || exit 1; a_name=$(val name) || exit 1; a_version=$(val version) || exit 1; a_mode=$(val mode) || exit 1
a_tree=$(val tree-sha256) || exit 1; a_tar=$(val tar-sha256) || exit 1; a_run=$(val run-id) || exit 1; a_exp=$(val expires) || exit 1
[ "$a_gate" = "site-signieren" ] || red "approval is for another gate"
[[ $a_name =~ ^[a-z0-9][a-z0-9.-]*$ && $a_version =~ ^[0-9][0-9A-Za-z.+-]*$ ]] || red "name or version of the approval is not plain"
[[ $a_mode == static || $a_mode == passthrough ]] || red "mode must be static or passthrough"
[ "$a_run" = "$run" ] || red "approval is for another run ($a_run, expected $run)"
[[ $a_tree =~ ^[0-9a-f]{64}$ ]] || red "tree-sha256 is not a sha256"
[[ $a_tar =~ ^[0-9a-f]{64}$ ]] || red "tar-sha256 is not a sha256"
[[ $a_exp =~ ^[0-9]{4}-[0-9]{2}-[0-9]{2}T[0-9]{2}:[0-9]{2}:[0-9]{2}Z$ ]] || red "expires must be UTC like 2026-11-17T12:00:00Z"
exp=$(date -u -d "$a_exp" +%s 2>/dev/null) || red "expires is not a date: $a_exp"
[ "$exp" -gt "$now" ] || red "approval expired at $a_exp"
[ "$exp" -le $((sigtime + 72 * 3600)) ] || red "approval is valid for more than 72 hours from the signature time"

# 3. dist: exactly the four files of site-bauen, nothing else (a foreign file would be signed with the release key)
base="$a_name-$a_version"
want=$(printf '%s\n' "$base-manifest.json" "$base-site.tar.gz" "$base.spdx.json" "SHA256SUMS.txt" | LC_ALL=C sort)
have=$(cd "$dist" && find . -mindepth 1 -printf '%P\t%y\n' | LC_ALL=C sort)
[ "$have" = "$(while IFS= read -r n; do printf '%s\tf\n' "$n"; done <<<"$want")" ] || red "dist must hold exactly: $(tr '\n' ' ' <<<"$want")(found: $(tr '\t\n' ': ' <<<"$have"))"

# 4. the object: the BYTES of the tarball, its allowlist check, its content hash, the sums, the manifest
tar="$dist/$base-site.tar.gz"
(cd "$dist" && sha256sum --check --strict --quiet SHA256SUMS.txt) 2>/dev/null || red "SHA256SUMS.txt does not match the files"
[ "$(wc -l < "$dist/SHA256SUMS.txt")" = 3 ] || red "SHA256SUMS.txt must list exactly the three files"
grep -q -F "  $base-site.tar.gz" "$dist/SHA256SUMS.txt" || red "tarball not listed in SHA256SUMS.txt"
realtar=$(sha256sum "$tar" | cut -d' ' -f1)
[ "$realtar" = "$a_tar" ] || red "tarball bytes $realtar differ from the approved tar-sha256 $a_tar"
tar_check "$tar" || red "tarball is not made of plain files and folders only"
real=$(tar_tree_hash "$tar") || red "tarball cannot be read"
[ "$real" = "$a_tree" ] || red "tarball content hash $real differs from the approved $a_tree"
python3 -I -c 'import json,sys; m=json.load(open(sys.argv[1])); sys.exit(0 if (m["tree_sha256"],m["tar_sha256"],m["mode"],m["name"],m["version"])==tuple(sys.argv[2:7]) else 1)' \
  "$dist/$base-manifest.json" "$real" "$realtar" "$a_mode" "$a_name" "$a_version" || red "manifest names another hash, mode, name or version"
echo "approver key $fpr, $a_name $a_version ($a_mode), tarball $realtar, tree $real, run $run"
echo "freigabe ok"
