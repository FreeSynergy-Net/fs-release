#!/usr/bin/env bash
# Self test of scripts/workspace-test-background with a fake build-slot.sh and a small git repo (no cargo):
# start+wait green, red with runde counting, timeout (rc 3), head mismatch (rc 2), wait without start (rc 2), second start is no second run,
# a symlinked state folder is refused, the result is bound to the head (a new commit is a new result).
#   bash scripts/test_workspace_test_background.sh
set -uo pipefail
here="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
tool="$here/workspace-test-background"
tmp="$(mktemp -d)"
trap 'rm -rf "$tmp"' EXIT
fails=0
g() { git -c user.name=t -c user.email=t@example.org -c commit.gpgsign=false "$@"; }
chk() { # chk <name> <command...>: the command is the condition
  local name="$1"; shift
  if "$@"; then echo "ok   $name"; else echo "FAIL $name"; fails=$((fails + 1)); fi
}
isval() { [ "$(field "$1" <<<"$2")" = "$3" ]; }
rce() { [ "$rc" -eq "$1" ]; }

mkdir -p "$tmp/repo" "$tmp/state"
(cd "$tmp/repo" && g init -q . && echo a >a && g add . && g commit -q -m base)
# the fake slot: behaviour from the file mode next to it (green | red | slow)
cat >"$tmp/slot.sh" <<SLOT
#!/usr/bin/env bash
mode="\$(cat "$tmp/mode")"
case "\$mode" in
  green) ulimit -c >"$tmp/corelimit"; echo "test result: ok. 3 passed; 0 failed"; exit 0 ;;
  red) echo "test result: FAILED. 2 passed; 1 failed"; exit 101 ;;
  slow) exec "$tmp/sleeper" ;;
esac
SLOT
printf '#!/usr/bin/env bash\necho $$ >"%s/pid"\nexec tail -f /dev/null\n' "$tmp" >"$tmp/sleeper"
chmod +x "$tmp/slot.sh" "$tmp/sleeper"
export BUILD_SLOT_SH="$tmp/slot.sh" TEST_BG_DIR="$tmp/state"
head="$(g -C "$tmp/repo" rev-parse HEAD)"
field() { python3 -I -c 'import json,sys; print(json.load(sys.stdin)[sys.argv[1]])' "$1"; }

echo green >"$tmp/mode"
ulimit -S -c "$(ulimit -H -c)" 2>/dev/null || true  # raise the soft limit first, so the check below proves the run lowers it
out="$("$tool" start --repo "$tmp/repo" --head "$head")"; chk "start returns at once with started true" isval started "$out" True
res="$("$tool" wait --repo "$tmp/repo" --head "$head" --timeout 30)"; rc=$?
chk "wait: result green (rc 0, red false)" rce 0; chk "wait: bound to the head" isval head "$res" "$head"
chk "wait: only the summary line" grep -q "3 passed" <<<"$res"; chk "wait: red false" isval red "$res" False
chk "slot run has core dumps off (ulimit -c 0)" test "$(cat "$tmp/corelimit")" = 0
out="$("$tool" start --repo "$tmp/repo" --head "$head")"; chk "a second start for the same head is no second run" isval started "$out" False

# red twice in a row: runde counts per head only for the same head, a new commit is a new head
(cd "$tmp/repo" && echo b >>a && g commit -q -am two); head2="$(g -C "$tmp/repo" rev-parse HEAD)"
echo red >"$tmp/mode"
"$tool" start --repo "$tmp/repo" --head "$head2" >/dev/null
res="$("$tool" wait --repo "$tmp/repo" --head "$head2" --timeout 30)"; rc=$?
chk "red result: rc 0 and red true" rce 0; chk "red: red true" isval red "$res" True; chk "red: runde 1" isval runde "$res" 1; (cd "$tmp/repo" && echo b2 >>a && g commit -q -am two2); head2b="$(g -C "$tmp/repo" rev-parse HEAD)"; "$tool" start --repo "$tmp/repo" --head "$head2b" >/dev/null; res="$("$tool" wait --repo "$tmp/repo" --head "$head2b" --timeout 30)"; chk "red again on a NEW head: runde counts on (2)" isval runde "$res" 2; chk "red: the new head" isval head "$res" "$head2b"

