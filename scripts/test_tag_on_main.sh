#!/usr/bin/env bash
# Self test of actions/tag-on-main/check.sh (a binary release only from a commit of main) with counter-checks on fixture clones, and of its use:
# the plan action and the finalize job call it, and both checkouts are full (fetch-depth 0). Mutations on copies (the ancestor check removed,
# the call removed from finalize, fetch-depth removed from the plan job) must turn the tests red.
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

# use: plan action, finalize job, full checkouts
used() { # <repo root> <label>
  local r="$1" l="$2"
  grep -q 'tag-on-main/check.sh' "$r/actions/plan/action.yml" && ok "$l: plan action runs the check" || bad "$l: plan action does not run the check"
  grep -q 'actions/tag-on-main@' "$r/.github/workflows/_release-finalize.yml" && ok "$l: finalize runs the check" || bad "$l: finalize does not run the check"
  python3 - "$r" <<'PY' && ok "$l: plan and finalize checkouts are full" || bad "$l: a checkout is shallow"
import sys, yaml
r = sys.argv[1]
for f, job in ((".github/workflows/release.yml", "plan"), (".github/workflows/_release-finalize.yml", "finalize")):
    steps = yaml.safe_load(open(f"{r}/{f}"))["jobs"][job]["steps"]
    co = [s for s in steps if str(s.get("uses", "")).startswith("actions/checkout@")][0]
    assert co["with"].get("fetch-depth") == 0, (f, co)
PY
}
python3 -c "import yaml" 2>/dev/null || { echo "skipped: no PyYAML (the structure was NOT checked)"; exit 2; }
used "$repo" real
cp -r "$repo/.github" "$repo/actions" "$tmp/mut/"
sed -i '/tag-on-main@/d' "$tmp/mut/.github/workflows/_release-finalize.yml"
sed -i '/fetch-depth: 0/d' "$tmp/mut/.github/workflows/release.yml"
out="$(used "$tmp/mut" mut)"; [ "$(grep -c '^FAIL' <<<"$out")" -ge 2 ] && ok "mutations (call removed, fetch-depth removed) are caught" || bad "mutations not caught: $out"
[ "$fails" -eq 0 ] && echo "all ok" || { echo "$fails failed"; exit 1; }
