#!/usr/bin/env bash
# Pattern: none (test).
# Self test of [GATE-BELEG] / `gates.sh --like-ci` with STUB cargo/rustc/tools: "CI-gleich: ja" ONLY with the pinned toolchain, all tools AND --like-ci (flag anywhere); a 301-line TEST file is red;
# a foreign toolchain gets the clippy counter-probe (-A of the lib lints, so test targets are checked); the Beleg carries head + time; mutants turn a case red.
#   bash scripts/test_gates_like_ci.sh
set -uo pipefail
here="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
mkdir -p "$here/../.test-tmp"; tmp="$(TMPDIR="${TEST_TMPDIR:-$here/../.test-tmp}" mktemp -d)"  # never /tmp (RAM)
trap '[ -n "${KEEP:-}" ] || { rm -rf "$tmp"; rmdir "$here/../.test-tmp" 2>/dev/null; }' EXIT
fails=0
ok() { echo "ok   $1"; }
bad() { echo "FAIL $1"; fails=$((fails + 1)); }
chk() { local name="$1"; shift; if "$@"; then ok "$name"; else bad "$name"; fi; }
export GIT_AUTHOR_NAME=t GIT_AUTHOR_EMAIL=t@t GIT_COMMITTER_NAME=t GIT_COMMITTER_EMAIL=t@t PYTHONDONTWRITEBYTECODE=1
unset CI FS_GATES_SKIP_TEST FS_GATES_DIFF_BASE
export TMPDIR_GATES="$tmp"
mkdir -p "$tmp/bin" "$tmp/alt"
# stub cargo: `clippy` is red (lib lint map_unwrap_or) unless that lint is allowed; with BADTEST it then reports a finding in tests/t.rs
cat >"$tmp/bin/cargo" <<'S'
#!/usr/bin/env bash
[ "$1" = clippy ] || exit 0
case "$*" in
  *"-A clippy::map_unwrap_or"*) [ -n "${BADTEST:-}" ] && { echo "error: format_collect"; echo " --> tests/t.rs:3:1"; exit 101; }; exit 0 ;;
  *) echo "error: map"; echo " --> crates/x/src/lib.rs:1:1"; echo " = help: https://rust-lang.github.io/rust-clippy/master/index.html#map_unwrap_or"; exit 101 ;;
