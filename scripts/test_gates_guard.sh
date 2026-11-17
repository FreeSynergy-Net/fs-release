#!/usr/bin/env bash
# Self test of the gates guard (rule [GATES-WAECHTER]: scripts/lib_gates_guard.sh + scripts/gates.sh). No cargo: gates.sh runs in test mode
# (GATES_ONLY_GUARD=1; GATES_GUARD_MIDDLE is a command that stands in for the long checks) on a throw-away git repo.
# Cases: unchanged tree = rc 0; file created / changed / deleted (tracked and untracked) in the middle = rc != 0 + fixed text + file name;
# a second parallel run = rc != 0 "gates läuft bereits" and the first run still ends ok; an orphaned lock (dead PID) is replaced; the
# lock is gone after the run; an ignored file (target/) may change. Mutants (compare removed, lock check removed, snapshot made blind)
# must turn the cases red.
#   bash scripts/test_gates_guard.sh
set -uo pipefail
here="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
tmp="$(mktemp -d)"
trap 'rm -rf "$tmp"' EXIT
fails=0
ok() { echo "ok   $1"; }
bad() { echo "FAIL $1"; fails=$((fails + 1)); }
TEXT="Arbeitsbaum während der Prüfung geändert"

# mkscripts <dir>: a copy of the scripts the guard needs (gates.sh, the library, RUST_VERSION) in <dir>/s/scripts
mkscripts() {
  mkdir -p "$1/s/scripts"
  cp "$here/gates.sh" "$here/lib_gates_guard.sh" "$1/s/scripts/"
  echo "1.0.0" > "$1/s/RUST_VERSION"
}
# mkrepo <dir>: a clean throw-away repo with two tracked files and an ignored target/
mkrepo() {
  mkdir -p "$1" && git -C "$1" init -q
  echo a > "$1/a.txt"; echo b > "$1/b.txt"; echo 'target/' > "$1/.gitignore"; mkdir -p "$1/target"
  git -C "$1" add -A && git -C "$1" -c user.name=t -c user.email=t@t commit -qm base
}
# run <scripts dir> <repo> <middle command>: gates.sh in test mode; output in $tmp/out, rc returned
run() {
  GATES_ONLY_GUARD=1 GATES_GUARD_MIDDLE="$3" CI=1 bash "$1/s/scripts/gates.sh" "$2" > "$tmp/out" 2>&1
}

mkscripts "$tmp/real"
R="$tmp/repo"; mkrepo "$R"

# 1 unchanged tree
run "$tmp/real" "$R" "true"; rc=$?
{ [ "$rc" -eq 0 ] && grep -q "gates guard ok" "$tmp/out"; } && ok "unchanged tree: rc 0" || bad "unchanged tree: rc $rc"
[ ! -e "$R/.gates.lock" ] && ok "lock removed after the run" || bad "lock left behind"
[ -z "$(git -C "$R" status --porcelain=v1 --untracked-files=all)" ] && ok "the guard leaves no file in the tree" || bad "guard left a file"

# 2 changes in the middle
check_change() { # <label> <middle> <expected file name>
  run "$tmp/real" "$R" "$2"; local rc=$?
  { [ "$rc" -ne 0 ] && grep -q "$TEXT" "$tmp/out" && grep -q "$3" "$tmp/out"; } && ok "$1: rc $rc, text and file named" || { bad "$1: rc $rc"; cat "$tmp/out"; }
  [ ! -e "$R/.gates.lock" ] && ok "$1: lock removed" || bad "$1: lock left behind"
  git -C "$R" checkout -q -- . && git -C "$R" clean -fdq
}
check_change "file created" "echo x > new.txt" "new.txt"
check_change "tracked file changed" "echo more >> a.txt" "a.txt"
check_change "tracked file deleted" "rm b.txt" "b.txt"
echo u > "$R/u.txt"
run "$tmp/real" "$R" "echo changed > u.txt"; rc=$?
{ [ "$rc" -ne 0 ] && grep -q "u.txt" "$tmp/out"; } && ok "untracked file changed: rc $rc" || bad "untracked file changed: rc $rc"
rm -f "$R/u.txt"
# the same content written again is no change; an ignored file may change
run "$tmp/real" "$R" "echo a > a.txt; echo junk > target/x.o"; rc=$?
[ "$rc" -eq 0 ] && ok "same content and ignored file: rc 0" || bad "same content / ignored file: rc $rc"

