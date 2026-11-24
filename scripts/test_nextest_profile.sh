#!/usr/bin/env bash
# Pattern: none. Self test of the `--profile` handling of scripts/test-gate.sh (input `profile` of check.yml) with a STUB cargo (no nextest needed): a profile the caller defines is passed on,
# a profile it does not define falls back to the default profile with a visible line and rc 0, a name outside the alphabet is refused (rc 2, nothing reaches the command line), no profile = no flag.
#   bash scripts/test_nextest_profile.sh
set -uo pipefail
here="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
t="$(mktemp -d "${TMPDIR:-/tmp}/nextest-profile.XXXXXX")"; trap 'rm -rf "$t"' EXIT
mkdir -p "$t/bin" "$t/with/.config" "$t/without"
cat > "$t/bin/cargo" <<'STUB'
#!/usr/bin/env bash
if [ "$1" = nextest ] && [ "$2" = --version ]; then echo "cargo-nextest 0.9.151 (stub)"; exit 0; fi
echo "STUB-CARGO $*"
if [ -n "${STUB_RED:-}" ] && [[ "$*" == *"--profile $STUB_RED "* ]]; then exit 100; fi
STUB
chmod +x "$t/bin/cargo"
printf '[profile.quick]\nfail-fast = true\n\n[profile.release.overrides]\n' > "$t/with/.config/nextest.toml"
fails=0
ok() { echo "ok   $1"; }
bad() { echo "FAIL $1"; fails=$((fails + 1)); }
# run <dir> <args...>: sets out and rc
run() { local d="$1"; shift; out="$(cd "$d" && PATH="$t/bin:$PATH" bash "$here/test-gate.sh" "$@" 2>&1)"; rc=$?; }

run "$t/with" --profile quick --workspace
if [ "$rc" -eq 0 ] && grep -q 'STUB-CARGO nextest run --color never --profile quick ' <<<"$out"; then ok "defined profile is passed to nextest"; else bad "defined profile not passed (rc $rc): $out"; fi
run "$t/with" --profile release --workspace
if [ "$rc" -eq 0 ] && grep -q -- '--profile release ' <<<"$out"; then ok "profile defined by a sub table ([profile.release.overrides]) counts"; else bad "sub table profile (rc $rc): $out"; fi
run "$t/with" --profile picture --workspace
if [ "$rc" -eq 0 ] && grep -q 'profile picture not defined here' <<<"$out" && ! grep -q -- '--profile' <<<"$(grep STUB-CARGO <<<"$out")"; then ok "missing profile: default + visible line, rc 0"; else bad "missing profile (rc $rc): $out"; fi
run "$t/without" --profile quick --workspace
if [ "$rc" -eq 0 ] && grep -q 'profile quick not defined here' <<<"$out" && ! grep -q -- '--profile' <<<"$(grep STUB-CARGO <<<"$out")"; then ok "repo without .config/nextest.toml: default + visible line, rc 0"; else bad "no config file (rc $rc): $out"; fi
run "$t/without" --profile default --workspace
if [ "$rc" -eq 0 ] && grep -q 'STUB-CARGO nextest run --color never --profile default ' <<<"$out"; then ok "profile default is always there"; else bad "profile default (rc $rc): $out"; fi
run "$t/with" --workspace
if [ "$rc" -eq 0 ] && ! grep -q -- '--profile' <<<"$out"; then ok "no profile given: no flag"; else bad "no profile (rc $rc): $out"; fi
for evil in 'quick --features x' 'quick;id' 'Quick' '-x' 'quick$(id)' 'a b' '../x' ''; do
  run "$t/with" --profile "$evil" --workspace
  if [ "$rc" -eq 2 ] && ! grep -q 'STUB-CARGO' <<<"$out"; then ok "refused: '$evil'"; else bad "'$evil' not refused (rc $rc): $out"; fi
