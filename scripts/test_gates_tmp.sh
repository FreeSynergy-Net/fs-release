#!/usr/bin/env bash
# Self test of the temp folder of a gates run (rule [GATES-TMP], scripts/gates.sh): a local run (no CI) gives the tests a TMPDIR folder of its own
# inside TMPDIR_GATES; it is gone after the run (ok, red and killed by SIGTERM), a folder of a dead PID is swept, one of a live PID stays.
# No cargo: gates.sh runs in test mode (GATES_ONLY_GUARD=1, GATES_GUARD_MIDDLE stands in for the long checks) on a throw-away git repo.
# Mutant: the removal taken out of the EXIT trap must turn the cases red.
#   bash scripts/test_gates_tmp.sh
set -uo pipefail
here="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
tmp="$(mktemp -d)"
trap 'rm -rf "$tmp"' EXIT
unset GITHUB_ACTIONS CI
fails=0
ok() { echo "ok   $1"; }
bad() { echo "FAIL $1"; fails=$((fails + 1)); }

mkscripts() { # <dir>
  mkdir -p "$1/s/scripts"
  cp "$here/gates.sh" "$here/lib_gates_guard.sh" "$here/lib_gates_list.sh" "$1/s/scripts/"
  echo "1.0.0" > "$1/s/RUST_VERSION"
}
R="$tmp/repo"; mkdir -p "$R" && git -C "$R" init -q
echo a > "$R/a.txt"; git -C "$R" add -A && git -C "$R" -c user.name=t -c user.email=t@t commit -qm base
base="$tmp/base"; mkdir -p "$base"
run() { # <scripts dir> <middle command>: local mode (CI unset), rc returned
  TMPDIR_GATES="$base" GATES_ONLY_GUARD=1 GATES_GUARD_MIDDLE="$2" bash "$1/s/scripts/gates.sh" "$R" > "$tmp/out" 2>&1
}
mkscripts "$tmp/real"

# 1 a test leaves a file and a folder in TMPDIR: nothing is left in the base after the run
run "$tmp/real" 'mkdir "$TMPDIR/.tmpLEAK" && echo x > "$TMPDIR/fs-app-live-1.png"; echo "$TMPDIR" > '"$tmp/seen"; rc=$?
{ [ "$rc" -eq 0 ] && [ -z "$(ls -A "$base")" ]; } && ok "ok run: base is empty afterwards" || { bad "ok run: rc $rc, left: $(ls -A "$base")"; }
case "$(cat "$tmp/seen" 2>/dev/null)" in "$base"/gates-run.*) ok "the tests wrote below $base/gates-run.*" ;; *) bad "TMPDIR of the tests: $(cat "$tmp/seen" 2>/dev/null)" ;; esac
# 2 a red run
run "$tmp/real" 'mkdir "$TMPDIR/.tmpLEAK"; false'; rc=$?
{ [ "$rc" -ne 0 ] && [ -z "$(ls -A "$base")" ]; } && ok "red run: rc $rc, base is empty afterwards" || bad "red run: rc $rc, left: $(ls -A "$base")"
# 3 killed by SIGTERM while a test runs
TMPDIR_GATES="$base" GATES_ONLY_GUARD=1 GATES_GUARD_MIDDLE='mkdir "$TMPDIR/.tmpLEAK"; sleep 4' bash "$tmp/real/s/scripts/gates.sh" "$R" > "$tmp/out" 2>&1 &
pid=$!
for _ in $(seq 1 50); do ls "$base"/gates-run.*/.tmpLEAK > /dev/null 2>&1 && break; sleep 0.1; done
kill -TERM "$pid" 2>/dev/null; wait "$pid" 2>/dev/null
sleep 0.5
[ -z "$(ls -A "$base")" ] && ok "SIGTERM: base is empty afterwards" || bad "SIGTERM: left: $(ls -A "$base")"
# 4 sweep: a dead PID's folder goes, a live PID's folder and foreign names stay
sleep 0.1 & dead=$!; wait "$dead"
sleep 5 & live=$!
mkdir -p "$base/gates-run.$dead.AAAAAA" "$base/gates-run.$live.BBBBBB" "$base/gates-run.abc.CCC" "$base/other"
run "$tmp/real" 'true'; rc=$?
kill "$live" 2>/dev/null; wait "$live" 2>/dev/null
{ [ "$rc" -eq 0 ] && [ ! -e "$base/gates-run.$dead.AAAAAA" ] && [ -d "$base/gates-run.$live.BBBBBB" ] && [ -d "$base/gates-run.abc.CCC" ] && [ -d "$base/other" ]; } \
  && ok "sweep: dead PID folder removed, live and foreign kept" || bad "sweep (rc $rc): $(ls -A "$base")"
rm -rf "$base"; mkdir -p "$base"

