#!/usr/bin/env bash
# Pattern: none (test). A set CARGO_TARGET_DIR (build wrappers set it) is honoured: the stub cargo puts the binaries
# where package-linux looks for them, and a caller's real target dir never gets stub files.
TT="$T/targetdir"; mkdir -p "$TT/outer"
make_pkg "$TT/pkg" tdapp 0.1.1 tdapp
plan=$(make_plan "$TT/pkg" BINARIES=tdapp KIND=cli)
for mode in unset set; do
  rc=0
  mkdir -p "$T/cargo-target"; : > "$T/cargo-target/stale"   # run_linux must clear the throw-away target dir per run
  if [[ $mode == set ]]; then
    out=$(CARGO_TARGET_DIR="$T/cargo-target" run_linux "$TT/pkg" "$plan" tar.gz 2>&1) || rc=$?
  else
    out=$(unset CARGO_TARGET_DIR; run_linux "$TT/pkg" "$plan" tar.gz 2>&1) || rc=$?
  fi
  [[ $rc -eq 0 ]] && pass "target_dir: package-linux finds the stub binaries (CARGO_TARGET_DIR $mode)" \
    || fail "target_dir: package-linux cannot find the stub binaries (CARGO_TARGET_DIR $mode, rc=$rc)"
  check_in "target_dir: tarball built (CARGO_TARGET_DIR $mode)" "$out" "tdapp-0.1.1-linux-x86_64.tar.gz"
  [[ ! -e "$T/cargo-target/stale" ]] && pass "target_dir: the target dir is cleared per run (CARGO_TARGET_DIR $mode)" \
    || fail "target_dir: a stale file survived in the target dir (CARGO_TARGET_DIR $mode)"
done
# the harness redirects an outer CARGO_TARGET_DIR into $T (checked in a child so the outer value is real)
outer="$TT/outer"
got=$(CARGO_TARGET_DIR="$outer" TEST_TMP="$TT/sub" bash -c 'source "$1/tests/packaging/harness.sh"; echo "$CARGO_TARGET_DIR"' _ "$ROOT" 2>&1) || true
check_in "target_dir: an outer CARGO_TARGET_DIR is redirected into the throw-away dir" "$got" "/sub/run."
[[ -z $(find "$outer" -type f) ]] && pass "target_dir: the outer target dir stays empty" || fail "target_dir: stub files landed in the outer target dir"
