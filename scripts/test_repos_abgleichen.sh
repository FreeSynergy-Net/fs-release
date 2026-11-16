#!/usr/bin/env bash
# Self test of scripts/repos-abgleichen with a local bare repo as origin: a branch that is deleted on origin after the first fetch is
# reported as `ref-verschwunden` and its origin/* ref stays (also with fetch.prune=true in the config); main is fast-forwarded.
#   bash scripts/test_repos_abgleichen.sh
set -uo pipefail
here="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
tmp="$(mktemp -d)"
trap 'rm -rf "$tmp"' EXIT
export GIT_AUTHOR_NAME=t GIT_AUTHOR_EMAIL=t@t GIT_COMMITTER_NAME=t GIT_COMMITTER_EMAIL=t@t
fails=0
ok() { echo "ok   $1"; }
bad() { echo "FAIL $1"; fails=$((fails + 1)); }

root="$tmp/root"
mkdir -p "$root"
git init -q --bare "$tmp/origin.git"
git clone -q "$tmp/origin.git" "$tmp/seed" 2>/dev/null
git -C "$tmp/seed" checkout -q -b main
git -C "$tmp/seed" commit -q --allow-empty -m one
git -C "$tmp/seed" branch feature
git -C "$tmp/seed" push -q origin main feature 2>/dev/null
git --git-dir="$tmp/origin.git" symbolic-ref HEAD refs/heads/main
git clone -q "$tmp/origin.git" "$root/fs-test" 2>/dev/null
git -C "$root/fs-test" config fetch.prune true

# origin: delete the branch, add a commit on main
git --git-dir="$tmp/origin.git" branch -q -D feature
git -C "$tmp/seed" commit -q --allow-empty -m two
git -C "$tmp/seed" push -q origin main 2>/dev/null

out="$("$here/repos-abgleichen" "$root" 2>&1)"
rc=$?
[ "$rc" -eq 0 ] && ok "rc 0" || bad "rc=$rc: $out"
grep -q '^fs-test|.*|ref-verschwunden|feature$' <<<"$out" && ok "vanished ref reported" || bad "no ref-verschwunden line: $out"
git -C "$root/fs-test" rev-parse --verify -q refs/remotes/origin/feature >/dev/null && ok "ref not deleted (no prune)" || bad "origin/feature was deleted"
grep -q '|ff|' <<<"$out" && ok "main fast-forwarded" || bad "no ff: $out"
[ "$fails" -eq 0 ] && echo "all ok" || { echo "$fails failure(s)"; exit 1; }
