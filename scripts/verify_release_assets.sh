#!/usr/bin/env bash
# Verifies the files of a release BEFORE it is published (called by actions/publish-release/publish.sh; self test tests/packaging/verify_assets.sh):
#   1. `sha256sum -c SHA256SUMS.txt` in the asset folder,
#   2. the public key file contains the FIXED fingerprint (the gate, compared with the primary key of the imported file),
#   3. every `<file>.asc` is GOODSIG and VALIDSIG whose primary key (field 12, also when a signing subkey signed) is exactly that fingerprint;
#      REVKEYSIG, EXPKEYSIG, EXPSIG, KEYREVOKED, BADSIG, ERRSIG are red.
# Own temporary GNUPGHOME (under RUNNER_TEMP or TMPDIR), removed on exit; the user's keyring is never touched. Exit 0 all good, 1 any problem
# (every problem is printed as ::error::), 2 usage.
#   scripts/verify_release_assets.sh <asset dir> <public key .asc> <fingerprint, spaces allowed>
set -uo pipefail
[ "$#" -eq 3 ] || { echo "usage: $0 <asset dir> <public key> <fingerprint>" >&2; exit 2; }
dir="$1"; key="$2"; want="$(tr -d '[:space:]' <<<"$3" | tr '[:lower:]' '[:upper:]')"
problems=0
fail() { printf '::error::%s\n' "$*"; problems=$((problems + 1)); }
[ -d "$dir" ] || { echo "no such folder: $dir" >&2; exit 2; }
home="$(mktemp -d "${RUNNER_TEMP:-${TMPDIR:-/tmp}}/verify-gnupg.XXXXXX")" || exit 2
trap 'gpgconf --homedir "$home" --kill gpg-agent >/dev/null 2>&1; rm -rf "$home"' EXIT
chmod 700 "$home"

if [ -f "$dir/SHA256SUMS.txt" ]; then
  (cd "$dir" && sha256sum -c --quiet SHA256SUMS.txt) || fail "sha256sum -c SHA256SUMS.txt failed: a file differs from its hash or is missing"
else
  fail "SHA256SUMS.txt is missing"
fi

export GNUPGHOME="$home"
if ! gpg --batch --quiet --import "$key" 2>/dev/null; then
  fail "cannot import the public key $key"
else
  primary="$(gpg --batch --with-colons --list-keys 2>/dev/null | awk -F: '$1=="pub"{p=1;next} p&&$1=="fpr"{print $10; p=0}')"
  grep -qxF "$want" <<<"$primary" || fail "the public key has the fingerprint(s) [$(tr '\n' ' ' <<<"$primary")], not the fixed $want"
fi

signed=0
for sig in "$dir"/*.asc; do
  [ -e "$sig" ] || break
  data="${sig%.asc}"
  [ -f "$data" ] || { fail "$(basename "$sig") has no file $(basename "$data")"; continue; }
  signed=$((signed + 1))
  st="$(gpg --batch --status-fd 1 --verify "$sig" "$data" 2>/dev/null)"
  if grep -qE '^\[GNUPG:\] (REVKEYSIG|EXPKEYSIG|EXPSIG|KEYREVOKED|BADSIG|ERRSIG)' <<<"$st"; then
    fail "$(basename "$sig"): signature by a revoked or expired key, or a bad signature"
  elif ! grep -q '^\[GNUPG:\] GOODSIG' <<<"$st" || ! grep -q 'VALIDSIG' <<<"$st"; then
    fail "$(basename "$sig"): not a good signature"
  elif ! awk -v w="$want" '$2=="VALIDSIG"{ if ($12==w) ok=1 } END{exit !ok}' <<<"$st"; then
    fail "$(basename "$sig"): signed by another key than $want"
  fi
done
[ "$signed" -gt 0 ] || fail "no .asc signature found in $dir"
[ "$problems" -eq 0 ] || { echo "verify: $problems problem(s)"; exit 1; }
echo "verify: SHA256SUMS ok, $signed signature(s) by $want"
