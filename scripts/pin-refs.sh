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
# A pin is never lowered: with a tag as <ref>, a `uses:` line that is already on a HIGHER tag stays (checked line by line).
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

# One `uses:` line at a time (awk, POSIX): `FreeSynergy/fs-release/<path>@<old>` becomes `@<ref>`, everything else on the line and in the file stays.
# keep=1 (`--dir`, a foreign repo): a pin is never lowered. When <ref> is a tag, an `@<old>` that is a tag with a HIGHER version (v0.0.11 against
# v0.0.10) stays, line by line (release.yml@v0.0.11 stays while check.yml@v0.0.9 is raised). @main, a SHA, an older or equal tag are replaced.
pin_file() {
  local keep=$1 f=$2 tmpf="$2.pin-new"
  awk -v ref="$ref" -v keep="$keep" '
    function ver(t,   p) { if (t !~ /^v[0-9]+[.][0-9]+[.][0-9]+$/) return ""; split(substr(t, 2), p, "."); return sprintf("%09d%09d%09d", p[1], p[2], p[3]) }
    BEGIN { re = "FreeSynergy/fs-release/[A-Za-z0-9_./-]+@[A-Za-z0-9_.-]+" }
    $0 ~ "^[[:space:]]*(-[[:space:]]+)?uses:" {
      out = ""; rest = $0
      while (match(rest, re)) {
        tok = substr(rest, RSTART, RLENGTH); at = index(tok, "@"); old = substr(tok, at + 1)
        if (!(keep == 1 && ver(ref) != "" && ver(old) != "" && ver(old) > ver(ref))) tok = substr(tok, 1, at) ref
        out = out substr(rest, 1, RSTART - 1) tok; rest = substr(rest, RSTART + RLENGTH)
      }
      print out rest; next
    }
    { print }' "$f" >"$tmpf" && cat "$tmpf" >"$f"
  local rc=$?
  rm -f "$tmpf"
  return $rc
}

if [ -n "$dir" ]; then
  wf="$dir/.github/workflows"
  [ -d "$wf" ] || { echo "pin-refs: no .github/workflows in '$dir' (is it a repository?)" >&2; exit 2; }
  changed=0
  for f in "$wf"/*.yml; do
    [ -f "$f" ] || continue
    before=$(cksum <"$f")
    pin_file 1 "$f"
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
  pin_file 0 "$f"
done
git diff --stat
