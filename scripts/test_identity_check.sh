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
env -u CI -u GITHUB_ACTIONS FS_ALLOWED_IDENTITY=other@example.org bash "$tool" "$tmp/r" HEAD~1 >/dev/null 2>&1; chk "FS_ALLOWED_IDENTITY is read (other value: red)" test $? -eq 1
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
# --- Bundle 5 cases ---
git -C "$tmp/r" branch -f base HEAD~2
# MUST: a call from a foreign cwd (a directory without Git) with the root argument works (junk lines before the shebang once ran before the cd)
mkdir "$tmp/nogit"; (cd "$tmp/nogit" && bash "$tool" "$tmp/r" HEAD~1 >"$tmp/o" 2>&1); chk "foreign cwd without Git, root argument: green (rc 0)" test $? -eq 0
chk "the script starts with the shebang (line 1)" test "$(head -n 1 "$tool")" = '#!/usr/bin/env bash'
sed '1a hits="$(git log --format=%h ..HEAD)" || exit 2' "$tool" >"$tmp/mutcwd"; (cd "$tmp/nogit" && bash "$tmp/mutcwd" "$tmp/r" HEAD~1 >/dev/null 2>&1); chk "mutant (a git call before the cd) is red in a foreign cwd" test $? -eq 2
# S3: in CI the environment cannot loosen the gate; a repo with origin/main behind a foreign commit stays red
mkdir "$tmp/c"; git -C "$tmp/c" init -q -b main; echo a >"$tmp/c/a"; git -C "$tmp/c" add . && GIT_AUTHOR_NAME=a GIT_AUTHOR_EMAIL=$BAD GIT_COMMITTER_NAME=a GIT_COMMITTER_EMAIL=$BAD git -C "$tmp/c" -c commit.gpgsign=false commit -q -m bad
git -C "$tmp/c" update-ref refs/remotes/origin/main HEAD
c2() { echo "$1" >"$tmp/c/$1"; git -C "$tmp/c" add . && GIT_AUTHOR_NAME=a GIT_AUTHOR_EMAIL="$2" GIT_COMMITTER_NAME=a GIT_COMMITTER_EMAIL="$2" git -C "$tmp/c" -c commit.gpgsign=false commit -q -m "$1"; }
c2 good1 $OK; c2 bad2 $BAD
envrun() { env "$@" bash "$tool" "$tmp/c" >"$tmp/o" 2>&1; }
envrun CI=true; chk "CI, origin/main base, foreign commit in range: red (rc 1)" test $? -eq 1
envrun CI=true FS_IDENTITY_BASE=HEAD; chk "CI with FS_IDENTITY_BASE=HEAD: rc 1 (an error, not a warning)" test $? -eq 1
envrun CI=true FS_ALLOWED_IDENTITY=$BAD; chk "CI with a changed FS_ALLOWED_IDENTITY: rc 1" test $? -eq 1
envrun CI=true FS_IDENTITY_BASE=origin/main; chk "CI with FS_IDENTITY_BASE=origin/main (the same): the gate runs, red on the foreign commit" test $? -eq 1
envrun CI=true GIT_CONFIG_COUNT=1 GIT_CONFIG_KEY_0=freesynergy.identity-test-base GIT_CONFIG_VALUE_0=HEAD; chk "CI: GIT_CONFIG_* from the environment cannot name a test base (still red)" test $? -eq 1
env -u CI -u GITHUB_ACTIONS FS_IDENTITY_BASE=HEAD bash "$tool" "$tmp/c" >/dev/null 2>&1; chk "local (no CI): FS_IDENTITY_BASE=HEAD is allowed (empty range, green)" test $? -eq 0
git -C "$tmp/c" config --local freesynergy.identity-test-base HEAD; envrun CI=true; chk "CI: the fixture key in the LOCAL repo config names the test base (green on the empty range)" test $? -eq 0
git -C "$tmp/c" config --local --unset freesynergy.identity-test-base
sed 's/\*) ci=1 ;;/*) ci=0 ;;/' "$tool" >"$tmp/mutci"; chk "mutant (CI branch off) differs from the original" test "$(cmp -s "$tool" "$tmp/mutci" && echo same)" != same
env CI=true FS_IDENTITY_BASE=HEAD bash "$tmp/mutci" "$tmp/c" >/dev/null 2>&1; chk "mutant (CI branch off): the CI case turns green (the test sees it)" test $? -eq 0
# S2: git replace must not hide a foreign commit
badsha="$(git -C "$tmp/c" rev-parse HEAD)"; goodsha="$(git -C "$tmp/c" rev-parse HEAD~1)"
git -C "$tmp/c" replace "$badsha" "$goodsha"; env -u CI -u GITHUB_ACTIONS bash "$tool" "$tmp/c" origin/main >/dev/null 2>&1; chk "git replace of the foreign commit: still red" test $? -eq 1
sed 's/--no-replace-objects //' "$tool" >"$tmp/mutrep"; env -u CI -u GITHUB_ACTIONS bash "$tmp/mutrep" "$tmp/c" origin/main >/dev/null 2>&1; chk "mutant (without --no-replace-objects): the replace trick turns green" test $? -eq 0
git -C "$tmp/c" replace -d "$badsha" >/dev/null
# --- Bundle 6 (N1): the CI case must not hang on the caller-settable CI alone, and GIT_DIR & co must not redirect the gate ---
envrun CI=false GITHUB_ACTIONS=true FS_IDENTITY_BASE=HEAD; chk "N1: CI=false GITHUB_ACTIONS=true FS_IDENTITY_BASE=HEAD: rc 1 (GITHUB_ACTIONS keeps the CI case)" test $? -eq 1
envrun CI=TRUE GITHUB_ACTIONS=true FS_IDENTITY_BASE=HEAD; chk "N1: CI=TRUE GITHUB_ACTIONS=true FS_IDENTITY_BASE=HEAD: rc 1" test $? -eq 1
envrun CI=false GITHUB_ACTIONS=true FS_ALLOWED_IDENTITY=$BAD; chk "N1: CI=false GITHUB_ACTIONS=true FS_ALLOWED_IDENTITY changed: rc 1" test $? -eq 1
mkdir "$tmp/d"; git -C "$tmp/d" init -q -b main; echo a >"$tmp/d/a"; git -C "$tmp/d" add . && GIT_AUTHOR_NAME=a GIT_AUTHOR_EMAIL=$OK GIT_COMMITTER_NAME=a GIT_COMMITTER_EMAIL=$OK git -C "$tmp/d" -c commit.gpgsign=false commit -q -m ok
git -C "$tmp/d" update-ref refs/remotes/origin/main HEAD
for v in GIT_DIR GIT_WORK_TREE GIT_OBJECT_DIRECTORY GIT_ALTERNATE_OBJECT_DIRECTORIES; do
  envrun CI=true "$v=$tmp/d/.git"; chk "N1: CI with $v set: rc 1" test $? -eq 1
done
envrun CI=false GITHUB_ACTIONS=true GIT_DIR="$tmp/d/.git"; chk "N1: GITHUB_ACTIONS=true CI=false with GIT_DIR: rc 1" test $? -eq 1
env -u CI -u GITHUB_ACTIONS GIT_DIR="$tmp/d/.git" bash "$tool" "$tmp/c" >/dev/null 2>&1; chk "N1: local (no CI) with GIT_DIR: not refused by the CI rule (green on the clean repo)" test $? -eq 0
sed 's/"\${GITHUB_ACTIONS:-}"/""/g' "$tool" >"$tmp/mutga"; env CI=false GITHUB_ACTIONS=true FS_IDENTITY_BASE=HEAD bash "$tmp/mutga" "$tmp/c" >/dev/null 2>&1; chk "N1 mutant (GITHUB_ACTIONS not read): the bypass turns green (the test sees it)" test $? -eq 0
if [ "$fails" -eq 0 ]; then echo "all ok"; else echo "$fails FAILED"; exit 1; fi
