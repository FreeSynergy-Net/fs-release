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
  cp "$here/gates.sh" "$here/lib_gates_guard.sh" "$here/lib_gates_list.sh" "$1/s/scripts/"
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
{ [ "$rc" -eq 0 ] && grep -q "^gates ok: " "$tmp/out"; } && ok "unchanged tree: rc 0" || bad "unchanged tree: rc $rc"
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
{ [ "$rc1" -eq 0 ] && grep -q "^gates ok: " "$tmp/out1"; } && ok "the first run is not disturbed (rc 0)" || bad "first run rc $rc1"

# 4 orphaned lock
sleep 0.1 & dead=$!; wait "$dead"
echo "$dead" > "$R/.gates.lock"
run "$tmp/real" "$R" "true"; rc=$?
{ [ "$rc" -eq 0 ] && [ ! -e "$R/.gates.lock" ]; } && ok "orphaned lock (dead PID) replaced: rc 0" || bad "orphaned lock: rc $rc"
echo "garbage" > "$R/.gates.lock"
run "$tmp/real" "$R" "true"; rc=$?
[ "$rc" -eq 0 ] && ok "unreadable lock replaced: rc 0" || bad "unreadable lock: rc $rc"

# 4b the lock PID is alive but no gates.sh (reused PID): the lock is replaced
sleep 5 & other=$!
echo "$other" > "$R/.gates.lock"
run "$tmp/real" "$R" "true"; rc=$?
kill "$other" 2>/dev/null; wait "$other" 2>/dev/null
[ "$rc" -eq 0 ] && ok "lock with a live PID that is no gates.sh replaced: rc 0" || bad "reused PID lock: rc $rc"

# 4c the snapshot is not in /tmp but in <git dir>/gates-guard; an orphaned one (dead PID) is swept, one of a live PID stays
mkdir -p "$R/.git/gates-guard"; echo old > "$R/.git/gates-guard/snap.$dead"; echo old > "$R/.git/gates-guard/snap.$dead.end"
sleep 5 & live=$!
echo keep > "$R/.git/gates-guard/snap.$live"
run "$tmp/real" "$R" 'ls "$(git rev-parse --absolute-git-dir)/gates-guard" > '"$tmp/ls"; rc=$?
kill "$live" 2>/dev/null; wait "$live" 2>/dev/null
{ [ "$rc" -eq 0 ] && grep -q '^snap\.[0-9]*$' "$tmp/ls"; } && ok "snapshot lives in <git dir>/gates-guard" || bad "snapshot location (rc $rc)"
{ [ ! -e "$R/.git/gates-guard/snap.$dead" ] && [ ! -e "$R/.git/gates-guard/snap.$dead.end" ]; } && ok "orphaned snapshots (dead PID) swept" || bad "orphaned snapshot left"
[ -e "$R/.git/gates-guard/snap.$live" ] && ok "snapshot of a live PID is kept" || bad "live snapshot swept"
rm -f "$R/.git/gates-guard/snap.$live"
[ -z "$(ls -A "$R/.git/gates-guard")" ] && ok "no snapshot left after the run" || bad "snapshot left after the run"

# 4d the sweep deletes only snap.<number>[.end]: foreign names stay
sweepcase() { # <scripts dir> -> prints rc; leaves $R/.git/gates-guard populated by the caller
  run "$1" "$R" "true"
}
mkforeign() { mkdir -p "$R/.git/gates-guard"; for n in snap.abc snap.12.x notiz snap. snap.1a 4999999 4999999.end; do echo f > "$R/.git/gates-guard/$n"; done; }
mkforeign
sweepcase "$tmp/real"; rc=$?
left=0; for n in snap.abc snap.12.x notiz snap. snap.1a 4999999 4999999.end; do [ -e "$R/.git/gates-guard/$n" ] || left=1; done
{ [ "$rc" -eq 0 ] && [ "$left" -eq 0 ]; } && ok "sweep keeps foreign files (snap.abc, snap.12.x, notiz, snap., snap.1a, bare number)" || bad "sweep removed a foreign file (rc $rc)"
for n in snap.abc snap.12.x notiz snap. snap.1a 4999999 4999999.end; do rm -f "$R/.git/gates-guard/$n"; done

