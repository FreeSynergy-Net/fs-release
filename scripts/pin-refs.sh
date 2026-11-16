#!/usr/bin/env bash
# Pin all internal fs-release references (workflows and composite actions) to one ref (a full commit SHA, or a tag).
# Usage: scripts/pin-refs.sh <40-char-sha>   (the SHA of a reviewed state of main, a fast-forward commit, not a merge commit;
# a tag also works). Only `uses:` lines are rewritten, comments and docs are left alone.
# GitHub cannot resolve "same ref as the caller" for nested reusable workflows and composite
# actions across repositories, so the references are explicit and rewritten at release time.
#
# Option `--dir <repo>` (before or after the ref): work in ANOTHER repository (a program or library that calls fs-release).
# Only `<repo>/.github/workflows/*.yml` is touched, only `uses: FreeSynergy/fs-release/<path>@<old>` becomes `@<ref>`; other
# actions (actions/checkout@..., ...) stay as they are. No `.github/workflows/` there = rc 2. Prints the changed files, rc 0.
# Without `--dir` the script rewrites this repository (workflows and actions) and prints `git diff --stat`, as before.
set -euo pipefail
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

pin_file() {
  sed -i -E "/^[[:space:]]*(-[[:space:]]+)?uses:/ s#(FreeSynergy/fs-release/[A-Za-z0-9_./-]+)@[A-Za-z0-9_.-]+#\1@${ref}#g" "$1"
}

if [ -n "$dir" ]; then
  wf="$dir/.github/workflows"
  [ -d "$wf" ] || { echo "pin-refs: no .github/workflows in '$dir' (is it a repository?)" >&2; exit 2; }
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
grep -rlE 'FreeSynergy/fs-release/.+@' .github actions | while read -r f; do
  pin_file "$f"
done
git diff --stat
