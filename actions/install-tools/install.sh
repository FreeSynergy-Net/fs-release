#!/usr/bin/env bash
# Pattern: none. Installs the cargo tools named in $TOOLS with cargo-binstall (already installed by the caller step).
# Shared by actions/install-tools and (through a path, not a `uses: ...@main`) by actions/sbom and actions/licence-check,
# so a pinned action never pulls a later main state of another action.
set -euo pipefail
# every tool is pinned to an exact version (name@x.y.z): what the build job installs is signed afterwards
for t in $TOOLS; do
  [[ $t =~ ^[a-z0-9_-]+@[0-9]+\.[0-9]+\.[0-9]+$ ]] || { echo "::error::install-tools: '$t' is not name@x.y.z (an exact version)"; exit 1; }
done
# shellcheck disable=SC2086  # word splitting of the tool list is intended
cargo binstall --no-confirm --locked $TOOLS
