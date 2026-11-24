#!/usr/bin/env bash
# Create (or refresh) a DRAFT release. Nothing here ever publishes it.
set -euo pipefail
: "${TAG:?}" "${REPO:?}" "${DIST:?}" "${GH_TOKEN:?}" "${KIND:?}"
export GH_REPO=$REPO

footer=$(mktemp)
trap 'rm -f "$footer"' EXIT
# shellcheck disable=SC2016  # envsubst wants the literal ${REPO}
REPO=$REPO envsubst '${REPO}' < "$(dirname "${BASH_SOURCE[0]}")/footer.md" > "$footer"
# the SBOM line stays only when an SBOM is among the files AND the kind makes one; data and module say "none" (never silent).
# A footer without the line to replace is an error, not a silent no-op.
if [[ "$KIND" == data || "$KIND" == module ]] || ! compgen -G "$DIST/*.spdx.json" > /dev/null; then
  sed -i "s|^- SBOM: .*|- SBOM: none for this release (kind $KIND: no exact package-level SBOM yet)|" "$footer"
  grep -qxF -- "- SBOM: none for this release (kind $KIND: no exact package-level SBOM yet)" "$footer" || { echo "::error::footer.md has no '- SBOM:' line to replace"; exit 1; }
fi

if state=$(gh release view "$TAG" --json isDraft --jq .isDraft 2>/dev/null); then
  # A published release is never touched again: replacing signed files under a public tag would break everything that
  # already trusts them. Publish a new tag instead.
  if [[ "$state" != "true" ]]; then
    printf '::error::%s %s is already published - refusing to replace its assets (use a new tag)\n' "$REPO" "$TAG"
    exit 1
  fi
  echo "Draft $TAG exists - replacing its assets"
  gh release upload "$TAG" "$DIST"/* --clobber
else
  args=(--draft --verify-tag --title "${NAME:-$REPO} $TAG" --generate-notes --notes-file "$footer")
  [[ "${PRERELEASE:-false}" == "true" ]] && args+=(--prerelease)
  gh release create "$TAG" "${args[@]}" "$DIST"/*
fi