# 5 mutants: each must turn a case red
mutant() { # <label> <sed expr> <case: change|content|parallel|orphan> [file in scripts/, default lib_gates_guard.sh]
  local mf="${4:-lib_gates_guard.sh}"
  mkscripts "$tmp/m"; sed -i "$2" "$tmp/m/s/scripts/$mf"
  if cmp -s "$tmp/m/s/scripts/$mf" "$here/$mf"; then bad "mutant $1: the sed expression changed nothing"; return; fi
  if [ "$3" = change ] || [ "$3" = content ]; then
    # content: an untracked file that exists at the start gets other content (the status lines stay equal, only the hash shows it)
    echo u > "$R/u.txt"
    if [ "$3" = content ]; then run "$tmp/m" "$R" "echo changed > u.txt"; else run "$tmp/m" "$R" "echo x > new.txt"; fi
    rc=$?
    git -C "$R" clean -fdq
    [ "$rc" -eq 0 ] && ok "mutant $1: change in the middle goes unnoticed (test would be red)" || bad "mutant $1 not caught (rc $rc)"
  elif [ "$3" = orphan ]; then
    mkdir -p "$R/.git/gates-guard"; echo old > "$R/.git/gates-guard/snap.$dead"
    run "$tmp/m" "$R" "true"
    [ -e "$R/.git/gates-guard/snap.$dead" ] && ok "mutant $1: orphaned snapshot stays (test would be red)" || bad "mutant $1 not caught"
    rm -f "$R/.git/gates-guard/snap.$dead"
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

# the end of the REAL run (gates.sh, the lines after the checks) is the same code in test mode: remove / neutralise its compare
mutant "gates.sh: end compare removed" '/^  gates_guard_compare "\$guard_snap" "\$guard_snap.end"$/d' change gates.sh
mutant "gates.sh: end compare || true" 's/^  gates_guard_compare "\$guard_snap" "\$guard_snap.end"$/& || true/' change gates.sh
mutant "gates.sh: snapshot in /tmp" 's|^guard_dir=.*|guard_dir="/tmp"|' orphan gates.sh
mutant "sweep removed" 's/kill -0 "\$pid" 2>\/dev\/null || rm -f -- "\$f"/true/' orphan
# mutant: the PID check does not look at gates.sh (return 0 at the start of gates_guard_is_gates) -> a lock with a live foreign PID
# blocks the run (the case of 4b turns red)
mkscripts "$tmp/m"; sed -i 's/^gates_guard_is_gates() {$/&\n  return 0/' "$tmp/m/s/scripts/lib_gates_guard.sh"
if cmp -s "$tmp/m/s/scripts/lib_gates_guard.sh" "$here/lib_gates_guard.sh"; then bad "mutant pid check: sed changed nothing"; else
  sleep 5 & other=$!; echo "$other" > "$R/.gates.lock"
  run "$tmp/m" "$R" "true"; rc=$?; kill "$other" 2>/dev/null; wait "$other" 2>/dev/null; rm -f "$R/.gates.lock"
  [ "$rc" -ne 0 ] && ok "mutant pid check ignores gates.sh: reused-PID case goes red (rc $rc)" || bad "mutant pid check not caught"
fi

# sweep mutants: number check removed; glob `*` instead of `snap.*`: the foreign files are deleted (the case of 4d turns red)
foreign_mutant() { # <label> <sed expr>
  mkscripts "$tmp/m"; sed -i "$2" "$tmp/m/s/scripts/lib_gates_guard.sh"
  if cmp -s "$tmp/m/s/scripts/lib_gates_guard.sh" "$here/lib_gates_guard.sh"; then bad "mutant $1: sed changed nothing"; return; fi
  mkforeign; sweepcase "$tmp/m"
  gone=0; for n in snap.abc snap.12.x notiz snap. snap.1a 4999999 4999999.end; do [ -e "$R/.git/gates-guard/$n" ] || gone=1; done
  [ "$gone" -eq 1 ] && ok "mutant $1: a foreign file is deleted (case 4d turns red)" || bad "mutant $1 not caught"
  for n in snap.abc snap.12.x notiz snap. snap.1a 4999999 4999999.end; do rm -f "$R/.git/gates-guard/$n"; done
}
foreign_mutant "sweep without number check" 's/case "\$pid" in .*esac//'
foreign_mutant "sweep glob * instead of snap.*" 's|for f in "\$1"/snap\.\*; do|for f in "$1"/*; do|'
foreign_mutant "sweep strips only .end, no name form" 's/pid="\${name#snap\.}"; pid="\${pid%\.end}"/pid="4999998"/'

# 6 gates.sh really uses the guard (the wiring)
grep -q 'gates_guard_lock' "$here/gates.sh" && grep -q 'gates_guard_compare' "$here/gates.sh" && ok "gates.sh calls lock and compare" || bad "gates.sh has no guard"

if [ "$fails" -eq 0 ]; then echo "test_gates_guard: all ok"; else echo "test_gates_guard: $fails FAILED"; exit 1; fi
