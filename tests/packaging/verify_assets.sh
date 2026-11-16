#!/usr/bin/env bash
# Pattern: none (test). scripts/verify_release_assets.sh: hashes, signature by the FIXED fingerprint, wrong hash/signature/fingerprint are red. Test key made here only.
TV="$T/verify"; mkdir -p "$TV/gen" "$TV/rt"
if ! command -v gpg >/dev/null; then pass "verify-assets: gpg missing, skipped"; else
  chmod 700 "$TV/gen"; export RUNNER_TEMP="$TV/rt"
  gk() { GNUPGHOME="$TV/gen/$1" gpg --batch --quiet --generate-key "$TV/gen/$1.spec" 2>/dev/null; }
  for k in good other; do mkdir -p "$TV/gen/$k"; chmod 700 "$TV/gen/$k"
    printf 'Key-Type: EDDSA\nKey-Curve: ed25519\nName-Real: t-%s\nName-Email: t@example.org\nExpire-Date: 0\n%%no-protection\n%%commit\n' "$k" > "$TV/gen/$k.spec"; gk "$k"; done
  FPR=$(GNUPGHOME="$TV/gen/good" gpg --batch --with-colons --list-keys 2>/dev/null | awk -F: '$1=="fpr"{print $10; exit}')
  OTHER=$(GNUPGHOME="$TV/gen/other" gpg --batch --with-colons --list-keys 2>/dev/null | awk -F: '$1=="fpr"{print $10; exit}')
  GNUPGHOME="$TV/gen/good" gpg --batch --armor --export > "$TV/good.asc" 2>/dev/null
  mk() { rm -rf "$TV/a"; mkdir -p "$TV/a"; echo one > "$TV/a/f1.txt"; echo two > "$TV/a/f2.txt"
    (cd "$TV/a" && sha256sum f1.txt f2.txt > SHA256SUMS.txt)
    for f in f1.txt f2.txt SHA256SUMS.txt; do GNUPGHOME="$TV/gen/${1:-good}" gpg --batch --detach-sign --armor -o "$TV/a/$f.asc" "$TV/a/$f" 2>/dev/null; done; }
  V() { "$ROOT/scripts/verify_release_assets.sh" "$TV/a" "$TV/good.asc" "${1:-$FPR}" >"$TV/out" 2>&1; }
  mk; V && pass "verify-assets: good hashes + good signatures + right fingerprint is green" || fail "verify-assets: must be green: $(cat "$TV/out")"
  V "$(sed 's/\(....\)/\1 /g' <<<"$FPR")" && pass "verify-assets: a fingerprint with spaces is accepted" || fail "verify-assets: spaced fingerprint: $(cat "$TV/out")"
  mk; echo changed > "$TV/a/f1.txt"; V && fail "verify-assets: a changed file must be red" || pass "verify-assets: a changed file (hash) is red"
  check_in "verify-assets: hash failure is named" "$(cat "$TV/out")" "sha256sum"
  mk; echo changed > "$TV/a/f2.txt"; (cd "$TV/a" && sha256sum f1.txt f2.txt > SHA256SUMS.txt); V && fail "verify-assets: a changed file with a fitting hash but old signature must be red" || pass "verify-assets: hash fits but the signature no longer does: red"
  mk other; V && fail "verify-assets: a signature by an unknown key must be red" || pass "verify-assets: signed by a key that is not in the key file is red"
  # a key file that ALSO holds the other key: its signature is a good one, but not by the fixed fingerprint
  GNUPGHOME="$TV/gen/other" gpg --batch --armor --export >> "$TV/good.asc" 2>/dev/null
  mk other; V && fail "verify-assets: a good signature by another key must be red" || pass "verify-assets: a good signature by another key than the fixed fingerprint is red"
  check_in "verify-assets: wrong signer is named" "$(cat "$TV/out")" "signed by another key"
  mk; V "$OTHER" && fail "verify-assets: a wrong fixed fingerprint must be red" || pass "verify-assets: a wrong fixed fingerprint is red (key file does not carry it)"
  mk; rm "$TV/a/SHA256SUMS.txt"; V && fail "verify-assets: no SHA256SUMS.txt must be red" || pass "verify-assets: missing SHA256SUMS.txt is red"
  mk; rm "$TV/a/"*.asc; V && fail "verify-assets: no signatures must be red" || pass "verify-assets: no .asc at all is red"
  # mutant: the VALIDSIG primary-fingerprint comparison removed -> the foreign-signer case would pass (so the test above is real)
  sed 's/if (\$12==w) ok=1/ok=1/' "$ROOT/scripts/verify_release_assets.sh" > "$TV/mut.sh"; chmod +x "$TV/mut.sh"
  mk other; "$TV/mut.sh" "$TV/a" "$TV/good.asc" "$FPR" >/dev/null 2>&1 && pass "verify-assets: mutant (signer comparison removed) lets the foreign signer through, so the red test is real" || fail "verify-assets: mutant must pass the foreign signer"
  [[ -z "$(ls -A "$TV/rt")" ]] && pass "verify-assets: no GNUPGHOME left in RUNNER_TEMP" || fail "verify-assets: left in RUNNER_TEMP: $(ls -A "$TV/rt")"
  GNUPGHOME="$TV/gen/good" gpgconf --kill gpg-agent 2>/dev/null; GNUPGHOME="$TV/gen/other" gpgconf --kill gpg-agent 2>/dev/null
  unset RUNNER_TEMP
fi
