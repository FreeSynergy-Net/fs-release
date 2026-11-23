#!/usr/bin/env bash
# Pattern: none (test).
# Self test of [GATE-BELEG] / `gates.sh --like-ci` with STUB cargo/rustc/tools. The local Beleg NEVER says "CI-gleich: ja" (always NEIN: the branch CI at the exact head incl. windows is the proof) and lists what ran / did NOT run.
# The cases that were red in CI while green locally each have a fixture and a mutant that makes the case green again: AW18 format_collect in a test file (hidden behind lib lints), assert-is-empty in tests (--all-targets),
# Windows cfg "unused self", shellcheck SC2164 (WP), rust-rules file length of a TEST file (T1), a self test of the CI lint job (test_affected_crates mutant), multi-line `run: |` blocks of lint.yml.
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
# stub cargo: `clippy` is red with the foreign lib lint map_unwrap_or unless allowed; BADTEST: also format_collect in tests/t.rs (a real CI lint in a changed file) unless allowed;
# BADASSERT: assert-is-empty in tests/forms.rs, only reported when `--all-targets` is on the command line (as clippy only lints test targets then); a call with --target (windows) fails with "unused self" when BADWIN
cat >"$tmp/bin/cargo" <<'S'
#!/usr/bin/env bash
[ "$1" = clippy ] || exit 0
rc=0
case "$*" in *"-A clippy::map_unwrap_or"*) ;; *) echo "error: map"; echo " --> crates/x/src/lib.rs:1:1"; echo " = help: https://x/index.html#map_unwrap_or"; rc=101 ;; esac
if [ -n "${BADTEST:-}" ]; then case "$*" in *"-A clippy::format_collect"*) ;; *) echo "error: fc"; echo " --> tests/t.rs:3:1"; echo " = help: https://x/index.html#format_collect"; rc=101 ;; esac; fi
if [ -n "${BADASSERT:-}" ]; then case "$*" in *--all-targets*) echo "error: assert-is-empty"; echo " --> tests/forms.rs:86:5"; rc=101 ;; esac; fi
if [ -n "${BADWIN:-}" ]; then case "$*" in *--target*) echo "error: unused self argument"; rc=101 ;; esac; fi
exit $rc
S
printf '#!/usr/bin/env bash\necho "rustc 9.9.9 (stub)"\n' >"$tmp/bin/rustc"; printf '#!/usr/bin/env bash\necho "rustc 1.0.0 (stub)"\n' >"$tmp/alt/rustc"
# the pinned-toolchain bin dir: cargo stub honours the same variables but never fails on the lib lint (CI toolchain has no map_unwrap_or)
sed 's/^case "\$\*" in \*"-A clippy::map_unwrap_or"\*).*$/:/' "$tmp/bin/cargo" >"$tmp/alt/cargo"
printf '#!/usr/bin/env bash\nexit 0\n' >"$tmp/alt/rustup"; printf '#!/usr/bin/env bash\necho shellcheck-stub\nexit 0\n' >"$tmp/alt/shellcheck"; printf '#!/usr/bin/env bash\necho 1.7.12\nexit 0\n' >"$tmp/alt/actionlint"
chmod +x "$tmp"/bin/* "$tmp"/alt/*
# windows target installed (stub sysroot) and the REAL shellcheck variants of the alt dir
mkdir -p "$tmp/sysroot/lib/rustlib/x86_64-pc-windows-msvc" "$tmp/alt-win" "$tmp/alt-sc"
cp "$tmp"/alt/* "$tmp/alt-win/"; printf '#!/usr/bin/env bash\n[ "$1" = --print ] && { echo "%s"; exit 0; }\necho "rustc 1.0.0 (stub)"\n' "$tmp/sysroot" >"$tmp/alt-win/rustc"
cp "$tmp"/alt/* "$tmp/alt-sc/"; rm -f "$tmp/alt-sc/shellcheck"
mkrepo() { rm -rf "$1"; mkdir -p "$1/crates/x/tests" "$1/crates/x/src" "$1/.github/workflows"; cp -r "$here" "$1/scripts"; rm -rf "$1/scripts/__pycache__"; echo 1.0.0 >"$1/RUST_VERSION"
  printf '__pycache__/\n.test-tmp/\n' >"$1/.gitignore"; git init -q -b main "$1"; git -C "$1" add -A; git -C "$1" commit -qm one; git -C "$1" update-ref refs/remotes/origin/main HEAD; git -C "$1" config --local freesynergy.identity-test-base HEAD; }
app="$tmp/app"; mkrepo "$app"
run() { # <script> <path dir> <args...>: output in $tmp/o.txt, the rc is returned
  local s="$1" p="$2"; shift 2
  PATH="$p:$PATH" CI=true FS_GATES_FEHLERLISTE_DIR="$tmp/l" bash "$s" "$@" >"$tmp/o.txt" 2>&1
}
G="$app/scripts/gates.sh"
commit() { git -C "$app" add -A; git -C "$app" commit -qm "$1"; }
gen_big() { { echo "//! Pattern: none (test)."; for i in $(seq 1 300); do echo "// line $i"; done; } >"$1"; }
# 0. "ja" does not exist: not in the code, never in the output
chk "no 'CI-gleich: ja' in the output code" bash -c "! grep -q 'CI-gleich: ja' '$here/gates.sh' '$here/lib_gates_list.sh'"
chk "the Beleg wording never prints the gates.sh narrowed line (test_affected_crates mutant)" bash -c "! grep -q 'test NARROWED' '$here/lib_gates_list.sh'"
# 1. foreign toolchain: NEIN + what did not run + counter-probe + head and time
run "$G" "$tmp/bin" --like-ci "$app"
chk "foreign toolchain: NEIN, the proof is the branch CI incl. windows, toolchain named as not CI-equal" bash -c "grep -q 'CI-gleich: NEIN - Zweig-CI am genauen Kopf (inkl. windows) ist der Beleg' '$tmp/o.txt' && grep -q 'lokal NICHT gelaufen: toolchain: Toolchain 1.0.0 nicht verfuegbar -> NICHT CI-gleich' '$tmp/o.txt' && grep -q 'lokal gelaufen: fmt=0' '$tmp/o.txt' && ! grep -q 'CI-gleich: ja' '$tmp/o.txt'"
chk "foreign toolchain: clippy counter-probe with -A of the lib lint" bash -c "grep -q 'clippy-gegenprobe rc=0 with -A clippy::map_unwrap_or' '$tmp/o.txt'"
chk "Beleg carries the full head and a time stamp, every line the gates-beleg prefix" bash -c "grep -q \"head=$(git -C "$app" rev-parse HEAD); time=20\" '$tmp/o.txt' && ! grep -q '^lokal NICHT' '$tmp/o.txt'"
# 2. AW18: a REAL lint (format_collect) in a CHANGED test file next to the foreign lib lint must not be allowed away
mkdir -p "$app/tests"; printf '//! Pattern: none (test).\n' >"$app/tests/t.rs"; commit t
BADTEST=1 run "$G" "$tmp/bin" "$app" --like-ci
chk "AW18 rebuild: only map_unwrap_or allowed, format_collect reported in tests/t.rs as CHANGED" bash -c "grep -q 'with -A clippy::map_unwrap_or ' '$tmp/o.txt' && ! grep -q -- '-A clippy::format_collect' '$tmp/o.txt' && grep -q 'ROT/gefunden' '$tmp/o.txt' && grep -q 'CHANGED file: tests/t.rs' '$tmp/o.txt' && grep -q 'not allowed: clippy::format_collect' '$tmp/o.txt'"
git -C "$app" update-ref -d refs/remotes/origin/main
BADTEST=1 run "$G" "$tmp/bin" "$app" --like-ci
chk "no origin/main: loud in the not-run list and in the clippy line, no file list" bash -c "grep -c 'origin/main fehlt' '$tmp/o.txt' | grep -q '^2$' && ! grep -q 'CHANGED file\|clippy-gegenprobe' '$tmp/o.txt'"
git -C "$app" update-ref refs/remotes/origin/main "$(git -C "$app" rev-list --max-parents=0 HEAD)"
# 3. assert-is-empty in a test target (SQ-2, SQ-1b): only --all-targets sees it
BADASSERT=1 run "$G" "$tmp/alt" --like-ci "$app"; rc=$?
chk "assert-is-empty in tests: clippy rc 101, run red, 'gates ok' absent" bash -c "[ $rc -eq 1 ] && grep -q '^gate clippy: rc=101' '$tmp/o.txt' && grep -q 'tests/forms.rs:86' '$tmp/o.txt' && ! grep -q '^gates ok' '$tmp/o.txt'"
# 4. pinned toolchain + all tools: still NEIN (never ja); lists lint steps as run; flag before or after the path; standard mode lists the lint job as not run
for pos in "--like-ci $app" "$app --like-ci"; do
  run "$G" "$tmp/alt" $pos; rc=$?
  chk "'$pos': rc 0, NEIN, lint steps ran, never ja" bash -c "[ $rc -eq 0 ] && grep -q 'CI-gleich: NEIN' '$tmp/o.txt' && ! grep -q 'CI-gleich: ja' '$tmp/o.txt' && grep -q '^gate shellcheck: rc=0' '$tmp/o.txt' && grep -q '^gate actionlint: rc=0' '$tmp/o.txt'"
done
run "$G" "$tmp/alt" "$app"
chk "standard mode: lint job and self tests listed as NOT run" bash -c "grep -q 'lokal NICHT gelaufen: mode standard' '$tmp/o.txt' && ! grep -q '^gate shellcheck:' '$tmp/o.txt'"
run "$G" "$tmp/alt" "$app" --bogus; rc=$?
chk "unknown option: rc 2, nothing runs" bash -c "[ $rc -eq 2 ] && ! grep -q '^gate ' '$tmp/o.txt'"
mkdir -p "$tmp/alt2"; cp "$tmp"/alt/* "$tmp/alt2/"; printf '#!/usr/bin/env bash\necho 1.6.0\n' >"$tmp/alt2/actionlint"
run "$G" "$tmp/alt2" --like-ci "$app"
chk "other actionlint version listed" bash -c "grep -q 'actionlint version differs from the CI pin' '$tmp/o.txt'"
FS_GATES_SKIP_TEST=released_changelog_sections_equal_their_tags run "$G" "$tmp/alt" --like-ci "$app"
chk "a skipped test is listed as not run" bash -c "grep -q 'SKIPPED (FS_GATES_SKIP_TEST)' '$tmp/o.txt'"
# 5. self tests of the CI lint job: single-line run, multi-line `run: |` block (all allowed lines run), other blocks listed as NOT run
printf 'jobs:\n  a:\n    steps:\n      - name: single\n        run: bash scripts/test_fail.sh\n' >"$app/.github/workflows/lint.yml"; printf '#!/usr/bin/env bash\necho boom\nexit 1\n' >"$app/scripts/test_fail.sh"; commit lintyml
run "$G" "$tmp/alt" --like-ci "$app"; rc=$?
chk "failing single-line CI self test: red, rc printed, no 'gates ok'" bash -c "[ $rc -eq 1 ] && grep -q '^selftest rc=1: bash scripts/test_fail.sh' '$tmp/o.txt' && grep -q '^gate selftests: rc=1' '$tmp/o.txt' && ! grep -q '^gates ok' '$tmp/o.txt'"
printf 'jobs:\n  a:\n    steps:\n      - name: block\n        run: |\n          bash scripts/test_ok.sh\n          # a comment\n          bash scripts/test_fail.sh\n      - name: curl step\n        run: |\n          curl -fsSL https://example.invalid | bash\n          echo x\n' >"$app/.github/workflows/lint.yml"; printf '#!/usr/bin/env bash\nexit 0\n' >"$app/scripts/test_ok.sh"; commit block
run "$G" "$tmp/alt" --like-ci "$app"; rc=$?
chk "multi-line block: its lines run (a red one is red) and the other block is listed as NOT run, never skipped silently" bash -c "[ $rc -eq 1 ] && grep -q '^selftest rc=0: bash scripts/test_ok.sh' '$tmp/o.txt' && grep -q '^selftest rc=1: bash scripts/test_fail.sh' '$tmp/o.txt' && grep -q 'lokal NICHT gelaufen: curl step (mehrzeilig)' '$tmp/o.txt'"
printf '#!/usr/bin/env bash\nexit 0\n' >"$app/scripts/test_fail.sh"; commit lintyml-ok
run "$G" "$tmp/alt" --like-ci "$app"; rc=$?
chk "all CI self tests green: rc 0, counted" bash -c "[ $rc -eq 0 ] && grep -q 'selftests run: 2' '$tmp/o.txt'"
git -C "$app" rm -q -f .github/workflows/lint.yml; commit nolint
# 6. Windows (PKG-2): no target = listed as NOT run; target installed = clippy for it runs and is red when red; never ja
printf '//! Pattern: none (test).\n#[cfg(windows)]\nfn w() {}\n' >"$app/crates/x/src/win.rs"; commit win
run "$G" "$tmp/alt" --like-ci "$app"
chk "windows, no target: 'Windows job ... Zweig-CI Pflicht (windows)' not run, no clippy-windows step" bash -c "grep -q 'lokal NICHT gelaufen: Windows job' '$tmp/o.txt' && grep -q 'Zweig-CI Pflicht (windows)' '$tmp/o.txt' && ! grep -q '^gate clippy-windows' '$tmp/o.txt' && ! grep -q 'CI-gleich: ja' '$tmp/o.txt'"
run "$G" "$tmp/alt-win" --like-ci "$app"; rc=$?
chk "windows, target installed: clippy-windows runs (rc 0) but the Windows TEST job is still listed as not run" bash -c "[ $rc -eq 0 ] && grep -q '^gate clippy-windows: rc=0' '$tmp/o.txt' && grep -q 'cargo test, cfg!(windows)' '$tmp/o.txt' && grep -q 'CI-gleich: NEIN' '$tmp/o.txt'"
BADWIN=1 run "$G" "$tmp/alt-win" --like-ci "$app"; rc=$?
chk "windows clippy red (unused self): run red, rc 1, no gates ok" bash -c "[ $rc -eq 1 ] && grep -q '^gate clippy-windows: rc=101' '$tmp/o.txt' && ! grep -q '^gates ok' '$tmp/o.txt'"
git -C "$app" rm -q -f crates/x/src/win.rs; commit unwin
# 7. WP: shellcheck SC2164 in a tracked script (real shellcheck)
if command -v shellcheck >/dev/null 2>&1; then
  mkdir -p "$app/tests/packaging"; printf '#!/usr/bin/env bash\ncd "$1"\necho x\n' >"$app/tests/packaging/rules.sh"; commit sc2164
  run "$G" "$tmp/alt-sc" --like-ci "$app"; rc=$?
  chk "SC2164 (WP): shellcheck step rc 1, run red" bash -c "[ $rc -eq 1 ] && grep -q '^gate shellcheck: rc=1' '$tmp/o.txt' && grep -q 'SC2164' '$tmp/o.txt'"
  git -C "$app" rm -q -f tests/packaging/rules.sh; commit nosc
else echo "skip SC2164 case: shellcheck not installed"; fi
# 8. T1: a 301-line TEST file is red in rust-rules, never gates ok
gen_big "$app/crates/x/tests/big.rs"; commit big
run "$G" "$tmp/alt" --like-ci "$app"; rc=$?
chk "301-line test file: red (rc 1), rust-rules rc 1, no 'gates ok'" bash -c "[ $rc -eq 1 ] && grep -q '^gate rust-rules: rc=1' '$tmp/o.txt' && grep -q 'big.rs: 30[0-9] lines' '$tmp/o.txt' && ! grep -q '^gates ok' '$tmp/o.txt'"
git -C "$app" rm -q -f crates/x/tests/big.rs; commit unbig
# 9. mutants (bash only): each one makes a case above green again, and must therefore be caught by its check command
mut() { # <name> <file> <old> <new> <check command that must FAIL on the mutant>; env: MPATH (bin dir), MENV (VAR=1), MARGS (gates args), MFIX (shell snippet run in the mutant repo before)
  local name="$1" f="$2" old="$3" new="$4" m="$tmp/m" s; shift 4
  mkrepo "$m"; s="$(<"$m/scripts/$f")"
  if [[ "$s" != *"$old"* ]]; then bad "mutant $name: NOT APPLIED"; return; fi
  printf '%s\n' "${s/"$old"/"$new"}" >"$m/scripts/$f"; mkdir -p "$m/tests"; printf '//! Pattern: none (test).\n' >"$m/tests/t.rs"
  [ -z "${MFIX:-}" ] || (cd "$m" && eval "$MFIX")
  git -C "$m" add -A; git -C "$m" commit -qm mut
  env ${MENV:-X=1} PATH="${MPATH:-$tmp/alt}:$PATH" CI=true FS_GATES_FEHLERLISTE_DIR="$tmp/l" bash "$m/scripts/gates.sh" ${MARGS:---like-ci} "$m" >"$tmp/om.txt" 2>&1
  if bash -c "$*"; then bad "mutant $name survived"; else ok "mutant $name is caught"; fi
}
export tmp
mut "a 'ja' is printed" lib_gates_list.sh 'CI-gleich: NEIN - Zweig-CI' 'CI-gleich: ja - Zweig-CI' 'grep -q "CI-gleich: NEIN" "$tmp/om.txt"'
mut "Beleg call removed" gates.sh 'gates_beleg "$root"' ': "$root"' 'grep -q "gates-beleg: CI-gleich" "$tmp/om.txt"'
mut "head left out of the Beleg" lib_gates_list.sh 'head=$(git -C "$root" rev-parse HEAD 2>/dev/null || echo unknown)' 'head=none' 'grep -q "head=[0-9a-f]\{40\}" "$tmp/om.txt"'
mut "narrowed line collides again" lib_gates_list.sh 'test scope narrowed' 'test NARROWED' '! grep -q "test NARROWED" "$tmp/m/scripts/lib_gates_list.sh"'
mut "windows not-run line removed" lib_gates_list.sh 'Windows job (native fmt/clippy/TEST on Windows; no windows target' 'W (native fmt/clippy/TEST on Windows; no windows target' 'grep -q "lokal NICHT gelaufen: Windows job" "$tmp/om.txt"'
MFIX='printf "//! Pattern: none (test).\n#[cfg(windows)]\nfn w() {}\n" >crates/x/src/w.rs' MPATH="$tmp/alt-win" MENV=BADWIN=1 mut "windows clippy step removed" gates.sh 'step clippy-windows lint' ': clippy-windows lint' 'grep -q "^gate clippy-windows: rc=101" "$tmp/om.txt"'
MPATH="$tmp/bin" MENV=BADTEST=1 mut "probe allows every lint" lib_gates_list.sh 'if [ -z "${bad[$l]:-}" ]; then lints' 'if true; then lints' 'grep -q "CHANGED file: tests/t.rs" "$tmp/om.txt"'
MPATH="$tmp/bin" MENV=BADTEST=1 mut "unmarked changed files" lib_gates_list.sh 'then echo "gates-beleg:   CHANGED file: $f"' 'then echo "gates-beleg:   $f"' 'grep -q "CHANGED file: tests/t.rs" "$tmp/om.txt"'
MENV=BADASSERT=1 mut "clippy without --all-targets (assert-is-empty)" gates.sh 'step clippy lint cargo clippy --workspace --all-targets' 'step clippy lint cargo clippy --workspace' 'grep -q "^gate clippy: rc=101" "$tmp/om.txt"'
MFIX='gen_big() { echo; }; { echo "//! Pattern: none (test)."; for i in $(seq 1 300); do echo "// line $i"; done; } >crates/x/tests/big.rs' mut "rust-rules step removed (301-line test file)" gates.sh 'step rust-rules rule' ': rust-rules rule' 'grep -q "^gate rust-rules: rc=1" "$tmp/om.txt"'
MFIX='mkdir -p tests/packaging; printf "#!/usr/bin/env bash\ncd \"\$1\"\n" >tests/packaging/rules.sh' MPATH="$tmp/alt-sc" mut "shellcheck step removed (SC2164)" gates.sh 'step shellcheck lint' ': shellcheck lint' 'grep -q "^gate shellcheck: rc=1" "$tmp/om.txt"'
MFIX='printf "jobs:\n  a:\n    steps:\n      - name: b\n        run: |\n          bash scripts/test_x.sh\n" >.github/workflows/lint.yml; printf "#!/usr/bin/env bash\nexit 1\n" >scripts/test_x.sh' mut "multi-line block handling removed" lib_gates_list.sh '/^ *run: *[|>]/ { blk = 1;' '/^ *run: *[|>]/ { blk = 0;' 'grep -q "^selftest rc=1: bash scripts/test_x.sh" "$tmp/om.txt"'
MFIX='printf "jobs:\n  a:\n    steps:\n      - name: b\n        run: bash scripts/test_x.sh\n" >.github/workflows/lint.yml; printf "#!/usr/bin/env bash\nexit 1\n" >scripts/test_x.sh' mut "selftests step removed" gates.sh 'step selftests error gates_lint_selftests' ': selftests' 'grep -q "^selftest rc=1" "$tmp/om.txt"'
if [ "$fails" -ne 0 ]; then echo; echo "test_gates_like_ci: $fails failures"; exit 1; fi
echo; echo "test_gates_like_ci: all ok"
