#!/usr/bin/env bash
# Pin all internal fs-release references (workflows and composite actions) to one ref (a full commit SHA, or a tag).
# Usage: scripts/pin-refs.sh <40-char-sha>   (the SHA of a reviewed state of main, a fast-forward commit, not a merge commit;
# a tag also works). Only `uses:` lines are rewritten, comments and docs are left alone.
# GitHub cannot resolve "same ref as the caller" for nested reusable workflows and composite
# actions across repositories, so the references are explicit and rewritten at release time.
#
# Option `--dir <repo>` (before or after the ref): work in ANOTHER repository (a program or library that calls fs-release).
# Only `<repo>/.github/workflows/*.yml` is touched, only `uses: <owner>/fs-release/<path>@<old>` (owner = one of the two known owners) becomes `@<ref>`; other
# actions (actions/checkout@..., ...) stay as they are. No `.github/workflows/` there = rc 2. Prints the changed files, rc 0.
# A pin is never lowered (rc 3, nothing written): every existing `@<old>` (tag or SHA) must resolve in the fs-release clone ($FS_RELEASE_REPO, default: this repo) and <ref> must be equal to it or a descendant (git merge-base --is-ancestor). Unknown, older or unrelated = rc 3. `@main` is raised. The owner name is rewritten to FreeSynergy-Net.
# Without `--dir` the script rewrites this repository (workflows and actions) and prints `git diff --stat`, as before.
set -euo pipefail
# The two known owners of fs-release, exactly (never "any owner"): the old one stays until part 2 of the address wave is done.
owners='FreeSynergy-Net|FreeSynergy'
usage="pin-refs: usage: pin-refs.sh [--dir <repo>] <40-char-sha or vX.Y.Z>"
ref=""
dir=""
while [ $# -gt 0 ]; do
  case $1 in
    --dir)
      [ $# -ge 2 ] || { echo "pin-refs: --dir needs a path. $usage" >&2; exit 2; }
      dir=$2
      shift 2
      ;;
    *)
      [ -z "$ref" ] || { echo "$usage (extra argument: $1)" >&2; exit 2; }
      ref=$1
      shift
      ;;
  esac
done
# The ref goes into a sed expression (`#` is the delimiter, `&` and flags are special): only a full SHA or vX.Y.Z is accepted.
[[ $ref =~ ^([0-9a-f]{40}|v[0-9]+\.[0-9]+\.[0-9]+)$ ]] || { echo "pin-refs: usage: pin-refs.sh [--dir <repo>] <40-char-sha or vX.Y.Z> (got: ${ref:-nothing})" >&2; exit 2; }

# One `uses:` line at a time (awk, POSIX): `<owner>/fs-release/<path>@<old>` becomes `@<ref>`, everything else on the line and in the file stays.
pin_file() {
  local f=$1 tmpf="$1.pin-new"
  awk -v ref="$ref" -v owners="$owners" '
    BEGIN { re = "(" owners ")/fs-release/[A-Za-z0-9_./-]+@[A-Za-z0-9_.-]+" }
    $0 ~ "^[[:space:]]*(-[[:space:]]+)?uses:" {
      out = ""; rest = $0
      while (match(rest, re)) {
        tok = substr(rest, RSTART, RLENGTH)
        sub(/^[A-Za-z-]+\//, "FreeSynergy-Net/", tok); tok = substr(tok, 1, index(tok, "@")) ref
        out = out substr(rest, 1, RSTART - 1) tok; rest = substr(rest, RSTART + RLENGTH)
      }
      print out rest; next
    }
    { print }' "$f" >"$tmpf" && cat "$tmpf" >"$f"
  local rc=$?
  rm -f "$tmpf"
  return $rc
}

# Never lower (rc 3, nothing written): every existing pin must equal <ref> or be an ancestor of it, checked in the fs-release clone.
check_no_lower() {
  local repo=${FS_RELEASE_REPO:-$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)} old n o rc=0
  n=$(git -C "$repo" rev-parse --verify -q "$ref^{commit}") || { echo "pin-refs: target $ref not found in $repo" >&2; return 3; }
  for old in $(cat "$@" | grep -E "^[[:space:]]*(-[[:space:]]+)?uses:" | grep -oE "($owners)/fs-release/[A-Za-z0-9_./-]+@[A-Za-z0-9_.-]+" | sed 's/.*@//' | sort -u); do
    if [ "$old" = main ] || [ "$old" = "$ref" ]; then continue; fi
    o=$(git -C "$repo" rev-parse --verify -q "$old^{commit}") || { echo "pin-refs: existing pin @$old not resolvable in $repo (not overwritten)" >&2; rc=3; continue; }
    [ "$o" = "$n" ] || git -C "$repo" merge-base --is-ancestor "$o" "$n" || { echo "pin-refs: $ref is older than or unrelated to the existing pin @$old (never lowered)" >&2; rc=3; }
  done
  return $rc
}

if [ -n "$dir" ]; then
  wf="$dir/.github/workflows"
  [ -d "$wf" ] || { echo "pin-refs: no .github/workflows in '$dir' (is it a repository?)" >&2; exit 2; }
  check_no_lower "$wf"/*.yml || exit 3
  changed=0
  for f in "$wf"/*.yml; do
    [ -f "$f" ] || continue
    before=$(cksum <"$f")
    pin_file "$f"
    if [ "$before" != "$(cksum <"$f")" ]; then
      echo "pin-refs: changed ${f}"
      changed=$((changed + 1))
    fi
  done
  echo "pin-refs: $changed file(s) changed in $dir (ref $ref)"
  exit 0
fi

cd "$(dirname "${BASH_SOURCE[0]}")/.."
grep -rlE "($owners)/fs-release/.+@" .github actions | while read -r f; do
  pin_file "$f"
done
git diff --stat
