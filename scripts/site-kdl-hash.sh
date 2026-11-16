#!/usr/bin/env bash
# Pattern: none. Hash of the Zentinel configuration a signed site tarball leads to (CI step "Hash of the Zentinel configuration", and what the person compares).
#   ACME_EMAIL=<contact> [WEBROOT=...] [HTTP_ADDR=...] [HTTPS_ADDR=...] site-kdl-hash.sh <name-version-site.tar.gz> [--allow-passthrough]
# Reads ONLY ./site.toml out of the tarball (nothing else is unpacked) and renders staging, production and http-only with templates/site/render.py, then prints
#   kdl-inputs webroot=<..> http-addr=<..> https-addr=<..> acme-email=<..>      the values the hashes depend on (the defaults when a variable is unset)
#   kdl-sha256 <variant> <hash>                                                  per variant
# The hash depends on site.toml AND on these four inputs: the person must render with the SAME values (the defaults below when nothing is set) or the hashes never match.
# Defaults (as in render.py): WEBROOT=/srv/www  HTTP_ADDR=0.0.0.0:80  HTTPS_ADDR=0.0.0.0:443; ACME_EMAIL has none (the repository variable SITE_ACME_EMAIL in CI).
set -euo pipefail
here=$(cd "$(dirname "$0")" && pwd)
tarball=${1:?usage: site-kdl-hash.sh <site tarball> [--allow-passthrough]}; shift
opt=(); case ${1:-} in "") ;; --allow-passthrough) opt=(--allow-passthrough) ;; *) echo "usage: site-kdl-hash.sh <site tarball> [--allow-passthrough]" >&2; exit 2 ;; esac
[ -n "${ACME_EMAIL:-}" ] || { echo "ACME_EMAIL is not set (repository variable SITE_ACME_EMAIL): no configuration hash" >&2; exit 1; }
export WEBROOT=${WEBROOT:-/srv/www} HTTP_ADDR=${HTTP_ADDR:-0.0.0.0:80} HTTPS_ADDR=${HTTPS_ADDR:-0.0.0.0:443}
w=$(mktemp -d); trap 'rm -rf "$w"' EXIT
tar -xzf "$tarball" -C "$w" --no-same-owner --no-same-permissions ./site.toml
echo "kdl-inputs webroot=$WEBROOT http-addr=$HTTP_ADDR https-addr=$HTTPS_ADDR acme-email=$ACME_EMAIL"
for v in staging production http-only; do
  python3 -I "$here/../templates/site/render.py" "--$v" "${opt[@]}" --site "$w/site.toml" "$w/$v.kdl" >/dev/null 2>&1 || { echo "render --$v failed" >&2; exit 1; }
  echo "kdl-sha256 $v $(sha256sum "$w/$v.kdl" | cut -d' ' -f1)"
done
