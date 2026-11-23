#!/usr/bin/env bash
# Self test of actions/tag-on-main/check.sh (a binary release only from a commit of main): a table of cases on fixture clones and one mutation (no ancestor check).
# Its use in plan and finalize is checked in crates/fs-release/tests/ci_tools_pinned.rs.   bash scripts/test_tag_on_main.sh
set -uo pipefail
repo="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"; tmp="$(mktemp -d)"; trap 'rm -rf "$tmp"' EXIT; fails=0
g() { git -C "$tmp/clone" -c user.name=t -c user.email=t@example.invalid "$@"; }
git init -q "$tmp/clone"; g commit -q --allow-empty -m one; main="$(g rev-parse HEAD)"; g update-ref refs/remotes/origin/main "$main"
g commit -q --allow-empty -m side; side="$(g rev-parse HEAD)"; g tag v-main "$main"; g tag v-side "$side"
git clone -q --depth 1 "file://$tmp/clone" "$tmp/shallow" 2>/dev/null
# t <label> <script> <clone> <sha> <ref_type> <tag> <want rc> [text of the message]
t() { local rc; (cd "$tmp/$3" && GITHUB_SHA="$4" GITHUB_REF_TYPE="$5" TAG="$6" GITHUB_REF_NAME="" bash "$2" >"$tmp/out" 2>&1); rc=$?
  if [ "$rc" = "$7" ] && { [ -z "${8:-}" ] || grep -Fq -- "$8" "$tmp/out"; }; then echo "ok   $1"; else echo "FAIL $1 (rc $rc, want $7)"; fails=$((fails + 1)); fi; }
c="$repo/actions/tag-on-main/check.sh"
t "tag commit on main passes" "$c" clone "$main" tag v-main 0
t "tag commit off main is red" "$c" clone "$side" tag v-side 1 "not on main"
t "tag that points at another commit than GITHUB_SHA is red" "$c" clone "$main" tag v-side 1 "does not point at"
t "a shallow clone is refused" "$c" shallow "$main" tag v-main 1
t "a branch ref is not checked here" "$c" clone "$side" branch "" 0
mkdir "$tmp/mut"; sed 's/merge-base --is-ancestor/rev-parse/' "$c" >"$tmp/mut/check.sh"
t "mutation (no ancestor check): off-main passes, so the test is sharp" "$tmp/mut/check.sh" clone "$side" tag v-side 0
[ "$fails" -eq 0 ] && echo "all ok" || { echo "$fails failed"; exit 1; }