# 5 exit before the trap of the full run (S1): uncommitted changes, and a lock held by another run -> the folder is gone as well
echo b > "$R/dirty.txt"; git -C "$R" add dirty.txt
run "$tmp/real" 'true'; rc=$?
{ [ "$rc" -eq 1 ] && [ -z "$(ls -A "$base")" ]; } && ok "exit on uncommitted changes: rc 1, base is empty" || bad "uncommitted: rc $rc, left: $(ls -A "$base")"
git -C "$R" rm -qf --cached dirty.txt; rm -f "$R/dirty.txt"
TMPDIR_GATES="$base" GATES_ONLY_GUARD=1 GATES_GUARD_MIDDLE='sleep 6' bash "$tmp/real/s/scripts/gates.sh" "$R" > "$tmp/out1" 2>&1 &
first=$!
for _ in $(seq 1 50); do [ -e "$R/.gates.lock" ] && break; sleep 0.1; done
run "$tmp/real" 'true'; rc=$?
n="$(ls -A "$base" | wc -l)"
kill -TERM "$first" 2>/dev/null; wait "$first" 2>/dev/null; sleep 0.5
{ [ "$rc" -eq 1 ] && [ "$n" -eq 1 ] && [ -z "$(ls -A "$base")" ]; } && ok "lock held: rc 1, only the folder of the first run (gone after it)" || bad "lock held: rc $rc, folders during: $n, after: $(ls -A "$base")"
rm -rf "$base"; mkdir -p "$base"
# 6 a leftover that cannot be removed (read-only folder, S2) is a warning, not a red gate: in the EXIT trap and in the sweep
run "$tmp/real" 'mkdir -p "$TMPDIR/ro/sub"; chmod 500 "$TMPDIR/ro"'; rc=$?
{ [ "$rc" -eq 0 ] && grep -q 'warning: could not remove' "$tmp/out"; } && ok "unremovable rest in the run folder: rc 0 and a warning" || bad "unremovable (trap): rc $rc: $(tail -3 "$tmp/out")"
chmod -R u+rwX "$base"; rm -rf "$base"; mkdir -p "$base"
sleep 0.1 & dead=$!; wait "$dead"
mkdir -p "$base/gates-run.$dead.RO/ro/sub"; chmod 500 "$base/gates-run.$dead.RO/ro"
run "$tmp/real" 'true'; rc=$?
{ [ "$rc" -eq 0 ] && grep -q 'warning: could not remove' "$tmp/out"; } && ok "unremovable rest of a dead run in the sweep: rc 0 and a warning" || bad "unremovable (sweep): rc $rc: $(tail -3 "$tmp/out")"
chmod -R u+rwX "$base"; rm -rf "$base"; mkdir -p "$base"
# 7 a relative TMPDIR_GATES is refused (S4), rc 2, nothing created in the repo
TMPDIR_GATES="rel" GATES_ONLY_GUARD=1 GATES_GUARD_MIDDLE='true' bash "$tmp/real/s/scripts/gates.sh" "$R" > "$tmp/out" 2>&1; rc=$?
{ [ "$rc" -eq 2 ] && [ ! -e "$R/rel" ] && grep -q 'must be an absolute path' "$tmp/out"; } && ok "relative TMPDIR_GATES: rc 2" || bad "relative base: rc $rc"

# 8 mutants: each must be caught by the cases above
# (a) no removal in the EXIT trap -> case 1 red
mkscripts "$tmp/m"
sed -i 's|; if \[ -n "\$gates_run_tmp" \]; then gates_rm_tmp "\$gates_run_tmp"; fi||' "$tmp/m/s/scripts/gates.sh"
if cmp -s "$tmp/m/s/scripts/gates.sh" "$here/gates.sh"; then bad "mutant a: the sed expression changed nothing"; else
  run "$tmp/m" 'mkdir "$TMPDIR/.tmpLEAK"'; rc=$?
  [ -n "$(ls -A "$base")" ] && ok "mutant a (no removal): a leftover stays (test would be red)" || bad "mutant a not caught (rc $rc)"
fi
rm -rf "$base"; mkdir -p "$base"
# (b) exit before the trap: the early trap taken out -> the uncommitted-changes exit leaves the folder
mkscripts "$tmp/m2"
sed -i '/# at once: the exits below/d' "$tmp/m2/s/scripts/gates.sh"
if cmp -s "$tmp/m2/s/scripts/gates.sh" "$here/gates.sh"; then bad "mutant b: the sed expression changed nothing"; else
  echo b > "$R/dirty.txt"; git -C "$R" add dirty.txt
  run "$tmp/m2" 'true'; rc=$?
  git -C "$R" rm -qf --cached dirty.txt; rm -f "$R/dirty.txt"
  [ -n "$(ls -A "$base")" ] && ok "mutant b (exit before the trap): the folder stays (test would be red)" || bad "mutant b not caught (rc $rc)"
fi
rm -rf "$base"; mkdir -p "$base"
# (c) a removal that fails is red: the function without the warning branch
mkscripts "$tmp/m3"
sed -i 's#rm -rf "\$1" 2>/dev/null || echo "gates: warning: could not remove \$1" >&2; return 0;#rm -rf "$1";#' "$tmp/m3/s/scripts/gates.sh"
if cmp -s "$tmp/m3/s/scripts/gates.sh" "$here/gates.sh"; then bad "mutant c: the sed expression changed nothing"; else
  run "$tmp/m3" 'mkdir -p "$TMPDIR/ro/sub"; chmod 500 "$TMPDIR/ro"'; rc=$?
  [ "$rc" -ne 0 ] && ok "mutant c (failing removal is fatal): rc $rc (test would be red)" || bad "mutant c not caught"
fi
chmod -R u+rwX "$base"; rm -rf "$base"
[ "$fails" -eq 0 ] && echo "test_gates_tmp: all ok" || { echo "test_gates_tmp: $fails failed"; exit 1; }
