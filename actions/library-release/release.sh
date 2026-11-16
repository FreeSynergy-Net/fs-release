#!/usr/bin/env bash
# Library release: a DRAFT with the notes of the CHANGELOG section, then (not in dry-run) published as Pre-release (0.0.N is always a
# pre-release, ADR 0064). No artefacts, no signatures. Never touches a release that already exists.
# The commit of the tag must be an ancestor of origin/main (needs a full clone: fetch-depth 0), as in the release job of fs-components.
# A DRAFT left over from an earlier run (the publish step failed after the draft was created) is recognised and named: publish it by hand or delete it.
set -euo pipefail
: "${TAG:?}" "${REPO:?}" "${GH_TOKEN:?}" "${NOTES:?}" "${GITHUB_SHA:?}"
export GH_REPO=$REPO
dry=${DRY_RUN:-false}
name=${NAME:-${REPO##*/}}

if ! git merge-base --is-ancestor "$GITHUB_SHA" origin/main 2>/dev/null; then
  printf '::error::%s %s: the tag commit %s is not on main (or the clone is shallow): a release is made only from a commit of main\n' "$REPO" "$TAG" "$GITHUB_SHA"
  exit 1
fi
if state=$(gh release view "$TAG" --json isDraft --jq .isDraft 2>/dev/null); then
  if [[ "$state" == "true" ]]; then
    printf '::error::%s %s: an unpublished DRAFT of this tag is left over from an earlier run - publish it by hand (gh release edit %s --draft=false --prerelease) or delete it (gh release delete %s) and run again\n' "$REPO" "$TAG" "$TAG" "$TAG"
  else
    printf '::error::%s %s: a release with this tag exists - refusing to touch it (use a new tag)\n' "$REPO" "$TAG"
  fi
  exit 1
fi
if [[ "$dry" == "true" ]]; then
  echo "dry-run: would create the draft $TAG with $(wc -l <"$NOTES") lines of notes and publish it as Pre-release"
  exit 0
fi
gh release create "$TAG" --draft --prerelease --verify-tag --title "$name $TAG" --notes-file "$NOTES"
gh release edit "$TAG" --draft=false --prerelease
