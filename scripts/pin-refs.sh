#!/usr/bin/env bash
# Pin all internal fs-release references (workflows and composite actions) to one ref (a full commit SHA, or a tag).
# Usage: scripts/pin-refs.sh <40-char-sha>   (the SHA of a reviewed state of main, a fast-forward commit, not a merge commit;
# a tag also works). Only `uses:` lines are rewritten, comments and docs are left alone.
# GitHub cannot resolve "same ref as the caller" for nested reusable workflows and composite
# actions across repositories, so the references are explicit and rewritten at release time.
set -euo pipefail
ref=${1:?usage: pin-refs.sh <sha-or-tag>}
cd "$(dirname "${BASH_SOURCE[0]}")/.."
grep -rlE 'FreeSynergy/fs-release/.+@' .github actions | while read -r f; do
  sed -i -E "/^[[:space:]]*(-[[:space:]]+)?uses:/ s#(FreeSynergy/fs-release/[A-Za-z0-9_./-]+)@[A-Za-z0-9_.-]+#\1@${ref}#g" "$f"
done
git diff --stat
