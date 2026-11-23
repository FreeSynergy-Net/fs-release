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
# stub cargo: `clippy` is red with the lib lint map_unwrap_or (foreign toolchain) unless allowed; with BADTEST it ALSO reports format_collect in tests/t.rs (a real CI lint in a changed file) unless allowed
cat >"$tmp/bin/cargo" <<'S'
#!/usr/bin/env bash
[ "$1" = clippy ] || exit 0
rc=0
case "$*" in *"-A clippy::map_unwrap_or"*) ;; *) echo "error: map"; echo " --> crates/x/src/lib.rs:1:1"; echo " = help: https://x/index.html#map_unwrap_or"; rc=101 ;; esac
if [ -n "${BADTEST:-}" ]; then case "$*" in *"-A clippy::format_collect"*) ;; *) echo "error: fc"; echo " --> tests/t.rs:3:1"; echo " = help: https://x/index.html#format_collect"; rc=101 ;; esac; fi
exit $rc
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
chk "foreign toolchain: NEIN with toolchain reason and the CI-proof rule" bash -c "grep -q 'CI-gleich: NEIN' '$tmp/o.txt' && grep -q 'Toolchain 1.0.0 nicht verfuegbar -> NICHT CI-gleich (local .rustc 9.9.9' '$tmp/o.txt' && grep -q 'branch CI at the head is the proof' '$tmp/o.txt'"
chk "foreign toolchain: clippy counter-probe with -A of the lib lint, green = test targets checked" bash -c "grep -q 'clippy-gegenprobe rc=0 with -A clippy::map_unwrap_or' '$tmp/o.txt'"
chk "Beleg carries the full head and a time stamp" bash -c "grep -q \"head=$(git -C "$app" rev-parse HEAD); time=20\" '$tmp/o.txt'"
# 2. a REAL lint (format_collect) in a CHANGED test file next to the foreign lib lint (AW18): it must not be allowed away; the probe names it and the file as CHANGED
mkdir -p "$app/tests"; printf '//! Pattern: none (test).\n' >"$app/tests/t.rs"; git -C "$app" add -A; git -C "$app" commit -qm t
BADTEST=1 run "$G" "$tmp/bin" "$app" --like-ci
chk "AW18 rebuild: only map_unwrap_or is allowed, format_collect is reported in tests/t.rs as CHANGED" bash -c "grep -q 'with -A clippy::map_unwrap_or ' '$tmp/o.txt' && ! grep -q -- '-A clippy::format_collect' '$tmp/o.txt' && grep -q 'ROT/gefunden' '$tmp/o.txt' && grep -q 'CHANGED file: tests/t.rs' '$tmp/o.txt' && grep -q 'not allowed: clippy::format_collect' '$tmp/o.txt'"
# 2b. no origin/main: loud, NEIN, no file list without marks
git -C "$app" update-ref -d refs/remotes/origin/main
BADTEST=1 run "$G" "$tmp/bin" "$app" --like-ci
chk "no origin/main: NICHT GEPRUEFT in the reasons and in the clippy line, no file list" bash -c "grep -c 'NICHT GEPRUEFT: origin/main fehlt' '$tmp/o.txt' | grep -q '^2$' && ! grep -q 'CHANGED file\|clippy-gegenprobe' '$tmp/o.txt'"
git -C "$app" update-ref refs/remotes/origin/main "$(git -C "$app" rev-list --max-parents=0 HEAD)"
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
# 3b. the self tests of the CI jobs (lint.yml `run: bash scripts/test_*.sh`) run in --like-ci: a failing one is red (CI-red-while-local-green: test_affected_crates)
printf 'jobs:\n  a:\n    steps:\n      - name: x\n        run: bash scripts/test_fail.sh\n' >"$app/.github/workflows/lint.yml"; printf '#!/usr/bin/env bash\necho boom\nexit 1\n' >"$app/scripts/test_fail.sh"
git -C "$app" add -A; git -C "$app" commit -qm lintyml
run "$G" "$tmp/alt" --like-ci "$app"; rc=$?
chk "failing CI self test: red (rc 1), its rc is printed, no 'gates ok'" bash -c "[ $rc -eq 1 ] && grep -q '^selftest rc=1: bash scripts/test_fail.sh' '$tmp/o.txt' && grep -q '^gate selftests: rc=1' '$tmp/o.txt' && ! grep -q '^gates ok' '$tmp/o.txt'"
printf '#!/usr/bin/env bash\nexit 0\n' >"$app/scripts/test_fail.sh"; git -C "$app" add -A; git -C "$app" commit -qm lintyml-ok
run "$G" "$tmp/alt" --like-ci "$app"; rc=$?
chk "passing CI self test: rc 0 and counted" bash -c "[ $rc -eq 0 ] && grep -q 'selftests run: 1' '$tmp/o.txt'"
# 3c. Windows cfg code (PKG-2): no windows target = NICHT GEPRUEFT + NEIN, never ja; target installed = clippy for it runs, red when it is red
mkdir -p "$tmp/sysroot/lib/rustlib/x86_64-pc-windows-msvc"; cat >"$tmp/alt/rustc" <<S
#!/usr/bin/env bash
[ "\$1" = --print ] && { echo "$tmp/sysroot-none"; exit 0; }
echo "rustc 1.0.0 (stub)"
S
mkdir -p "$tmp/alt-win"; cp "$tmp"/alt/* "$tmp/alt-win/"; sed -i "s|sysroot-none|sysroot|" "$tmp/alt-win/rustc"
printf '//! Pattern: none (test).\n#[cfg(windows)]\nfn w() {}\n' >"$app/crates/x/src/win.rs"; git -C "$app" add -A; git -C "$app" commit -qm win
run "$G" "$tmp/alt" --like-ci "$app"; rc=$?
chk "windows cfg, no target: NICHT GEPRUEFT + NEIN (never ja), no clippy-windows step" bash -c "grep -q 'NICHT GEPRUEFT: Windows-cfg nicht lokal pruefbar' '$tmp/o.txt' && grep -q 'Zweig-CI Pflicht (windows)' '$tmp/o.txt' && grep -q 'CI-gleich: NEIN' '$tmp/o.txt' && ! grep -q 'CI-gleich: ja' '$tmp/o.txt' && ! grep -q '^gate clippy-windows' '$tmp/o.txt'"
run "$G" "$tmp/alt-win" --like-ci "$app"; rc=$?
chk "windows cfg, target installed: clippy-windows runs (rc 0) and ja is possible" bash -c "[ $rc -eq 0 ] && grep -q '^gate clippy-windows: rc=0' '$tmp/o.txt' && grep -q 'CI-gleich: ja' '$tmp/o.txt'"
printf '#!/usr/bin/env bash\n[ "$1" = clippy ] && case "$*" in *--target*) echo "error: unused self argument"; exit 101;; esac\nexit 0\n' >"$tmp/alt-win/cargo"; chmod +x "$tmp/alt-win/cargo"
run "$G" "$tmp/alt-win" --like-ci "$app"; rc=$?
chk "windows clippy red (unused self): run red, rc 1, no gates ok" bash -c "[ $rc -eq 1 ] && grep -q '^gate clippy-windows: rc=101' '$tmp/o.txt' && ! grep -q '^gates ok' '$tmp/o.txt'"
git -C "$app" rm -q -f crates/x/src/win.rs; git -C "$app" commit -qm unwin
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
  printf '%s\n' "${s/"$old"/"$new"}" >"$m/scripts/$f"; mkdir -p "$m/tests"; printf '//! Pattern: none (test).\n' >"$m/tests/t.rs"; git -C "$m" add -A; git -C "$m" commit -qm mut
  env ${MENV:-X=1} PATH="${MPATH:-$tmp/alt}:$PATH" CI=true FS_GATES_FEHLERLISTE_DIR="$tmp/l" bash "$m/scripts/gates.sh" ${MARGS:-} "$m" >"$tmp/om.txt" 2>&1  # default: standard mode, pinned toolchain: must say NEIN
  if bash -c "$*"; then bad "mutant $name survived"; else ok "mutant $name is caught"; fi
}
export tmp
mut "standard mode counted as ja" lib_gates_list.sh '[ -n "$4" ] || echo "mode standard' '[ -n "$4" ] || : "mode standard' 'grep -q "CI-gleich: NEIN" "$tmp/om.txt"'
mut "Beleg call removed" gates.sh 'gates_beleg "$root"' ': "$root"' 'grep -q "gates-beleg: CI-gleich" "$tmp/om.txt"'
mut "head left out of the Beleg" lib_gates_list.sh 'head=$(git -C "$root" rev-parse HEAD 2>/dev/null || echo unknown)' 'head=none' 'grep -q "head=[0-9a-f]\{40\}" "$tmp/om.txt"'
MPATH="$tmp/bin" MENV=BADTEST=1 MARGS=--like-ci mut "probe allows every lint" lib_gates_list.sh 'if [ -z "${bad[$l]:-}" ]; then lints' 'if true; then lints' 'grep -q "CHANGED file: tests/t.rs" "$tmp/om.txt"'
MPATH="$tmp/bin" MENV=BADTEST=1 MARGS=--like-ci mut "unmarked changed files" lib_gates_list.sh 'then echo "gates-beleg:   CHANGED file: $f"' 'then echo "gates-beleg:   $f"' 'grep -q "CHANGED file: tests/t.rs" "$tmp/om.txt"'
mut "selftests step removed" gates.sh 'step selftests error gates_lint_selftests' ': selftests' 'printf "jobs:\\n  a:\\n    steps:\\n      - name: x\\n        run: bash scripts/test_x.sh\\n" >"$tmp/m/.github/workflows/lint.yml"; printf "#!/usr/bin/env bash\\nexit 1\\n" >"$tmp/m/scripts/test_x.sh"; git -C "$tmp/m" add -A; git -C "$tmp/m" commit -qm x; env PATH="$tmp/alt:$PATH" CI=true FS_GATES_FEHLERLISTE_DIR="$tmp/l" bash "$tmp/m/scripts/gates.sh" --like-ci "$tmp/m" 2>&1 | grep -q "selftest rc=1"'
mut "windows reason removed" lib_gates_list.sh 'echo "NICHT GEPRUEFT: Windows-cfg' ': "Windows-cfg' 'printf "//! Pattern: none (test).\\n#[cfg(windows)]\\nfn w() {}\\n" >"$tmp/m/crates/x/src/w.rs"; git -C "$tmp/m" add -A; git -C "$tmp/m" commit -qm w; env PATH="$tmp/alt:$PATH" CI=true FS_GATES_FEHLERLISTE_DIR="$tmp/l" bash "$tmp/m/scripts/gates.sh" --like-ci "$tmp/m" 2>&1 | grep -q "Windows-cfg nicht lokal"'
mut "windows clippy step removed" gates.sh 'step clippy-windows lint' ': clippy-windows lint' 'printf "//! Pattern: none (test).\\n#[cfg(windows)]\\nfn w() {}\\n" >"$tmp/m/crates/x/src/w.rs"; git -C "$tmp/m" add -A; git -C "$tmp/m" commit -qm w; env PATH="$tmp/alt-win:$PATH" CI=true FS_GATES_FEHLERLISTE_DIR="$tmp/l" bash "$tmp/m/scripts/gates.sh" --like-ci "$tmp/m" 2>&1 | grep -q "^gate clippy-windows"'
mut "origin/main reason removed" lib_gates_list.sh 'echo "NICHT GEPRUEFT: origin/main fehlt (changed' ': "(changed' 'git -C "$tmp/m" update-ref -d refs/remotes/origin/main; env PATH="$tmp/alt:$PATH" CI=true FS_GATES_FEHLERLISTE_DIR="$tmp/l" bash "$tmp/m/scripts/gates.sh" --like-ci "$tmp/m" 2>&1 | grep -q "origin/main fehlt"'
if [ "$fails" -ne 0 ]; then echo; echo "test_gates_like_ci: $fails failures"; exit 1; fi
echo; echo "test_gates_like_ci: all ok"