# 3 second parallel run
GATES_ONLY_GUARD=1 GATES_GUARD_MIDDLE="sleep 3" CI=1 bash "$tmp/real/s/scripts/gates.sh" "$R" > "$tmp/out1" 2>&1 &
first=$!
for _ in $(seq 1 50); do [ -e "$R/.gates.lock" ] && break; sleep 0.1; done
run "$tmp/real" "$R" "true"; rc=$?
{ [ "$rc" -ne 0 ] && grep -q "gates läuft bereits" "$tmp/out"; } && ok "second parallel run: rc $rc, gates läuft bereits" || bad "second parallel run: rc $rc"
wait "$first"; rc1=$?
{ [ "$rc1" -eq 0 ] && grep -q "gates guard ok" "$tmp/out1"; } && ok "the first run is not disturbed (rc 0)" || bad "first run rc $rc1"

# 4 orphaned lock
sleep 0.1 & dead=$!; wait "$dead"
echo "$dead" > "$R/.gates.lock"
run "$tmp/real" "$R" "true"; rc=$?
{ [ "$rc" -eq 0 ] && [ ! -e "$R/.gates.lock" ]; } && ok "orphaned lock (dead PID) replaced: rc 0" || bad "orphaned lock: rc $rc"
echo "garbage" > "$R/.gates.lock"
run "$tmp/real" "$R" "true"; rc=$?
[ "$rc" -eq 0 ] && ok "unreadable lock replaced: rc 0" || bad "unreadable lock: rc $rc"

# 5 mutants: each must turn a case red
mutant() { # <label> <sed expr on the library> <case: change|parallel>
  mkscripts "$tmp/m"; sed -i "$2" "$tmp/m/s/scripts/lib_gates_guard.sh"
  if cmp -s "$tmp/m/s/scripts/lib_gates_guard.sh" "$here/lib_gates_guard.sh"; then bad "mutant $1: the sed expression changed nothing"; return; fi
  if [ "$3" = change ] || [ "$3" = content ]; then
    # content: an untracked file that exists at the start gets other content (the status lines stay equal, only the hash shows it)
    echo u > "$R/u.txt"
    if [ "$3" = content ]; then run "$tmp/m" "$R" "echo changed > u.txt"; else run "$tmp/m" "$R" "echo x > new.txt"; fi
    rc=$?
    git -C "$R" clean -fdq
    [ "$rc" -eq 0 ] && ok "mutant $1: change in the middle goes unnoticed (test would be red)" || bad "mutant $1 not caught (rc $rc)"
  else
    GATES_ONLY_GUARD=1 GATES_GUARD_MIDDLE="sleep 2" CI=1 bash "$tmp/m/s/scripts/gates.sh" "$R" > "$tmp/out1" 2>&1 &
    local p=$!
    for _ in $(seq 1 50); do [ -e "$R/.gates.lock" ] && break; sleep 0.1; done
    run "$tmp/m" "$R" "true"; rc=$?
    wait "$p"
    [ "$rc" -eq 0 ] && ok "mutant $1: second run is let through (test would be red)" || bad "mutant $1 not caught (rc $rc)"
  fi
  rm -f "$R/.gates.lock"
}
mutant "compare always equal" 's/if cmp -s "\$1" "\$2"; then/if true; then/' change
mutant "files not hashed" 's/echo "\$(sha256sum < "\$root\/\$f" | cut -d. . -f1)  \$f"/echo "$f"/' content
mutant "lock check removed" 's/kill -0 "\$pid" 2>\/dev\/null/false/' parallel
mutant "lock never taken" 's/if (set -o noclobber; echo "\$\$" > "\$lock") 2>\/dev\/null; then/if true; then/' parallel
mutant "gates.sh without guard end" 's/^gates_guard_compare().*/gates_guard_compare() { return 0; }\n_unused() {/' change

# 6 gates.sh really uses the guard (the wiring)
grep -q 'gates_guard_lock' "$here/gates.sh" && grep -q 'gates_guard_compare' "$here/gates.sh" && ok "gates.sh calls lock and compare" || bad "gates.sh has no guard"

if [ "$fails" -eq 0 ]; then echo "test_gates_guard: all ok"; else echo "test_gates_guard: $fails FAILED"; exit 1; fi