esac
S
printf '#!/usr/bin/env bash\necho "rustc 9.9.9 (stub)"\n' >"$tmp/bin/rustc"; printf '#!/usr/bin/env bash\necho "rustc 1.0.0 (stub)"\n' >"$tmp/alt/rustc"
printf '#!/usr/bin/env bash\necho "cargo-ok"\n' >"$tmp/alt/cargo"
printf '#!/usr/bin/env bash\nexit 0\n' >"$tmp/alt/rustup"; printf '#!/usr/bin/env bash\necho shellcheck-stub\nexit 0\n' >"$tmp/alt/shellcheck"; printf '#!/usr/bin/env bash\necho 1.7.12\nexit 0\n' >"$tmp/alt/actionlint"
chmod +x "$tmp"/bin/* "$tmp"/alt/*
mkrepo() { rm -rf "$1"; mkdir -p "$1/crates/x/tests" "$1/crates/x/src" "$1/.github/workflows"; cp -r "$here" "$1/scripts"; rm -rf "$1/scripts/__pycache__"; echo 1.0.0 >"$1/RUST_VERSION"
  printf '__pycache__/\n.test-tmp/\n' >"$1/.gitignore"; git init -q -b main "$1"; git -C "$1" add -A; git -C "$1" commit -qm one; git -C "$1" update-ref refs/remotes/origin/main HEAD; }
app="$tmp/app"; mkrepo "$app"
run() { # <script> <path dir> <args...>: output in $tmp/o.txt, the rc is returned
  local s="$1" p="$2"; shift 2
  PATH="$p:$PATH" CI=true FS_GATES_FEHLERLISTE_DIR="$tmp/l" bash "$s" "$@" >"$tmp/o.txt" 2>&1
}
G="$app/scripts/gates.sh"
# 1. foreign toolchain: rc follows the steps (clippy red), NEIN with reason, counter-probe runs, head + time in the Beleg
run "$G" "$tmp/bin" --like-ci "$app"
chk "foreign toolchain: NEIN with toolchain reason and the CI-proof rule" bash -c "grep -q 'CI-gleich: NEIN' '$tmp/o.txt' && grep -q 'toolchain: local .rustc 9.9.9' '$tmp/o.txt' && grep -q 'branch CI at the head is the proof' '$tmp/o.txt'"
chk "foreign toolchain: clippy counter-probe with -A of the lib lint, green = test targets checked" bash -c "grep -q 'clippy-gegenprobe rc=0 with -A clippy::map_unwrap_or' '$tmp/o.txt'"
chk "Beleg carries the full head and a time stamp" bash -c "grep -q \"head=$(git -C "$app" rev-parse HEAD); time=20\" '$tmp/o.txt'"
# 2. a finding in a test file hidden behind the lib error: the probe names it as CHANGED
mkdir -p "$app/tests"; printf '//! Pattern: none (test).\n' >"$app/tests/t.rs"; git -C "$app" add -A; git -C "$app" commit -qm t
BADTEST=1 run "$G" "$tmp/bin" "$app" --like-ci
chk "hidden test-target finding (AW18): probe RED and names tests/t.rs as CHANGED" bash -c "grep -q 'clippy-gegenprobe RED' '$tmp/o.txt' && grep -q 'CHANGED file: tests/t.rs' '$tmp/o.txt'"
# 3. pinned toolchain + all tools: ja only with --like-ci (flag before OR after the path); standard mode is NEIN
for pos in "--like-ci $app" "$app --like-ci"; do
  run "$G" "$tmp/alt" $pos; rc=$?
  chk "ja with '$pos' (rc 0, lint steps ran)" bash -c "[ $rc -eq 0 ] && grep -q 'CI-gleich: ja' '$tmp/o.txt' && grep -q '^gate shellcheck: rc=0' '$tmp/o.txt' && grep -q '^gate actionlint: rc=0' '$tmp/o.txt'"
done
run "$G" "$tmp/alt" "$app"; rc=$?
chk "standard mode (no --like-ci): rc 0 but NEIN with 'mode standard', never ja" bash -c "[ $rc -eq 0 ] && grep -q 'CI-gleich: NEIN' '$tmp/o.txt' && grep -q 'mode standard' '$tmp/o.txt' && ! grep -q 'CI-gleich: ja' '$tmp/o.txt'"
run "$G" "$tmp/alt" "$app" --bogus; rc=$?
chk "unknown option: rc 2, nothing runs" bash -c "[ $rc -eq 2 ] && ! grep -q '^gate ' '$tmp/o.txt'"
mkdir -p "$tmp/alt2"; cp "$tmp"/alt/* "$tmp/alt2/"; printf '#!/usr/bin/env bash\necho 1.6.0\n' >"$tmp/alt2/actionlint"
run "$G" "$tmp/alt2" --like-ci "$app"
chk "other actionlint version: NEIN with the pin named" bash -c "grep -q 'CI-gleich: NEIN' '$tmp/o.txt' && grep -q 'actionlint version differs from the CI pin' '$tmp/o.txt'"
FS_GATES_SKIP_TEST=released_changelog_sections_equal_their_tags run "$G" "$tmp/alt" --like-ci "$app"
chk "a skipped test is a NEIN reason" bash -c "grep -q 'CI-gleich: NEIN' '$tmp/o.txt' && grep -q 'SKIPPED (FS_GATES_SKIP_TEST)' '$tmp/o.txt'"
# 4. a 301-line TEST file: rust-rules red, never gates ok
{ echo "//! Pattern: none (test)."; for i in $(seq 1 300); do echo "// line $i"; done; } >"$app/crates/x/tests/big.rs"
git -C "$app" add -A; git -C "$app" commit -qm big
run "$G" "$tmp/alt" --like-ci "$app"; rc=$?
chk "301-line test file: red (rc 1), rust-rules rc 1, no 'gates ok'" bash -c "[ $rc -eq 1 ] && grep -q '^gate rust-rules: rc=1' '$tmp/o.txt' && grep -q 'big.rs: 30[0-9] lines' '$tmp/o.txt' && ! grep -q '^gates ok' '$tmp/o.txt'"
git -C "$app" rm -q -f crates/x/tests/big.rs; git -C "$app" commit -qm unbig
# 5. mutants (bash only): each must turn a case red
mut() { # <name> <file> <old> <new> <check command that must FAIL on the mutant output>
  local name="$1" f="$2" old="$3" new="$4" m="$tmp/m" s; shift 4
  mkrepo "$m"; s="$(<"$m/scripts/$f")"
  if [[ "$s" != *"$old"* ]]; then bad "mutant $name: NOT APPLIED"; return; fi
  printf '%s\n' "${s/"$old"/"$new"}" >"$m/scripts/$f"; git -C "$m" add -A; git -C "$m" commit -qm mut
  PATH="$tmp/alt:$PATH" CI=true FS_GATES_FEHLERLISTE_DIR="$tmp/l" bash "$m/scripts/gates.sh" "$m" >"$tmp/om.txt" 2>&1  # standard mode, pinned toolchain: must say NEIN
  if bash -c "$*"; then bad "mutant $name survived"; else ok "mutant $name is caught"; fi
}
export tmp
mut "standard mode counted as ja" lib_gates_list.sh '[ -n "$4" ] || echo "mode standard' '[ -n "$4" ] || : "mode standard' 'grep -q "CI-gleich: NEIN" "$tmp/om.txt"'
mut "Beleg call removed" gates.sh 'gates_beleg "$root"' ': "$root"' 'grep -q "gates-beleg: CI-gleich" "$tmp/om.txt"'
mut "head left out of the Beleg" lib_gates_list.sh 'head=$(git -C "$root" rev-parse HEAD 2>/dev/null || echo unknown)' 'head=none' 'grep -q "head=[0-9a-f]\{40\}" "$tmp/om.txt"'
if [ "$fails" -ne 0 ]; then echo; echo "test_gates_like_ci: $fails failures"; exit 1; fi
echo; echo "test_gates_like_ci: all ok"
