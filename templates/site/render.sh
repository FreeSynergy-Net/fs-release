#!/usr/bin/env bash
# Renders the Zentinel configuration for the three stage-0 sites and installs the pages into the document roots.
#   SITE1=freesynergy.net SITE2=helfa.org SITE3=freeplanetearth.org ACME_EMAIL=<contact> \
#     render.sh [--staging|--production|--test] [--install-pages] <output.kdl>
# --staging (default): Let's Encrypt staging. --production: only after a clean staging run (see README). --test: local variant without TLS/ACME.
# Environment: SITE1..3, ACME_EMAIL (not needed with --test), WEBROOT (default /srv/www), HTTP_ADDR (default 0.0.0.0:80), HTTPS_ADDR (default 0.0.0.0:443).
# With --install-pages the folder sites/<domain> goes to $WEBROOT/<domain> and port80/<domain> to $WEBROOT/_port80/<domain>.
# No secrets are read or written. Values are checked strictly because they end up inside a KDL file.
set -euo pipefail
here=$(cd "$(dirname "$0")" && pwd)
die() { echo "error: $*" >&2; exit 2; }
staging=true; install=0; out=""; tpl=zentinel-static.kdl.in; test_mode=0
for a in "$@"; do
  case $a in
    --staging) staging=true ;;
    --production) staging=false ;;
    --test) tpl=zentinel-static-test.kdl.in; test_mode=1 ;;
    --install-pages) install=1 ;;
    -*) die "unknown option $a" ;;
    *) [ -z "$out" ] || die "only one output file"; out=$a ;;
  esac
done
[ -n "$out" ] || die "usage: render.sh [--staging|--production|--test] [--install-pages] <output.kdl>"
: "${SITE1:?SITE1 is not set}" "${SITE2:?SITE2 is not set}" "${SITE3:?SITE3 is not set}"
email=${ACME_EMAIL:-}
if [ "$test_mode" = 0 ]; then : "${ACME_EMAIL:?ACME_EMAIL is not set}"; else email=test@example.org; fi
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
text=$(cat "$here/$tpl")
for pair in "SITE1=$SITE1" "SITE2=$SITE2" "SITE3=$SITE3" "ACME_EMAIL=$email" "ACME_STAGING=$staging" "WEBROOT=$webroot" "HTTP_ADDR=$http_addr" "HTTPS_ADDR=$https_addr"; do
  text=${text//"@${pair%%=*}@"/${pair#*=}}
done
if grep -q -E '@[A-Z0-9_]+@' <<<"$text"; then
  # the test template has no ACME markers, the production one has them all replaced; anything left is a mistake
  die "unreplaced marker left in the template"
fi
printf '%s\n' "$text" > "$out"
echo "wrote $out (staging=$staging, test=$test_mode, webroot=$webroot)"
if [ "$install" = 1 ]; then
  for s in "$SITE1" "$SITE2" "$SITE3"; do
    [ -d "$here/sites/$s" ] || die "no page folder for $s (sites/$s): the pages exist only for the three stage-0 domains"
    install -d -m 0755 "$webroot/$s/de" "$webroot/$s/en" "$webroot/_port80/$s"
    for f in index.html de/index.html en/index.html; do install -m 0644 "$here/sites/$s/$f" "$webroot/$s/$f"; done
    install -m 0644 "$here/port80/$s/index.html" "$webroot/_port80/$s/index.html"
    echo "pages installed: $webroot/$s and $webroot/_port80/$s"
  done
fi
