#!/usr/bin/env bash
# Pattern: none (test). actions/publish-release/publish.sh against a mock `gh` (logs every call) and a test key: it must call
# `gh release edit` only when hashes, signatures and attestations are all fine. The script copy gets the TEST fingerprint and key (the real
# fixed fingerprint stays in publish.sh); the mutants MB (attestation guard deleted) and MC (ATTEST inverted) must change a result.
TP="$T/publish"; mkdir -p "$TP/gen" "$TP/rt" "$TP/bin"
if ! command -v gpg >/dev/null || ! command -v jq >/dev/null; then pass "publish: gpg or jq missing, skipped"; else
  chmod 700 "$TP/gen"; export RUNNER_TEMP="$TP/rt"
  for k in good other; do mkdir -p "$TP/gen/$k"; chmod 700 "$TP/gen/$k"
    printf 'Key-Type: EDDSA\nKey-Curve: ed25519\nName-Real: p-%s\nName-Email: t@example.org\nExpire-Date: 0\n%%no-protection\n%%commit\n' "$k" > "$TP/gen/$k.spec"
    GNUPGHOME="$TP/gen/$k" gpg --batch --quiet --generate-key "$TP/gen/$k.spec" 2>/dev/null; done
  PFPR=$(GNUPGHOME="$TP/gen/good" gpg --batch --with-colons --list-keys 2>/dev/null | awk -F: '$1=="fpr"{print $10; exit}')
  # a release tree as the action sees it: actions/publish-release/publish.sh, scripts/verify_release_assets.sh, release-signing-key.asc
  tree() { rm -rf "$TP/t"; mkdir -p "$TP/t/actions/publish-release" "$TP/t/scripts"
    sed "s/^FPR=\"[0-9A-F]*\"/FPR=\"$PFPR\"/" "$ROOT/actions/publish-release/publish.sh" > "$TP/t/actions/publish-release/publish.sh"
    cp "$ROOT/scripts/verify_release_assets.sh" "$TP/t/scripts/"; chmod +x "$TP/t/actions/publish-release/publish.sh" "$TP/t/scripts/verify_release_assets.sh"
    GNUPGHOME="$TP/gen/good" gpg --batch --armor --export > "$TP/t/release-signing-key.asc" 2>/dev/null; }
  assets() { rm -rf "$TP/as"; mkdir -p "$TP/as"; echo bin > "$TP/as/p.tar.gz"; (cd "$TP/as" && sha256sum p.tar.gz > SHA256SUMS.txt)
    for f in p.tar.gz SHA256SUMS.txt; do GNUPGHOME="$TP/gen/${1:-good}" gpg --batch --yes --detach-sign --armor -o "$TP/as/$f.asc" "$TP/as/$f" 2>/dev/null; done; view; }
  view() { (cd "$TP/as" && printf '%s\n' * | jq -R . | jq -s '{isDraft: true, isPrerelease: false, assets: map({name: .})}') > "$TP/view.json"; }
  cat > "$TP/bin/gh" <<'GH'
