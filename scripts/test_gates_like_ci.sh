#!/usr/bin/env bash
# Pattern: none (test).
# Self test of [GATE-BELEG] / `gates.sh --like-ci` with a STUB cargo and rustc: a 301-line TEST file is red in rust-rules (rule runs on tests too) and never "gates ok";
# a toolchain that is not the pinned one is marked "CI-gleich: NEIN" (never silent); all tools + the pinned toolchain = "CI-gleich: ja"; mutants of gates.sh / the lib turn a case red.
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
unset CI FS_GATES_SKIP_TEST
export TMPDIR_GATES="$tmp"
mkdir -p "$tmp/bin" "$tmp/alt"
printf '#!/usr/bin/env bash\nexit 0\n' >"$tmp/bin/cargo"
printf '#!/usr/bin/env bash\necho "rustc 9.9.9 (stub)"\n' >"$tmp/bin/rustc"
printf '#!/usr/bin/env bash\necho "rustc 1.0.0 (stub)"\n' >"$tmp/alt/rustc"
for t in shellcheck actionlint rustup; do printf '#!/usr/bin/env bash\nexit 0\n' >"$tmp/alt/$t"; done
cp "$tmp/bin/cargo" "$tmp/alt/cargo"; chmod +x "$tmp"/bin/* "$tmp"/alt/*
mkrepo() { rm -rf "$1"; mkdir -p "$1/crates/x/tests"; cp -r "$here" "$1/scripts"; rm -rf "$1/scripts/__pycache__"; echo 1.0.0 >"$1/RUST_VERSION"
  printf '__pycache__/\n.test-tmp/\n' >"$1/.gitignore"; git init -q -b main "$1"; git -C "$1" add -A; git -C "$1" commit -qm one; }
app="$tmp/app"; mkrepo "$app"
run() { # <script> <path dir> [--like-ci]: output in $tmp/o.txt, the rc is returned
  local s="$1" p="$2"; shift 2
  PATH="$p:$PATH" CI=true FS_GATES_FEHLERLISTE_DIR="$tmp/l" bash "$s" "$@" "$app" >"$tmp/o.txt" 2>&1
}
# 1. clean repo, foreign toolchain: green, but loudly NOT CI-equal with the reason and the rule
run "$app/scripts/gates.sh" "$tmp/bin" --like-ci; rc=$?
chk "foreign toolchain: still rc 0 (no finding)" [ "$rc" -eq 0 ]
chk "foreign toolchain: Beleg says CI-gleich: NEIN with the toolchain reason" bash -c "grep -q 'CI-gleich: NEIN' '$tmp/o.txt' && grep -q 'toolchain: local .rustc 9.9.9' '$tmp/o.txt' && grep -q 'branch CI at the head is the proof' '$tmp/o.txt'"
chk "foreign toolchain: Beleg lists the flags and every step with rc" bash -c "grep -q 'clippy=.*-D warnings -W clippy::pedantic' '$tmp/o.txt' && grep -q 'steps: fmt=0 clippy=0 test=0 rust-rules=0' '$tmp/o.txt'"
# 2. pinned toolchain + all tools: CI-gleich: ja, lint steps ran (like-ci)
run "$app/scripts/gates.sh" "$tmp/alt" --like-ci; rc=$?
chk "pinned toolchain + tools: rc 0 and CI-gleich: ja" bash -c "[ $rc -eq 0 ] && grep -q 'CI-gleich: ja' '$tmp/o.txt' && ! grep -q 'CI-gleich: NEIN' '$tmp/o.txt'"
chk "like-ci runs the lint steps (shellcheck, actionlint)" bash -c "grep -q '^gate shellcheck: rc=0' '$tmp/o.txt' && grep -q 'mode=like-ci' '$tmp/o.txt'"
run "$app/scripts/gates.sh" "$tmp/alt"
chk "standard mode does not run the lint steps" bash -c "! grep -q '^gate shellcheck:' '$tmp/o.txt' && grep -q 'mode=standard' '$tmp/o.txt'"
# 3. a 301-line TEST file: rust-rules red (rc 1), the run is red, never gates ok, not CI-equal says NEIN or the run fails
{ echo "//! Pattern: none (test)."; for i in $(seq 1 300); do echo "// line $i"; done; } >"$app/crates/x/tests/big.rs"
git -C "$app" add -A; git -C "$app" commit -qm big
run "$app/scripts/gates.sh" "$tmp/alt" --like-ci; rc=$?
chk "301-line test file: --like-ci is red (rc 1), rust-rules rc 1, no 'gates ok'" bash -c "[ $rc -eq 1 ] && grep -q '^gate rust-rules: rc=1' '$tmp/o.txt' && grep -q 'big.rs: 30[0-9] lines' '$tmp/o.txt' && ! grep -q '^gates ok' '$tmp/o.txt'"
git -C "$app" rm -q -f crates/x/tests/big.rs; git -C "$app" commit -qm unbig
# 4. mutants: each must turn a case red
mut() { # <name> <file in scripts> <python old> <python new> <case: a command that must FAIL on the mutant>
  local name="$1" f="$2" old="$3" new="$4"; shift 4
  local m="$tmp/m"; mkrepo "$m"; python3 -I - "$m/scripts/$f" "$old" "$new" <<'PY'
import sys
p, old, new = sys.argv[1:4]; s = open(p).read()
if old not in s: sys.exit(3)
open(p, "w").write(s.replace(old, new, 1))
PY
  if [ $? -eq 3 ]; then bad "mutant $name: NOT APPLIED"; return; fi
  git -C "$m" add -A; git -C "$m" commit -qm mut
  PATH="$tmp/bin:$PATH" CI=true FS_GATES_FEHLERLISTE_DIR="$tmp/l" bash "$m/scripts/gates.sh" --like-ci "$m" >"$tmp/om.txt" 2>&1
  if bash -c "$*"; then bad "mutant $name survived"; else ok "mutant $name is caught"; fi
}
export tmp
mut "toolchain compare removed" lib_gates_list.sh '*) echo "toolchain:' '*) : "toolchain:' 'grep -q "CI-gleich: NEIN" "$tmp/om.txt"'
mut "Beleg call removed" gates.sh 'gates_beleg "$root"' ': "$root"' 'grep -q "gates-beleg: CI-gleich" "$tmp/om.txt"'
mut "like-ci flag ignored" gates.sh 'if [ -n "$like_ci" ]; then
    if command -v shellcheck' 'if false; then
    if command -v shellcheck' 'grep -q "gate shellcheck: rc=" "$tmp/om.txt"'
if [ "$fails" -ne 0 ]; then echo; echo "test_gates_like_ci: $fails failures"; exit 1; fi
echo; echo "test_gates_like_ci: all ok"
