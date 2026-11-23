#!/usr/bin/env bash
# A binary release is made only from a commit of main: the commit of the tag must be an ancestor of origin/main (as in library-release/release.sh).
# Needs a full clone (checkout with fetch-depth 0): a shallow clone cannot answer and is refused. Only a tag is checked (finalize accepts only tags anyway).
set -euo pipefail
: "${GITHUB_SHA:?}"
[ "${GITHUB_REF_TYPE:-}" = tag ] || { echo "tag-on-main: not a tag (ref_type=${GITHUB_REF_TYPE:-none}), nothing to check"; exit 0; }
if [ "$(git rev-parse --is-shallow-repository)" = true ]; then
  echo "::error::tag-on-main: the clone is shallow - the checkout needs fetch-depth: 0"
  exit 1
fi
if ! git merge-base --is-ancestor "$GITHUB_SHA" origin/main 2>/dev/null; then
  echo "::error::tag-on-main: the tag commit $GITHUB_SHA is not on main (or origin/main is missing): a release is made only from a commit of main"
  exit 1
fi
echo "tag-on-main: $GITHUB_SHA is on main"