#!/usr/bin/env bash
echo "$*" >> "$MOCK_LOG"
case "$1 $2" in
  "release view") cat "$MOCK_VIEW" ;;
  "release download") pat=""; dir=""; shift 3 2>/dev/null; set -- $MOCK_ARGS
    while [ $# -gt 0 ]; do case "$1" in --pattern) pat=$2; shift ;; --dir) dir=$2; shift ;; esac; shift; done
    [ -z "${MOCK_DL_FAIL:-}" ] || exit 1
    if [ -n "$pat" ]; then cp "$MOCK_ASSETS/$pat" "$dir/"; else cp "$MOCK_ASSETS"/* "$dir/"; fi ;;
  "attestation verify") exit "${MOCK_ATTEST_RC:-0}" ;;
  "release edit") exit "${MOCK_EDIT_RC:-0}" ;;
  *) exit 3 ;;
esac
GH
  # the mock needs the original arguments of `download`: wrap it
  sed -i 's|^case "\$1 \$2" in|MOCK_ARGS="$*"\ncase "$1 $2" in|' "$TP/bin/gh"; chmod +x "$TP/bin/gh"
  P() { # <publish.sh> [VAR=val ...] -> rc; log in $TP/log
    local sc=$1; shift; : > "$TP/log"
    env PATH="$TP/bin:$PATH" MOCK_LOG="$TP/log" MOCK_VIEW="$TP/view.json" MOCK_ASSETS="$TP/as" TAG=v0.0.1 REPO=o/r GH_TOKEN=x DRY_RUN=false ATTEST=true "$@" bash "$sc" >"$TP/out" 2>&1; }
  S="$TP/t/actions/publish-release/publish.sh"
  edited() { grep -q '^release edit' "$TP/log"; }
  attested() { grep -q '^attestation verify' "$TP/log"; }
  tree; assets
  P "$S" && edited && attested && pass "publish: all good publishes (edit called, attestation checked)" || fail "publish: all good must publish: $(cat "$TP/out")"
  check_in "publish: the attestation call names the signing workflow" "$(cat "$TP/log")" "--signer-workflow FreeSynergy/fs-release/.github/workflows/_release-finalize.yml"
  P "$S" DRY_RUN=true && ! edited && pass "publish: dry-run is green and calls no edit" || fail "publish: dry-run must not edit"
  P "$S" MOCK_ATTEST_RC=1 && fail "publish: a missing attestation must be red" || { edited && fail "publish: missing attestation but edit called" || pass "publish: a missing attestation is red, no edit"; }
  P "$S" ATTEST=false && edited && ! attested && pass "publish: ATTEST=false publishes without the attestation call" || fail "publish: ATTEST=false"
  P "$S" MOCK_DL_FAIL=1 && fail "publish: a download failure must be red" || { edited && fail "publish: download failure but edit called" || pass "publish: a download failure is red, no edit"; }
  P "$S" MOCK_EDIT_RC=1 && fail "publish: a failing gh release edit must be red" || pass "publish: a failing gh release edit is red"
  cp "$TP/as/p.tar.gz" "$TP/p.bak"; echo tampered > "$TP/as/p.tar.gz"
  P "$S" && fail "publish: a changed file must be red" || { edited && fail "publish: hash error but edit called" || pass "publish: a hash error is red, no edit"; }
  assets other; P "$S" && fail "publish: a signature by another key must be red" || { edited && fail "publish: bad signature but edit called" || pass "publish: a signature by another key is red, no edit"; }
  assets; cp "$TP/view.json" "$TP/view.ok"; jq '.assets += [{name: "late.txt"}]' "$TP/view.ok" > "$TP/view.json"
  P "$S" && fail "publish: a file attached later (not downloaded) must be red" || { edited && fail "publish: extra asset but edit called" || pass "publish: assets that differ from the download are red, no edit"; }
  printf '{"isDraft": true, "isPrerelease": false, "assets": []}' > "$TP/view.json"
  P "$S" && fail "publish: an empty release must be red" || { edited && fail "publish: empty release but edit called" || pass "publish: an empty release is red, no edit"; }
  view; jq '.isDraft = false' "$TP/view.json" > "$TP/v2" && mv "$TP/v2" "$TP/view.json"
  P "$S" && ! edited && pass "publish: an already published release is left alone" || fail "publish: published release must not be edited"
  # mutants: MB attestation guard deleted, MC ATTEST inverted: the red cases above would turn green, i.e. these tests are real
  assets; sed 's/|| fail "\$(basename "\$f"): no valid build attestation.*$/|| true/' "$TP/t/actions/publish-release/publish.sh" > "$TP/t/actions/publish-release/mb.sh"
  P "$TP/t/actions/publish-release/mb.sh" MOCK_ATTEST_RC=1 && edited && pass "publish: mutant MB (attestation guard deleted) publishes without attestation: caught by the red case" || fail "publish: mutant MB must change the result"
  sed 's/\[\[ "\${ATTEST:-true}" == "true" \]\]/[[ "${ATTEST:-true}" != "true" ]]/' "$TP/t/actions/publish-release/publish.sh" > "$TP/t/actions/publish-release/mc.sh"
  P "$TP/t/actions/publish-release/mc.sh" MOCK_ATTEST_RC=1 && edited && pass "publish: mutant MC (ATTEST inverted) publishes despite a missing attestation: caught" || fail "publish: mutant MC must change the result"
  P "$TP/t/actions/publish-release/mc.sh" ATTEST=false; attested && pass "publish: mutant MC calls the attestation with ATTEST=false: caught" || fail "publish: mutant MC must call the attestation with ATTEST=false"
  for h in good other; do GNUPGHOME="$TP/gen/$h" gpgconf --kill gpg-agent 2>/dev/null; done; unset RUNNER_TEMP
fi
