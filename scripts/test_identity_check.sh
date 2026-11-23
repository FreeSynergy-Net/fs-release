#!/usr/bin/env bash
# Self test of scripts/identity-check: allowed identity green; machine address as author, as committer, in the middle of the range: red (commit named,
# address not); commit before the base is not looked at; missing base rc 2; mutant (the comparison made always true) turns the red cases green.
#   bash scripts/test_identity_check.sh
set -uo pipefail
here="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
tool="$here/identity-check"
tmp="$(mktemp -d)"; trap 'rm -rf "$tmp"' EXIT
fails=0
chk() { local n="$1"; shift; if "$@"; then echo "ok   $n"; else echo "FAIL $n"; fails=$((fails + 1)); fi; }
# c <author email> <committer email> <file>
c() { echo "$3" >"$tmp/r/$3"; git -C "$tmp/r" add . && GIT_AUTHOR_NAME=a GIT_AUTHOR_EMAIL="$1" GIT_COMMITTER_NAME=c GIT_COMMITTER_EMAIL="$2" git -C "$tmp/r" -c commit.gpgsign=false commit -q -m "$3"; }
OK=support@freesynergy.net; BAD=agents@Beelink.hnc
mkdir "$tmp/r"; git -C "$tmp/r" init -q -b main
c "$BAD" "$BAD" old   # before the base: the exception for the existing stock
git -C "$tmp/r" branch base
run() { bash "${1:-$tool}" "$tmp/r" base >"$tmp/o" 2>&1; rc=$?; }
c $OK $OK one; run; chk "allowed identity: green (rc 0)" test $rc -eq 0
c $BAD $OK two; run; chk "foreign author: red (rc 1), commit named" test $rc -eq 1 -a -n "$(grep "$(git -C "$tmp/r" rev-parse --short HEAD)" "$tmp/o")"
chk "the address is never printed" test -z "$(grep -F "$BAD" "$tmp/o")"
git -C "$tmp/r" reset -q --hard HEAD~1; c $OK agent@localhost three; run; chk "foreign committer: red (rc 1)" test $rc -eq 1
c $OK $OK four; run; chk "foreign commit in the middle of the range: red" test $rc -eq 1
git -C "$tmp/r" branch -f base HEAD; run; chk "empty range: green" test $rc -eq 0
bash "$tool" "$tmp/r" nonexistent >/dev/null 2>&1; chk "missing base: rc 2" test $? -eq 2
FS_ALLOWED_IDENTITY=other@example.org bash "$tool" "$tmp/r" HEAD~1 >/dev/null 2>&1; chk "FS_ALLOWED_IDENTITY is read (other value: red)" test $? -eq 1
# special characters in the address must never shift the fields (a blank, a TAB): author "ok ok" with a blank + foreign committer, TAB in author, TAB+blank in committer
git -C "$tmp/r" branch -f base HEAD
c "$OK $OK" bad@x blank; run; chk "blank in the author address cannot hide a foreign committer: red" test $rc -eq 1
git -C "$tmp/r" reset -q --hard HEAD~1; c "$(printf 'x\t%s' "$OK")" "$OK" tab; run; chk "TAB in the author address: red" test $rc -eq 1
git -C "$tmp/r" reset -q --hard HEAD~1; c "$OK" "$(printf '%s\tx %s' "$OK" "$OK")" tab2; run; chk "TAB and blank in the committer address: red" test $rc -eq 1
git -C "$tmp/r" reset -q --hard HEAD~1
# mutant of the old kind (blank separated fields): the blank case turns green, so the test sees the difference
c "$OK $OK" bad@x blank2; sed 's/-F.\\t. //; s/%x09/ /g; s/NF != 3 || //' "$tool" >"$tmp/old"; run "$tmp/old"; chk "old blank-separated comparison lets the blank address through (green)" test $rc -eq 0
run; chk "the new tool is red on the same range" test $rc -eq 1
git -C "$tmp/r" reset -q --hard HEAD~1; git -C "$tmp/r" branch -f base HEAD~2
# mutant: the comparison always true -> the red cases must turn green (the test sees the difference)
# shellcheck disable=SC2016
sed 's/\$2 != ok || \$3 != ok/0/' "$tool" >"$tmp/mut"; git -C "$tmp/r" branch -f base HEAD~2; run "$tmp/mut"; chk "mutant is green where the original is red" test $rc -eq 0
run; chk "original is red on the same range" test $rc -eq 1
if [ "$fails" -eq 0 ]; then echo "all ok"; else echo "$fails FAILED"; exit 1; fi
