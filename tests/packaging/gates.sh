#!/usr/bin/env bash
# Pattern: none (test). check.yml must not carry its own copy of the gate flags: it runs scripts/gates.sh (one definition) and the pinned toolchain.
W="$ROOT/.github/workflows/check.yml"; G="$ROOT/scripts/gates.sh"
grep -q 'clippy --workspace --all-targets --all-features --locked --keep-going -- -D warnings -W clippy::pedantic' "$G" && pass "gates: gates.sh defines clippy -D warnings + pedantic" || fail "gates: gates.sh lacks the clippy flags"
grep -Eq 'cargo (fmt|clippy|test)' "$W" && fail "gates: check.yml has its own cargo command (must only call actions/gates)" || pass "gates: check.yml has no own cargo commands"
grep -q 'actions/gates@' "$W" && pass "gates: check.yml calls actions/gates" || fail "gates: check.yml does not call actions/gates"
grep -q 'gates.sh' "$ROOT/actions/gates/action.yml" && pass "gates: actions/gates runs gates.sh" || fail "gates: actions/gates does not run gates.sh"
grep -q 'toolchain: pinned' "$W" && pass "gates: check.yml uses the pinned toolchain" || fail "gates: check.yml toolchain not pinned"
grep -q 'RUST_VERSION' "$G" && pass "gates: gates.sh reads RUST_VERSION" || fail "gates: gates.sh ignores RUST_VERSION"
grep -Eq '^[0-9]+\.[0-9]+\.[0-9]+$' "$ROOT/RUST_VERSION" && pass "gates: RUST_VERSION is an exact x.y.z" || fail "gates: RUST_VERSION must be exact x.y.z"
# Release builds use the same pinned toolchain as the gates: no 'stable', no other 'toolchain:' value (one allowed: the msrv
# input in ci.yml), no own rustup/cargo +x in a run step. Comments are ignored; the helper is checked with fixtures below.
TS="$ROOT/tests/packaging/toolchain_scan.py"
if bad=$(python3 -I "$TS" "$ROOT"); then pass "gates: no workflow/action names stable or an unpinned toolchain, release builds pinned"; else fail "gates: unpinned toolchain: $bad"; fi
TG="$T/gates"; mkdir -p "$TG/.github/workflows" "$TG/actions"
scan() { python3 -I "$TS" "$TG" >/dev/null 2>&1; }
printf 'jobs:\n  a:\n    steps:\n      - uses: x/setup-rust@1 # stable is not meant here\n        with:\n          toolchain: pinned # the pinned one\n      # stable\n' > "$TG/.github/workflows/c.yml"
scan && pass "gates: 'stable' in a comment and a trailing comment after 'pinned' are fine" || fail "gates: comments must not count"
printf 'jobs:\n  a:\n    steps:\n      - run: rustup default stable\n' > "$TG/.github/workflows/c.yml"
scan && fail "gates: run: rustup default stable must fail" || pass "gates: run: rustup default stable fails"
printf 'jobs:\n  a:\n    steps:\n      - run: cargo +nightly fmt\n' > "$TG/.github/workflows/c.yml"
scan && fail "gates: run: cargo +nightly must fail" || pass "gates: run: cargo +nightly fails"
printf 'jobs:\n  a:\n    steps:\n      - uses: x/setup-rust@1\n        with:\n          toolchain: stable\n' > "$TG/.github/workflows/c.yml"
scan && fail "gates: toolchain: stable must fail" || pass "gates: toolchain: stable fails"
rm -f "$TG/.github/workflows/c.yml"
printf 'jobs:\n  a:\n    steps:\n      - uses: x/setup-rust@1\n        with:\n          a: 1\n          b: 2\n          c: 3\n          toolchain: pinned\n' > "$TG/.github/workflows/_release-x.yml"
scan && pass "gates: toolchain: pinned far below uses: is found" || fail "gates: toolchain more than 2 lines below uses: must pass"
printf 'jobs:\n  a:\n    steps:\n      - uses: x/setup-rust@1\n      - uses: y/other@1\n        with:\n          toolchain: pinned\n' > "$TG/.github/workflows/_release-x.yml"
scan && fail "gates: a pinned toolchain in the NEXT step must not count" || pass "gates: setup-rust without its own toolchain: pinned fails"
printf 'jobs:\n  a:\n    steps:\n      - uses: x/setup-rust@1\n  b:\n    steps:\n      - uses: y@1\n        with:\n          toolchain: pinned\n' > "$TG/.github/workflows/_release-x.yml"
scan && fail "gates: a pinned toolchain in the NEXT JOB must not count" || pass "gates: a step ends at the end of its job"
printf 'jobs:\n  a:\n    steps:\n      - run: rustup default stable\n' > "$TG/.github/workflows/z.yaml"; rm -f "$TG/.github/workflows/_release-x.yml" "$TG/.github/workflows/c.yml"
scan && fail "gates: *.yaml must be scanned" || pass "gates: *.yaml files are scanned"
rm -rf "$TG"
grep -q 'default: pinned' "$ROOT/actions/setup-rust/action.yml" && pass "gates: setup-rust defaults to pinned" || fail "gates: setup-rust default is not pinned"
# --locked lint: every `cargo clippy` / `cargo test` line of gates.sh carries --locked; the old lines (without it) must be red.
unlocked() { grep -E '^[[:space:]]*([^#[:space:]].*[[:space:]])?cargo (clippy|test)( |$)' "$1" | grep -vq -- '--locked'; }  # [GATES-SAMMELLISTE]: the lines are `step <name> <severity> cargo ...`, so not only line starts count
if unlocked "$G"; then fail "gates: gates.sh has a cargo clippy/test line without --locked"; else pass "gates: every cargo clippy/test line of gates.sh has --locked"; fi
printf 'cargo clippy --workspace --all-targets --all-features -- -D warnings -W clippy::pedantic\ncargo test --workspace --all-features\n' > "$T/gates-old.sh"
if unlocked "$T/gates-old.sh"; then pass "gates: the OLD lines (no --locked) are red"; else fail "gates: the old lines must be red"; fi
printf 'cargo clippy --workspace --all-targets --all-features --locked -- -D warnings\ncargo test --workspace --all-features --locked\n' > "$T/gates-new.sh"
if unlocked "$T/gates-new.sh"; then fail "gates: the new lines must be green"; else pass "gates: the new lines are green"; fi
printf 'cargo clippy --workspace --locked\ncargo test --workspace\n' > "$T/gates-one.sh"
if unlocked "$T/gates-one.sh"; then pass "gates: one line without --locked among locked ones is red"; else fail "gates: one unlocked line must be red"; fi
# FS_GATES_SKIP_TEST (named exception of scripts/gates-komplett): only the ONE entry may be skipped, visibly; any other value is refused; unset = no skip.
GK="$T/gk"; mkdir -p "$GK/scripts" "$GK/root" "$GK/bin"
cp "$G" "$GK/scripts/gates.sh"; cp "$ROOT/scripts/lib_gates_guard.sh" "$ROOT/scripts/lib_gates_list.sh" "$GK/scripts/"; git init -q "$GK/root"; git -C "$GK/root" -c user.name=t -c user.email=t@t commit -q --allow-empty -m x; git -C "$GK/root" config --local freesynergy.identity-test-base HEAD; echo 1.0.0 > "$GK/RUST_VERSION"; printf '#!/usr/bin/env bash\nexit 0\n' > "$GK/scripts/gates-lock.sh"; cp "$GK/scripts/gates-lock.sh" "$GK/scripts/gates-files.sh"; cp "$ROOT/scripts/identity-check" "$GK/scripts/"; : > "$GK/scripts/check_rust_rules.py"
printf '#!/usr/bin/env bash\necho "cargo $*" >> "%s/cargo.log"\n' "$GK" > "$GK/bin/cargo"; printf '#!/usr/bin/env bash\necho rustc-stub\n' > "$GK/bin/rustc"; chmod +x "$GK/bin/cargo" "$GK/bin/rustc"
skiprun() { : > "$GK/cargo.log"  # <script> <value or empty>: runs a copy of gates.sh with stub cargo; output in $GK/out
  PATH="$GK/bin:$PATH" CI=true FS_GATES_SKIP_TEST="$2" bash "$1" "$GK/root" > "$GK/out" 2>&1; }
