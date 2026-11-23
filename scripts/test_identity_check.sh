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
# --- Bundle 7 (N1c, S9): identity is the FIRST gate; Repo code (build.rs, tests: here a stub cargo) must not move refs/remotes/origin/main before it ---
envrun CI=false GITHUB_ACTIONS=true FS_IDENTITY_BASE=HEAD; chk "S9: the CI message names CI or GITHUB_ACTIONS (not only CI=true)" test -n "$(grep -F 'in CI (CI or GITHUB_ACTIONS set)' "$tmp/o")" -a -z "$(grep -F 'CI=true' "$tmp/o")"
gates_src="$(cd "$here" && pwd)/gates.sh"
chk "N1c static: in gates.sh the step identity stands before the first cargo step" test "$(grep -n '^  step identity ' "$gates_src" | head -1 | cut -d: -f1)" -lt "$(grep -n '^  step [a-z-]* [a-z]* cargo ' "$gates_src" | head -1 | cut -d: -f1)"
chk "N1c static: step identity exists exactly once" test "$(grep -c '^  step identity ' "$gates_src")" -eq 1
# dynamic: a workspace whose stub cargo does what a hostile build.rs would do (origin/main := HEAD); origin/main stands before a foreign commit
GK="$tmp/gk"; mkdir -p "$GK/scripts" "$GK/bin"
cp "$here/gates.sh" "$here/lib_gates_guard.sh" "$here/lib_gates_list.sh" "$tool" "$GK/scripts/"
echo 1.0.0 >"$GK/RUST_VERSION"; printf '#!/usr/bin/env bash\nexit 0\n' >"$GK/scripts/gates-lock.sh"; cp "$GK/scripts/gates-lock.sh" "$GK/scripts/gates-files.sh"; cp "$GK/scripts/gates-lock.sh" "$GK/scripts/licence-gate.sh"; : >"$GK/scripts/check_rust_rules.py"
printf '#!/usr/bin/env bash\ngit -C "$FIXTURE" update-ref refs/remotes/origin/main HEAD\nexit 0\n' >"$GK/bin/cargo"
printf '#!/usr/bin/env bash\necho rustc 0\n' >"$GK/bin/rustc"; chmod +x "$GK/bin/cargo" "$GK/bin/rustc"
cp -r "$tmp/c" "$tmp/e"; git -C "$tmp/e" update-ref refs/remotes/origin/main "$(git -C "$tmp/e" rev-parse HEAD~2)"; git -C "$tmp/e" config --local --unset freesynergy.identity-test-base 2>/dev/null
gates_ci() { env -u GATES_ONLY_GUARD -u GIT_DIR PATH="$GK/bin:$PATH" FIXTURE="$tmp/e" CI=true GITHUB_ACTIONS=true TMPDIR="$tmp" FS_GATES_FEHLERLISTE_DIR="$tmp" bash "$1" "$tmp/e" >"$tmp/g.out" 2>&1; }
gates_ci "$GK/scripts/gates.sh"; grc=$?
chk "N1c: stub cargo moves origin/main, identity still red: 'gate identity: rc=1', gates rc 1" test "$grc" -eq 1 -a -n "$(grep -x 'gate identity: rc=1' "$tmp/g.out")"
chk "N1c: the identity line comes before the first cargo gate line" test "$(grep -n '^gate identity: ' "$tmp/g.out" | head -1 | cut -d: -f1)" -lt "$(grep -n '^gate fmt: ' "$tmp/g.out" | head -1 | cut -d: -f1)"
git -C "$tmp/e" update-ref refs/remotes/origin/main "$(git -C "$tmp/e" rev-parse HEAD~2)"
# mutant: the old order (identity after files): the stub has moved the ref by then, identity is green on an empty range (the bypass of REVIEW-bundle6 N1c)
awk '/^  step identity /{next} {print} /^  step files /{print "  step identity error bash \"$here/identity-check\" \"$root\""}' "$GK/scripts/gates.sh" >"$GK/scripts/gates-old.sh"
gates_ci "$GK/scripts/gates-old.sh"
chk "N1c mutant (identity last, the old order): identity turns green on the moved ref (the test sees it)" test -n "$(grep -x 'gate identity: rc=0' "$tmp/g.out")"
git -C "$tmp/e" update-ref refs/remotes/origin/main "$(git -C "$tmp/e" rev-parse HEAD~2)"
# --- Bundle 8 (decision 9f): on pull_request in CI the checkout is GitHub's merge commit (committer GitHub); the tip is its 2nd parent, the PR head, as in the job identity ---
mkdir "$tmp/m"; M() { git -C "$tmp/m" "$@"; }
mc() { echo "$1" >"$tmp/m/$1"; M add . && GIT_AUTHOR_NAME=a GIT_AUTHOR_EMAIL=$OK GIT_COMMITTER_NAME=a GIT_COMMITTER_EMAIL=$OK M -c commit.gpgsign=false commit -q -m "$1"; }
M init -q -b main; mc base; M checkout -q -b pr; mc prc; M checkout -q main; mc m1; M update-ref refs/remotes/origin/main HEAD
GIT_AUTHOR_NAME=a GIT_AUTHOR_EMAIL=$OK GIT_COMMITTER_NAME=GitHub GIT_COMMITTER_EMAIL=noreply@github.com M -c commit.gpgsign=false merge -q --no-ff -m merge pr
mrun() { env "$@" bash "$tool" "$tmp/m" >"$tmp/o" 2>&1; }
mrun CI=true GITHUB_EVENT_NAME=pull_request GITHUB_SHA="$(M rev-parse HEAD)"; chk "9f: pull_request, merge commit of GitHub, PR head clean: green (rc 0)" test $? -eq 0
mrun CI=true GITHUB_EVENT_NAME=push; chk "9f: the same merge commit on another event: red (the tip stays HEAD)" test $? -eq 1
sed "s/tip='HEAD^2'/tip=HEAD/" "$tool" >"$tmp/mutpr"; env CI=true GITHUB_EVENT_NAME=pull_request GITHUB_SHA="$(M rev-parse HEAD)" bash "$tmp/mutpr" "$tmp/m" >/dev/null 2>&1; chk "9f mutant (tip stays HEAD): the pull_request case turns red (the test sees it)" test $? -eq 1
# --- M1 (r1 review): the PR head itself is a merge commit (GitHub "Update branch" / local `git merge main`) with a foreign identity in a feature commit; HEAD is then NOT GITHUB_SHA, the tip must stay HEAD ---
mkdir "$tmp/h"; H() { git -C "$tmp/h" "$@"; }
hc() { echo "$1" >"$tmp/h/$1"; H add . && GIT_AUTHOR_NAME=a GIT_AUTHOR_EMAIL=${2:-$OK} GIT_COMMITTER_NAME=a GIT_COMMITTER_EMAIL=${2:-$OK} H -c commit.gpgsign=false commit -q -m "$1"; }
H init -q -b main; hc base; H checkout -q -b feat; hc bad agents@Beelink.hnc; H checkout -q main; hc m1; H update-ref refs/remotes/origin/main HEAD; H checkout -q feat
GIT_AUTHOR_NAME=a GIT_AUTHOR_EMAIL=$OK GIT_COMMITTER_NAME=a GIT_COMMITTER_EMAIL=$OK H -c commit.gpgsign=false merge -q --no-ff -m "merge main" main
hrun() { env "$@" bash "$tool" "$tmp/h" >"$tmp/o" 2>&1; }
hrun CI=true GITHUB_EVENT_NAME=pull_request GITHUB_SHA=0000000000000000000000000000000000000000; chk "M1: pull_request, PR head is itself a merge commit (HEAD != GITHUB_SHA), foreign identity inside: red (rc 1)" test $? -eq 1
sed 's/ && \[ "\$(g rev-parse HEAD)" = "\${GITHUB_SHA:-}" \]//' "$tool" >"$tmp/mutsha"; cmp -s "$tool" "$tmp/mutsha" && chk "M1 mutant: the GITHUB_SHA condition was found in the tool" false
env CI=true GITHUB_EVENT_NAME=pull_request GITHUB_SHA=0000000000000000000000000000000000000000 bash "$tmp/mutsha" "$tmp/h" >/dev/null 2>&1; chk "M1 mutant (no GITHUB_SHA condition): the merge-head case turns green (the test sees it)" test $? -eq 0
# --- Decision 9f (first check): no origin/main ON THE REMOTE = check ALL commits of the branch and say so; a fetch or network error is NOT "no main" ---
# Threat model: the gate protects against MISTAKES, not against someone who can change workflow, toolchain or scripts.
fx() { # fx <dir> <remote-state: none|main|broken>: clone-like repo with origin; commits: root(OK) + one
  rm -rf "${tmp:?}/$1" "${tmp:?}/$1.git"; git init -q --bare -b main "$tmp/$1.git"; git init -q -b feat "$tmp/$1"; git -C "$tmp/$1" remote add origin "$tmp/$1.git"
  [ "$2" = broken ] && git -C "$tmp/$1" remote set-url origin "$tmp/$1-does-not-exist.git"
  r="$tmp/$1"; echo root >"$r/root"; git -C "$r" add . && GIT_AUTHOR_NAME=a GIT_AUTHOR_EMAIL=$OK GIT_COMMITTER_NAME=a GIT_COMMITTER_EMAIL=$OK git -C "$r" -c commit.gpgsign=false commit -q -m root
  if [ "$2" = main ]; then git -C "$r" push -q origin feat:main; git -C "$r" fetch -q origin; fi
  fc() { echo "$2" >"$r/$2"; git -C "$r" add . && GIT_AUTHOR_NAME=a GIT_AUTHOR_EMAIL="$1" GIT_COMMITTER_NAME=a GIT_COMMITTER_EMAIL="$1" git -C "$r" -c commit.gpgsign=false commit -q -m "$2"; }
}
erun() { env CI=true GITHUB_ACTIONS=true bash "${2:-$tool}" "$tmp/$1" >"$tmp/o" 2>&1; rc=$?; }
# (a) main exists: as before (range main..HEAD; the stock before main is not looked at)
fx fa main; fc "$BAD" late; erun fa; chk "9f (a) main exists, foreign commit after it: red (rc 1), no first check" test $rc -eq 1 -a -z "$(grep -F 'Erstpruefung' "$tmp/o")"
git -C "$tmp/fa" reset -q --hard HEAD~1; fc $OK fine; erun fa; chk "9f (a) main exists, clean range: green, no first check" test $rc -eq 0 -a -z "$(grep -F 'Erstpruefung' "$tmp/o")"
# (b) no main on the remote, all commits FreeSynergy: green and visible
fx fb none; fc $OK two; erun fb; chk "9f (b) no main, all commits allowed: green (rc 0)" test $rc -eq 0
chk "9f (b) the message 'kein main: Erstpruefung' is shown" test -n "$(grep -F 'kein main: Erstpruefung' "$tmp/o")"
chk "9f (b) all commits are counted (2)" test -n "$(grep -F 'identity ok: 2 commit(s)' "$tmp/o")"
env -u CI -u GITHUB_ACTIONS bash "$tool" "$tmp/fb" >"$tmp/o" 2>&1; chk "9f (b) local (no CI): the same, green" test $? -eq 0
# (c) no main, ONE foreign commit in the middle: red, commit named, address not
fx fc none; fc "$BAD" mid; fc $OK three; erun fc; chk "9f (c) no main, foreign commit in the middle: red (rc 1)" test $rc -eq 1 -a -n "$(grep "$(git -C "$tmp/fc" rev-parse --short HEAD~1)" "$tmp/o")"
chk "9f (c) the address is never printed" test -z "$(grep -F "$BAD" "$tmp/o")"
fx fc2 none; GIT_AUTHOR_EMAIL=$BAD GIT_AUTHOR_NAME=a GIT_COMMITTER_EMAIL=$BAD GIT_COMMITTER_NAME=a git -C "$tmp/fc2" -c commit.gpgsign=false commit -q --amend --no-edit --reset-author; erun fc2; chk "9f (c) the foreign identity sits in the ROOT commit: red" test $rc -eq 1
# (d) fetch error (the remote cannot be asked): red rc 2, NOT a first check
fx fd broken; erun fd; chk "9f (d) remote not reachable: red (rc 2)" test $rc -eq 2
chk "9f (d) not treated as a first check" test -z "$(grep -F 'Erstpruefung' "$tmp/o")"
fx fe none; git -C "$tmp/fe" remote remove origin; erun fe; chk "9f (d) no remote at all: red (rc 2), never skipped" test $rc -eq 2
# main exists on the remote but was not fetched: red, never a first check
fx ff main; git -C "$tmp/ff" update-ref -d refs/remotes/origin/main; erun ff; chk "9f main exists on the remote but is not fetched: red (rc 2)" test $rc -eq 2 -a -z "$(grep -F 'Erstpruefung' "$tmp/o")"
# an explicit base that does not exist stays red (rc 2), also without main
bash "$tool" "$tmp/fb" nonexistent >/dev/null 2>&1; chk "9f explicit missing base: rc 2 as before" test $? -eq 2
# shallow clone without main: the root commit is not known: red
git clone -q --depth 1 "file://$tmp/fb" "$tmp/fs" 2>/dev/null; erun fs; chk "9f shallow clone without main: red (rc 2)" test $rc -eq 2
# mutants: (1) the check of the first-check range removed (range empty) -> (c) turns green; (2) any ls-remote error counted as "no main" -> (d) turns green
sed 's/^  range_base=""$/  range_base="HEAD.."/' "$tool" >"$tmp/mut9f1"; cmp -s "$tool" "$tmp/mut9f1" && chk "9f mutant 1 was applied" false
erun fc "$tmp/mut9f1"; chk "9f mutant 1 (first-check range empty): case (c) turns green (the test sees it)" test $rc -eq 0
sed 's/"\$lrc" -ne 2/"$lrc" -eq 99/' "$tool" >"$tmp/mut9f2"; cmp -s "$tool" "$tmp/mut9f2" && chk "9f mutant 2 was applied" false
erun fd "$tmp/mut9f2"; chk "9f mutant 2 (ls-remote error = no main): case (d) turns green (the test sees it)" test $rc -eq 0
if [ "$fails" -eq 0 ]; then echo "all ok"; else echo "$fails FAILED"; exit 1; fi
