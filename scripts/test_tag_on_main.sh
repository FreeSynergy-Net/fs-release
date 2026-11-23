#!/usr/bin/env bash
# Self test of actions/tag-on-main/check.sh (a binary release only from a commit of main) with counter-checks on fixture clones and a mutation
# (the ancestor check removed). Its use in plan and finalize is checked in crates/fs-release/tests/ci_tools_pinned.rs.
#   bash scripts/test_tag_on_main.sh
set -uo pipefail
here="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
repo="$(cd "$here/.." && pwd)"
tmp="$(mktemp -d)"
trap 'rm -rf "$tmp"' EXIT
fails=0
ok() { echo "ok   $1"; }
bad() { echo "FAIL $1"; fails=$((fails + 1)); }
g() { git -C "$tmp/clone" -c user.name=t -c user.email=t@example.invalid "$@"; }

git init -q "$tmp/clone"; g commit -q --allow-empty -m one
main_sha="$(g rev-parse HEAD)"; g update-ref refs/remotes/origin/main "$main_sha"
g commit -q --allow-empty -m side; side_sha="$(g rev-parse HEAD)"
git clone -q --depth 1 "file://$tmp/clone" "$tmp/shallow" 2>/dev/null

# chk <script> <dir> <sha> <ref_type>: rc of the check in that clone
chk() { (cd "$2" && GITHUB_SHA="$3" GITHUB_REF_TYPE="$4" bash "$1" >"$tmp/out" 2>&1); echo $?; }
cases() { # <script> <label> <expect off-main: 1 or 0>
  local s="$1" l="$2" off="$3"
  [ "$(chk "$s" "$tmp/clone" "$main_sha" tag)" = 0 ] && ok "$l: tag commit on main passes" || bad "$l: tag commit on main refused"
  [ "$(chk "$s" "$tmp/clone" "$side_sha" tag)" = "$off" ] && ok "$l: tag commit off main rc $off" || bad "$l: tag commit off main gave the wrong rc"
  if [ "$off" = 1 ]; then grep -q "not on main" "$tmp/out" && ok "$l: the message names the cause" || bad "$l: no cause named"; fi
  [ "$(chk "$s" "$tmp/shallow" "$main_sha" tag)" = "$off" ] && ok "$l: a shallow clone is refused" || bad "$l: shallow clone accepted"
  [ "$(chk "$s" "$tmp/clone" "$side_sha" branch)" = 0 ] && ok "$l: a branch ref is not checked here" || bad "$l: branch ref was checked"
}
cases "$repo/actions/tag-on-main/check.sh" real 1
# mutation: the ancestor check removed
mkdir -p "$tmp/mut"; sed 's/merge-base --is-ancestor/rev-parse/' "$repo/actions/tag-on-main/check.sh" >"$tmp/mut/check.sh"
[ "$(chk "$tmp/mut/check.sh" "$tmp/clone" "$side_sha" tag)" = 0 ] && ok "mutation (no ancestor check): off-main passes, so the real test is sharp" || bad "mutation not recognised"

[ "$fails" -eq 0 ] && echo "all ok" || { echo "$fails failed"; exit 1; }