OK=released_changelog_sections_equal_their_tags
rc=0; skiprun "$GK/scripts/gates.sh" "" || rc=$?
if [ "$rc" -eq 0 ] && grep -q 'cargo test' "$GK/cargo.log" && ! grep -q -- '--skip' "$GK/cargo.log" && ! grep -q SKIPPED "$GK/out"; then pass "gates: FS_GATES_SKIP_TEST unset skips nothing"; else fail "gates: unset variable must skip nothing (rc $rc)"; fi
rc=0; skiprun "$GK/scripts/gates.sh" "$OK" || rc=$?
if [ "$rc" -eq 0 ] && grep -q -- "-- --skip $OK" "$GK/cargo.log" && grep -q "SKIPPED" "$GK/out"; then pass "gates: the named exception test is skipped and says so"; else fail "gates: the named exception must be skipped visibly (rc $rc)"; fi
rc=0; skiprun "$GK/scripts/gates.sh" "some_other_test" || rc=$?
if [ "$rc" -ne 0 ] && ! grep -q 'cargo test' "$GK/cargo.log" && grep -q 'no exception entry' "$GK/out"; then pass "gates: a value without an exception entry is refused, nothing skipped"; else fail "gates: any other test name must be refused (rc $rc)"; fi
# mutant: the variable takes effect without an exception entry (the allow check removed) must turn the refusal case red
python3 -I - "$G" "$GK/scripts/gates-mut.sh" <<'PY'
import sys
t = open(sys.argv[1], encoding="utf-8").read()
old = '[ "$FS_GATES_SKIP_TEST" != released_changelog_sections_equal_their_tags ]'
assert old in t
open(sys.argv[2], "w", encoding="utf-8").write(t.replace(old, "false", 1))
PY
rc=0; skiprun "$GK/scripts/gates-mut.sh" "some_other_test" || rc=$?
if [ "$rc" -ne 0 ] && ! grep -q 'cargo test' "$GK/cargo.log" && grep -q 'no exception entry' "$GK/out"; then fail "gates: MUTANT (skip without an exception entry) SURVIVED the refusal case"; else pass "gates: mutant (skip without an exception entry) is caught by the refusal case"; fi
