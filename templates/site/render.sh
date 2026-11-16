#!/usr/bin/env bash
# Renders the Zentinel configuration for the three stage-0 sites. It installs NOTHING and copies no pages: the pages come only through the signed
# artefact (site-bauen -> site-signieren -> the person delivers into <webroot>/<domain>/releases/<version> and swaps the `current` link).
#   SITE1=freesynergy.net SITE2=helfa.org SITE3=freeplanetearth.org ACME_EMAIL=<contact> \
#     render.sh [--staging|--production|--http-only|--test] <output.kdl>
# --staging (default): Let's Encrypt staging. --production: only after a clean staging run (see README). --http-only: first start, plain HTTP, no ACME.
# --test: local variant without TLS/ACME (test-podman.sh).
# Environment: SITE1..3, ACME_EMAIL (not needed with --http-only/--test), WEBROOT (default /srv/www; the roots are <WEBROOT>/<domain>/current),
#   HTTP_ADDR (default 0.0.0.0:80), HTTPS_ADDR (default 0.0.0.0:443),
#   CSP_MODE1..3 = static (default: no script, nothing from outside) | passthrough (own scripts of the site: script-src 'self', connect-src 'self'),
#   CSP1..3 = a complete Content-Security-Policy that replaces the mode's preset (strictly checked characters).
# Also writes <output.kdl>.sha256: the configuration is a file a person reads and approves, and its hash is what the person records and compares on
# the server (it is not part of the signed tarball: it holds server values and changes rarely). Same input, same bytes.
# No secrets are read or written. Values are checked strictly because they end up inside a KDL file.
set -euo pipefail
here=$(cd "$(dirname "$0")" && pwd)
die() { echo "error: $*" >&2; exit 2; }
staging=true; out=""; tpl=zentinel-static.kdl.in; variant=acme
for a in "$@"; do
  case $a in
    --staging) staging=true; tpl=zentinel-static.kdl.in; variant=acme ;;
    --production) staging=false; tpl=zentinel-static.kdl.in; variant=acme ;;
    --http-only) tpl=zentinel-static-http-only.kdl.in; variant=plain ;;
    --test) tpl=zentinel-static-test.kdl.in; variant=plain ;;
    -*) die "unknown option $a" ;;
    *) [ -z "$out" ] || die "only one output file"; out=$a ;;
  esac
done
[ -n "$out" ] || die "usage: render.sh [--staging|--production|--http-only|--test] <output.kdl>"
: "${SITE1:?SITE1 is not set}" "${SITE2:?SITE2 is not set}" "${SITE3:?SITE3 is not set}"
email=${ACME_EMAIL:-}
if [ "$variant" = acme ]; then : "${ACME_EMAIL:?ACME_EMAIL is not set}"; else email=test@example.org; fi
webroot=${WEBROOT:-/srv/www}; http_addr=${HTTP_ADDR:-0.0.0.0:80}; https_addr=${HTTPS_ADDR:-0.0.0.0:443}
dom='^([a-z0-9]([a-z0-9-]{0,61}[a-z0-9])?\.)+[a-z]{2,63}$'
for s in "$SITE1" "$SITE2" "$SITE3"; do [[ $s =~ $dom ]] || die "'$s' is not a plain lower-case domain name"; done
if [ "$SITE1" = "$SITE2" ] || [ "$SITE1" = "$SITE3" ] || [ "$SITE2" = "$SITE3" ]; then die "the three domains must differ (one domain, one acme block)"; fi
[[ $email =~ ^[A-Za-z0-9._+-]+@([a-z0-9-]+\.)+[a-z]{2,63}$ ]] || die "ACME_EMAIL is not a plain e-mail address"
if ! { [[ $webroot =~ ^/[A-Za-z0-9._/-]*$ ]] && [[ $webroot != */ ]] && [[ $webroot != *..* ]]; }; then die "WEBROOT must be an absolute path of plain characters, without trailing slash"; fi
addr='^[0-9]{1,3}(\.[0-9]{1,3}){3}:[0-9]{1,5}$'
[[ $http_addr =~ $addr ]] || die "HTTP_ADDR must look like 0.0.0.0:80"
[[ $https_addr =~ $addr ]] || die "HTTPS_ADDR must look like 0.0.0.0:443"
[ "$http_addr" != "$https_addr" ] || die "HTTP_ADDR and HTTPS_ADDR must differ"

# Content-Security-Policy per site: a setting with safe defaults. The presets are the only values nobody has to think about.
csp_static="default-src 'none'; img-src 'self'; style-src 'self' 'unsafe-inline'; font-src 'self'; base-uri 'none'; form-action 'none'; frame-ancestors 'none'"
csp_pass="default-src 'self'; script-src 'self'; style-src 'self' 'unsafe-inline'; img-src 'self' data:; font-src 'self'; connect-src 'self'; object-src 'none'; base-uri 'none'; form-action 'self'; frame-ancestors 'none'"
cspv=()
for n in 1 2 3; do
  m=CSP_MODE$n; c=CSP$n; mode=${!m:-static}; custom=${!c:-}
  case $mode in static) v=$csp_static ;; passthrough) v=$csp_pass ;; *) die "$m must be static or passthrough" ;; esac
  if [ -n "$custom" ]; then
    [[ $custom =~ ^[A-Za-z0-9\ \;:\'*._/=+,%-]{1,1000}$ ]] || die "$c holds a character that is not allowed in a policy (no double quote, backslash, @, & or line break)"
    v=$custom
  fi
  cspv+=("$v")
  if [ "$mode" = passthrough ]; then
    echo "WARNING: site $n is in passthrough mode (its own scripts run). Zentinel 26.11_2 does NOT send route response-headers for static-files routes (measured by test-podman.sh), so the CSP set here is written into the configuration but not delivered yet. Do not go live with passthrough pages before a version that sends it (or another wall) is proven." >&2
  fi
done

text=$(cat "$here/$tpl")
for pair in "SITE1=$SITE1" "SITE2=$SITE2" "SITE3=$SITE3" "ACME_EMAIL=$email" "ACME_STAGING=$staging" "WEBROOT=$webroot" "HTTP_ADDR=$http_addr" "HTTPS_ADDR=$https_addr"; do
  text=${text//"@${pair%%=*}@"/${pair#*=}}
done
for n in 1 2 3; do text=${text//"@CSP$n@"/${cspv[$((n - 1))]}}; done
if grep -q -E '@[A-Z0-9_]+@' <<<"$text"; then
  # the plain templates have no ACME markers, the production one has them all replaced; anything left is a mistake
  die "unreplaced marker left in the template"
fi
printf '%s\n' "$text" > "$out"
(cd "$(dirname "$out")" && sha256sum -- "$(basename "$out")" > "$(basename "$out").sha256")
echo "wrote $out and $out.sha256 (variant=$variant, staging=$staging, webroot=$webroot)"
