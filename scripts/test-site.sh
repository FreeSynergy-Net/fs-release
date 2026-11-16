#!/usr/bin/env bash
# Selftest of site-bauen.sh, site-freigabe-pruefen.sh, site-signieren.sh and the stage-0 templates. Own temporary keys, nothing else touched.
# Every rule has a counter-test: the thing must turn RED when the rule is broken. Exit 0 only when all pass.
# shellcheck disable=SC2015,SC2329  # "A && ok || bad": ok and bad never fail; helper functions are used through red/grn
set -uo pipefail
here=$(cd "$(dirname "$0")" && pwd); root=$(cd "$here/.." && pwd)
t=$(mktemp -d); export GNUPGHOME="$t/gh"; mkdir -m 700 "$GNUPGHOME"; trap 'gpgconf --kill gpg-agent >/dev/null 2>&1; rm -rf "$t"' EXIT
fail=0; ok() { echo "ok   $1"; }; bad() { echo "FAIL $1"; fail=1; }
red() { local n=$1; shift; if "$@" >"$t/o" 2>&1; then bad "$n (was green)"; else ok "$n"; [ -z "${VERBOSE:-}" ] || sed "s/^/       /" "$t/o" | grep -E "RED|error|:" | head -2; fi; }
grn() { local n=$1; shift; if "$@" >"$t/o" 2>&1; then ok "$n"; else bad "$n"; sed 's/^/     /' "$t/o"; fi; }

# --- keys: approver (human), release (CI), stranger
mk() { gpg --batch --quiet --passphrase '' --quick-generate-key "$1" ed25519 sign 1d >/dev/null 2>&1; gpg --batch --list-keys --with-colons "$1" | awk -F: '/^fpr:/{print $10; exit}'; }
appr=$(mk "approver@test"); rel=$(mk "release@test"); other=$(mk "stranger@test")
gpg --batch --export "$appr" > "$t/appr.pub"; gpg --batch --export "$other" > "$t/other.pub"
gpg --batch --armor --export "$rel" > "$t/rel.pub.asc"
gpg --batch --armor --export-secret-keys "$rel" > "$t/rel.sec.asc"
for k in appr other; do gpg --batch --no-default-keyring --keyring "$t/$k.ring" --import "$t/$k.pub" 2>/dev/null; done
printf '%s\n' "# allowed approvers" "$appr" > "$t/allowed"

# --- the folder, built from the real stage-0 pages
mkdir -p "$t/src/de"; cp "$root/templates/site/sites/helfa.org/index.html" "$t/src/"; cp "$root/templates/site/sites/helfa.org/de/index.html" "$t/src/de/"
grn "build: good folder" "$here/site-bauen.sh" build "$t/src" "$t/dist" helfa.org 0.0.1
tree=$(grep '^tree-sha256=' "$t/o" | cut -d= -f2)
[ -n "$tree" ] && ok "build prints the tree hash" || bad "build prints the tree hash"
grn "build: second build into another folder is byte-identical (same hash)" bash -c "'$here/site-bauen.sh' build '$t/src' '$t/dist2' helfa.org 0.0.1 && cmp '$t/dist/helfa.org-0.0.1-site.tar.gz' '$t/dist2/helfa.org-0.0.1-site.tar.gz'"
red "build: non-empty outdir refused" "$here/site-bauen.sh" build "$t/src" "$t/dist" helfa.org 0.0.1
for case in "script|<script>1</script>" "external|<img src=\"https://x.test/a.png\">" "secret|password = abc" "private-key|-----BEGIN PRIVATE KEY-----"; do
  n=${case%%|*}; cp -r "$t/src" "$t/bad-$n"; printf '%s\n' "${case#*|}" >> "$t/bad-$n/index.html"
  red "build: $n in the folder is refused" "$here/site-bauen.sh" build "$t/bad-$n" "$t/o-$n" helfa.org 0.0.1
done
cp -r "$t/src" "$t/bad-link"; ln -s /etc/passwd "$t/bad-link/x.txt"; red "build: symlink refused" "$here/site-bauen.sh" check "$t/bad-link"
cp -r "$t/src" "$t/bad-name"; : > "$t/bad-name/a b.html"; red "build: unsafe file name refused" "$here/site-bauen.sh" check "$t/bad-name"
mkdir "$t/noindex"; echo x > "$t/noindex/a.html"; red "build: no index.html refused" "$here/site-bauen.sh" check "$t/noindex"
red "build: bad version refused" "$here/site-bauen.sh" build "$t/src" "$t/o-v" helfa.org "1;rm"

