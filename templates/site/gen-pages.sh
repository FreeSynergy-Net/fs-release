#!/usr/bin/env bash
# Writes the neutral "we are rebuilding" pages of stage 0 (no script, no external resource, no tracking; lang attribute per page).
#   gen-pages.sh <output-dir>      writes <out>/sites/<domain>/{index,de/index,en/index}.html and <out>/port80/<domain>/index.html
# The committed folders sites/ and port80/ are the output of this script (test: test-site.sh compares them). Only domains of DOMAINS.
# TODO(user): real contents, imprint (Impressum) and privacy text come from the person who runs the sites; nothing is invented here.
set -euo pipefail
out=${1:?usage: gen-pages.sh <output-dir>}
DOMAINS=(freesynergy.net helfa.org freeplanetearth.org)

head() { # $1 lang, $2 title
  cat <<HTML
<!doctype html>
<html lang="$1">
<head>
<meta charset="utf-8">
<meta name="viewport" content="width=device-width, initial-scale=1">
<title>$2</title>
<meta name="robots" content="noindex">
<meta name="color-scheme" content="light dark">
<style>
  :root { color-scheme: light dark; }
  body { font: 1.125rem/1.6 system-ui, sans-serif; max-width: 40rem; margin: 0 auto; padding: 2rem 1rem; color: #1a1a1a; background: #ffffff; }
  h1 { font-size: 1.75rem; line-height: 1.25; margin: 0 0 .5rem; }
  a { color: #0b5cad; }
  hr { border: 0; border-top: 1px solid #767676; margin: 2rem 0; }
  @media (prefers-color-scheme: dark) { body { color: #f0f0f0; background: #121212; } a { color: #8ab4f8; } }
</style>
</head>
<body>
<main>
HTML
}
foot() { printf '</main>\n</body>\n</html>\n'; }

for d in "${DOMAINS[@]}"; do
  s="$out/sites/$d"; p="$out/port80/$d"
  mkdir -p "$s/de" "$s/en" "$p"
  {
    head de "$d"
    cat <<HTML
<!-- TODO(user): real contents, imprint and privacy text for $d; stage 0 shows only this neutral placeholder. -->
  <h1>$d</h1>
  <p lang="de">Diese Website wird gerade umgebaut. Bitte w&auml;hlen Sie eine Sprache:</p>
  <p lang="en">This website is being rebuilt. Please choose a language:</p>
  <p><a href="/de/" hreflang="de" lang="de">Deutsch</a> &middot; <a href="/en/" hreflang="en" lang="en">English</a></p>
HTML
    foot
  } > "$s/index.html"
  {
    head de "$d - Wir bauen um"
    cat <<HTML
<!-- TODO(user): real contents, imprint (Impressum) and privacy text for $d. -->
  <h1>Wir bauen um</h1>
  <p>Diese Website wird gerade neu aufgebaut. Im Moment finden Sie hier keine Inhalte.</p>
  <p>Bitte schauen Sie zu einem sp&auml;teren Zeitpunkt wieder vorbei.</p>
  <p><a href="/en/" hreflang="en" lang="en">English version</a></p>
HTML
    foot
  } > "$s/de/index.html"
  {
    head en "$d - We are rebuilding"
    cat <<HTML
<!-- TODO(user): real contents, imprint and privacy text for $d. -->
  <h1>We are rebuilding</h1>
  <p>This website is being rebuilt. There is no content here at the moment.</p>
  <p>Please check back at a later time.</p>
  <p><a href="/de/" hreflang="de" lang="de">Deutsche Fassung</a></p>
HTML
    foot
  } > "$s/en/index.html"
  {
    head en "$d"
    cat <<HTML
  <h1>$d</h1>
  <p lang="de">Weiter zur gesicherten Seite: <a href="https://$d/">https://$d/</a></p>
  <p lang="en">Continue to the secure site: <a href="https://$d/">https://$d/</a></p>
HTML
    foot
  } | sed "s#<meta name=\"robots\" content=\"noindex\">#<meta name=\"robots\" content=\"noindex\">\n<meta http-equiv=\"refresh\" content=\"0;url=https://$d/\">#" > "$p/index.html"
done
