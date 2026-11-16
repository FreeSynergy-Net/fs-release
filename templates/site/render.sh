#!/usr/bin/env bash
# Renders the Zentinel configuration from the site.toml files (the only place of the host names). Wrapper of render.py, see there and README.md.
#   ACME_EMAIL=<contact> render.sh [--staging|--production|--http-only|--test] --site <site.toml> [--site <site.toml> ...] <output.kdl>
# Installs nothing, copies no pages (pages only come through the signed artefact), writes <output.kdl>.sha256 (the file a person reads and records).
exec python3 -I "$(dirname "$0")/render.py" "$@"