# --- the approval
exp=$(date -u -d '+24 hours' +%Y-%m-%dT%H:%M:%SZ)
approval() { # file tree run expires [gate]
  printf 'gate: %s\nname: helfa.org\nversion: 0.0.1\ntree-sha256: %s\nrun-id: %s\nexpires: %s\n' "${5:-site-signieren}" "$2" "$3" "$4" > "$1"; }
signit() { gpg --batch --yes --local-user "$1" --armor --detach-sign --output "$2.asc" "$2" 2>/dev/null; }
approval "$t/a.txt" "$tree" run-0001 "$exp"; signit "$appr" "$t/a.txt"
check() { "$here/site-freigabe-pruefen.sh" "$t/dist" "$1" "$1.asc" "${2:-$t/appr.ring}" "${3:-$t/allowed}" "${4:-run-0001}" "${5:-}"; }
grn "approval: good one passes" check "$t/a.txt"
red "approval: other run id (replay) is red" check "$t/a.txt" "" "" run-0002
approval "$t/b.txt" "$(printf x | sha256sum | cut -d' ' -f1)" run-0001 "$exp"; signit "$appr" "$t/b.txt"; red "approval: for another hash is red" check "$t/b.txt"
approval "$t/c.txt" "$tree" run-0001 "$(date -u -d '-1 hour' +%Y-%m-%dT%H:%M:%SZ)"; signit "$appr" "$t/c.txt"; red "approval: expired is red" check "$t/c.txt"
approval "$t/d.txt" "$tree" run-0001 "$(date -u -d '+10 days' +%Y-%m-%dT%H:%M:%SZ)"; signit "$appr" "$t/d.txt"; red "approval: valid for more than 72 hours is red" check "$t/d.txt"
approval "$t/e.txt" "$tree" run-0001 "$exp"; signit "$other" "$t/e.txt"
red "approval: signed by a stranger (not on the list) is red" check "$t/e.txt" "$t/other.ring"
red "approval: stranger's key not in the keyring is red" check "$t/e.txt"
approval "$t/f.txt" "$tree" run-0001 "$exp" other-gate; signit "$appr" "$t/f.txt"; red "approval: another gate is red" check "$t/f.txt"
cp "$t/a.txt" "$t/g.txt"; cp "$t/a.txt.asc" "$t/g.txt.asc"; echo "extra: 1" >> "$t/g.txt"; red "approval: text changed after signing is red" check "$t/g.txt"
cp "$t/a.txt" "$t/h.txt"; : > "$t/h.txt.asc"; red "approval: empty signature file is red" check "$t/h.txt"
red "approval: missing approval file is red" check "$t/none.txt"
: > "$t/empty-allowed"; red "approval: empty allowed list is red" check "$t/a.txt" "" "$t/empty-allowed"
printf '%s\n' "not-a-fingerprint" > "$t/junk-allowed"; red "approval: junk in the allowed list is red" check "$t/a.txt" "" "$t/junk-allowed"
approval "$t/i.txt" "$tree" run-0001 "$exp"; echo "tree-sha256: $tree" >> "$t/i.txt"; signit "$appr" "$t/i.txt"; red "approval: key twice is red" check "$t/i.txt"
red "approval: unsigned text (a plain event) is red" "$here/site-freigabe-pruefen.sh" "$t/dist" "$t/a.txt" "$t/none.asc" "$t/appr.ring" "$t/allowed" run-0001
cp -r "$t/dist" "$t/dist-t"; echo tamper >> "$t/dist-t/helfa.org-0.0.1-site.tar.gz"
red "approval: tarball changed after the build is red" "$here/site-freigabe-pruefen.sh" "$t/dist-t" "$t/a.txt" "$t/a.txt.asc" "$t/appr.ring" "$t/allowed" run-0001
cp -r "$t/dist" "$t/dist-m"; sed -i 's/"tree_sha256": "[0-9a-f]*"/"tree_sha256": "0000000000000000000000000000000000000000000000000000000000000000"/' "$t/dist-m/helfa.org-0.0.1-manifest.json"
(cd "$t/dist-m" && sha256sum helfa.org-0.0.1-site.tar.gz helfa.org-0.0.1-manifest.json helfa.org-0.0.1.spdx.json > SHA256SUMS.txt)
red "approval: manifest lying about the hash is red" "$here/site-freigabe-pruefen.sh" "$t/dist-m" "$t/a.txt" "$t/a.txt.asc" "$t/appr.ring" "$t/allowed" run-0001