done
run "$t/with" --profile
if [ "$rc" -eq 2 ]; then ok "--profile without a name is refused"; else bad "--profile alone (rc $rc)"; fi
# the doc test step ignores a profile (gates.sh never gives one to it)
run "$t/with" --doc --workspace
if [ "$rc" -eq 0 ] && grep -q 'STUB-CARGO test --doc' <<<"$out"; then ok "doc tests run plain cargo test --doc"; else bad "doc step (rc $rc): $out"; fi
run "$t/with" --profile quick --workspace
if grep 'STUB-CARGO nextest run' <<<"$out" | grep -q -- '--no-fail-fast'; then ok "--no-fail-fast always (beats fail-fast of a profile)"; else bad "no --no-fail-fast: $out"; fi
printf '[profile.security]\nfail-fast = true\n' >> "$t/with/.config/nextest.toml"
run "$t/with" --then-profile security --profile quick --workspace
if [ "$rc" -eq 0 ] && grep -q 'second run with profile security' <<<"$out" && grep -q 'STUB-CARGO nextest run --color never --profile security ' <<<"$out"; then ok "then-profile defined: second run"; else bad "then-profile (rc $rc): $out"; fi
run "$t/without" --then-profile security --workspace
if [ "$rc" -eq 0 ] && grep -q 'profile security not defined here, second run skipped' <<<"$out"; then ok "then-profile missing: skipped with a line, rc 0"; else bad "then-profile missing (rc $rc): $out"; fi
run "$t/with" --then-profile 'x;id' --workspace
if [ "$rc" -eq 2 ]; then ok "then-profile name refused"; else bad "then-profile evil (rc $rc)"; fi
# --only-profile (the landing-head runs): a profile the caller does not define is SKIPPED (nothing runs, rc 0), a defined one runs
run "$t/with" --only-profile picture --workspace
if [ "$rc" -eq 0 ] && grep -q 'profile picture not defined here.*run skipped' <<<"$out" && ! grep -q 'STUB-CARGO nextest run' <<<"$out"; then ok "only-profile missing: skipped, nothing runs, rc 0"; else bad "only-profile missing (rc $rc): $out"; fi
run "$t/with" --only-profile quick --workspace
if [ "$rc" -eq 0 ] && grep -q 'STUB-CARGO nextest run --color never --profile quick ' <<<"$out"; then ok "only-profile defined: runs with the profile"; else bad "only-profile defined (rc $rc): $out"; fi
run "$t/with" --only-profile 'x;id' --workspace
if [ "$rc" -eq 2 ] && ! grep -q 'STUB-CARGO' <<<"$out"; then ok "only-profile name refused"; else bad "only-profile evil (rc $rc): $out"; fi
# no colour codes: CARGO_TERM_COLOR=always of the CI would break the FAIL lines the list of re-checks reads (SOLL 4 of the review)
run "$t/with" --profile quick --workspace
if grep 'STUB-CARGO nextest run' <<<"$out" | grep -q -- '--color never'; then ok "nextest runs with --color never"; else bad "no --color never: $out"; fi
# counter proofs: no nextest = rc 64 and never green; a red second run (or a red first run) keeps the gate red
mkdir -p "$t/bin2"; printf '#!/usr/bin/env bash\nexit 1\n' > "$t/bin2/cargo"; chmod +x "$t/bin2/cargo"
out="$(cd "$t/with" && PATH="$t/bin2:$PATH" bash "$here/test-gate.sh" --profile quick --workspace 2>&1)"; rc=$?
if [ "$rc" -eq 64 ] && grep -q 'test gate NOT RUN' <<<"$out"; then ok "nextest missing: rc 64, NOT RUN"; else bad "nextest missing (rc $rc): $out"; fi
STUB_RED=security run "$t/with" --then-profile security --profile quick --workspace
if [ "$rc" -eq 100 ] && grep -q 'second run with profile security' <<<"$out"; then ok "red second run: gate red (rc 100)"; else bad "red second run (rc $rc): $out"; fi
STUB_RED=quick run "$t/with" --then-profile security --profile quick --workspace
if [ "$rc" -eq 100 ] && grep -q 'second run with profile security' <<<"$out"; then ok "red first run: still red, second run ran too (rc 100)"; else bad "red first run (rc $rc): $out"; fi
if [ "$fails" -eq 0 ]; then echo "all ok"; else echo "$fails failed"; exit 1; fi