# T4: the same red head started again (a fix that made no new commit) is a NEW run and counts: runde 3, never a cached red result for ever
out="$("$tool" start --repo "$tmp/repo" --head "$head2b")"; chk "a red head started again is run again" isval started "$out" True
res="$("$tool" wait --repo "$tmp/repo" --head "$head2b" --timeout 30)"; chk "the repeat of the same red head counts (runde 3)" isval runde "$res" 3
out="$("$tool" start --repo "$tmp/repo" --head "$head2b")"; chk "and again: runde 4, the loop is visible to the caller" isval started "$out" True
"$tool" wait --repo "$tmp/repo" --head "$head2b" --timeout 30 >/dev/null

# the head moved: both modes refuse
"$tool" wait --repo "$tmp/repo" --head "$head" --timeout 1 >/dev/null 2>&1; rc=$?
chk "a result of an older head is not read for the current head (rc 2)" rce 2
(cd "$tmp/repo" && echo c >>a && g commit -q -am three); head3="$(g -C "$tmp/repo" rev-parse HEAD)"
"$tool" wait --repo "$tmp/repo" --head "$head3" --timeout 1 >/dev/null 2>&1; rc=$?
chk "wait without a start is rc 2, never green" rce 2

# timeout: a run that does not end
echo slow >"$tmp/mode"
"$tool" start --repo "$tmp/repo" --head "$head3" >/dev/null
"$tool" wait --repo "$tmp/repo" --head "$head3" --timeout 1 >/dev/null 2>&1; rc=$?
chk "timeout while the run is still going is rc 3" rce 3
kill "$(cat "$tmp/pid")" 2>/dev/null || true
res="$("$tool" wait --repo "$tmp/repo" --head "$head3" --timeout 30 2>/dev/null)"; rc=$?
chk "a killed run ends the wait with a red result" rce 0; chk "a killed run is red, never green" isval red "$res" True

# the whole run dies without writing a result (kill -9 of its group): the wait is rc 2, never green
(cd "$tmp/repo" && echo d >>a && g commit -q -am four); head4="$(g -C "$tmp/repo" rev-parse HEAD)"
echo slow >"$tmp/mode"; rm -f "$tmp/pid"
"$tool" start --repo "$tmp/repo" --head "$head4" >/dev/null
for _ in $(seq 1 100); do [ -s "$tmp/pid" ] && break; "$tool" wait --repo "$tmp/repo" --head "$head4" --timeout 1 >/dev/null 2>&1; done
pg="$(ps -o pgid= -p "$(cat "$tmp/pid")" | tr -d ' ')"
kill -9 -- "-$pg" 2>/dev/null
"$tool" wait --repo "$tmp/repo" --head "$head4" --timeout 30 >/dev/null 2>&1; rc=$?
chk "a run that died without a result file is rc 2, never green" rce 2

"$tool" start --repo "$tmp/repo" --head deadbeef >/dev/null 2>&1; rc=$?
chk "head that is not the head of the repo is refused" rce 2

# the symlink check must be the reason: the CURRENT head is passed (head4), the link target stays untouched (mode 755, not chmod 700)
ln -s "$tmp/state" "$tmp/link"; chmod 755 "$tmp/state"
err="$(TEST_BG_DIR="$tmp/link" "$tool" start --repo "$tmp/repo" --head "$head4" 2>&1 >/dev/null)"; rc=$?
chk "a symlinked state folder is refused (rc 2)" rce 2
chk "... because of the symlink, not for another reason" grep -q "symlink" <<<"$err"
chk "... and the target of the link was not chmod-ed" test "$(stat -c %a "$tmp/state")" = 755

if [ "$fails" -eq 0 ]; then echo "all ok"; else echo "$fails failed"; exit 1; fi