# --- signing
rm -rf "$t/d1"; cp -r "$t/dist" "$t/d1"
sign() { GPG_PRIVATE_KEY=$(cat "$t/rel.sec.asc") "$here/site-signieren.sh" "$1" "$t/a.txt" "$t/a.txt.asc" "$t/appr.ring" "$t/allowed" run-0001 "$t/rel.pub.asc"; }
red "sign: without the key is red (not 'not signed, exit 0')" env -u GPG_PRIVATE_KEY "$here/site-signieren.sh" "$t/d1" "$t/a.txt" "$t/a.txt.asc" "$t/appr.ring" "$t/allowed" run-0001 "$t/rel.pub.asc"
red "sign: with a bad approval signs nothing" env "GPG_PRIVATE_KEY=$(cat "$t/rel.sec.asc")" "$here/site-signieren.sh" "$t/d1" "$t/e.txt" "$t/e.txt.asc" "$t/appr.ring" "$t/allowed" run-0001 "$t/rel.pub.asc"
[ -z "$(find "$t/d1" -name '*.asc' -print -quit)" ] && ok "sign: a bad approval left no .asc" || bad "sign: a bad approval left no .asc"
grn "sign: good approval signs" sign "$t/d1"
grn "sign: every file has a signature" bash -c "cd '$t/d1' && for f in helfa.org-0.0.1-site.tar.gz helfa.org-0.0.1-manifest.json helfa.org-0.0.1.spdx.json SHA256SUMS.txt approval.txt; do test -s \$f.asc; done"
rm -rf "$t/d2"; cp -r "$t/dist" "$t/d2"
red "sign: wrong public key for the check is red" env "GPG_PRIVATE_KEY=$(cat "$t/rel.sec.asc")" "$here/site-signieren.sh" "$t/d2" "$t/a.txt" "$t/a.txt.asc" "$t/appr.ring" "$t/allowed" run-0001 "$t/other.pub"

# --- templates and pages
grn "pages: committed pages equal the generator output" bash -c "'$root/templates/site/gen-pages.sh' '$t/gen' && diff -r '$t/gen/sites' '$root/templates/site/sites' && diff -r '$t/gen/port80' '$root/templates/site/port80'"
for d in freesynergy.net helfa.org freeplanetearth.org; do
  grn "pages: $d passes the folder check" "$here/site-bauen.sh" check "$root/templates/site/sites/$d"
  grep -q "http-equiv=\"refresh\" content=\"0;url=https://$d/\"" "$root/templates/site/port80/$d/index.html" && grep -q "href=\"https://$d/\"" "$root/templates/site/port80/$d/index.html" && ok "port 80 page of $d: meta refresh and link" || bad "port 80 page of $d"
  grep -q 'TODO(user)' "$root/templates/site/sites/$d/index.html" && ok "$d: TODO for the real contents is marked" || bad "$d: TODO marker"
done
export SITE1=freesynergy.net SITE2=helfa.org SITE3=freeplanetearth.org ACME_EMAIL=admin@helfa.org
R="$root/templates/site/render.sh"
grn "render: staging is the default" bash -c "'$R' '$t/s.kdl' >/dev/null && grep -q 'staging #true' '$t/s.kdl' && ! grep -q '@[A-Z0-9_]*@' '$t/s.kdl'"
grn "render: production" bash -c "'$R' --production '$t/p.kdl' >/dev/null && grep -q 'staging #false' '$t/p.kdl'"
grn "render: port 80 has no redirect filter, serves pages" bash -c "! grep -q 'type \"redirect\"' '$t/s.kdl' && grep -q '/_port80/helfa.org' '$t/s.kdl'"
grn "render: install-pages" bash -c "WEBROOT='$t/www' '$R' --install-pages '$t/i.kdl' >/dev/null && test -f '$t/www/helfa.org/de/index.html' && test -f '$t/www/_port80/helfa.org/index.html'"
red "render: quote in a domain refused" env SITE2='evil".org' "$R" "$t/x.kdl"
red "render: two equal domains refused" env SITE2=helfa.org SITE3=helfa.org "$R" "$t/x.kdl"
red "render: path in WEBROOT refused" env WEBROOT='/srv/../etc' "$R" "$t/x.kdl"
red "render: domain without pages refused with --install-pages" env SITE1=other.org WEBROOT="$t/www2" "$R" --install-pages "$t/x.kdl"
exit "$fail"
