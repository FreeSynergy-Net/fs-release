#!/usr/bin/env bash
# Library release: a DRAFT with the notes of the CHANGELOG section, then (not in dry-run) published as Pre-release (0.0.N is always a
# pre-release, ADR 0064). No artefacts, no signatures. Never touches a release that already exists.
set -euo pipefail
: "${TAG:?}" "${REPO:?}" "${GH_TOKEN:?}" "${NOTES:?}"
export GH_REPO=$REPO
dry=${DRY_RUN:-false}
name=${NAME:-${REPO##*/}}

if gh release view "$TAG" >/dev/null 2>&1; then
  printf '::error::%s %s: a release with this tag exists - refusing to touch it (use a new tag)\n' "$REPO" "$TAG"
  exit 1
fi
if [[ "$dry" == "true" ]]; then
  echo "dry-run: would create the draft $TAG with $(wc -l <"$NOTES") lines of notes and publish it as Pre-release"
  exit 0
fi
gh release create "$TAG" --draft --prerelease --verify-tag --title "$name $TAG" --notes-file "$NOTES"
gh release edit "$TAG" --draft=false --prerelease
