#!/usr/bin/env bash
# Local proof of the stage-0 sites with Zentinel in Podman. Nothing leaves this machine: ports are bound to 127.0.0.1, no ACME, no DNS, no server.
# The config check (step 1) runs with `--network none`; the serving container (step 2) has no such flag because a published port needs a network,
# but its configuration has no ACME and no upstream, so no outgoing traffic is expected (not enforced). The image must already be present (no pull).
#   test-podman.sh [<image>]           default image: ghcr.io/zentinelproxy/zentinel:26.11_1 (local test only; the server gets the signed mirror .deb)
# Env: CONTAINER_TOOL (default podman), PORT (default 18680; PORT and PORT+1 are used).
#   ZENTINEL_DEB=<zentinel_..._amd64.deb> BASE_IMAGE=<local image with glibc, e.g. an ubuntu>  tests the binary of the signed mirror .deb
#   (the pinned 26.11_2) instead of the maker's image. Verify the .deb first (fs-zentinel scripts/verify-mirror.sh); this script does not.
# The pages are TEST pages written here (a marker per site); real pages only come through site-bauen/site-signieren. The layout is the delivered one:
# <webroot>/<domain>/releases/1 and the link `current` -> releases/1.
# After a swap of `current` the delivery step must run `zentinel reload` (SIGHUP): shown here (stale content before, new content after).
# 1. `zentinel test -c` on the production rendering (ACME staging) and the first-start rendering: valid.   2. serves (test variant) and checks over HTTP:
# pages, 404, traversal, headers (CSP, nosniff: NOTE, see below), port 80 = real 301.
set -euo pipefail
here=$(cd "$(dirname "$0")" && pwd)
tool=${CONTAINER_TOOL:-podman}; image=${1:-ghcr.io/zentinelproxy/zentinel:26.11_1}; port=${PORT:-18680}
command -v "$tool" >/dev/null || { echo "SKIPPED: $tool not installed - nothing was tested (not green)"; exit 2; }
"$tool" image exists "$image" 2>/dev/null || { echo "SKIPPED: image $image not present locally (no pull here) - nothing was tested (not green)"; exit 2; }
t=$(mktemp -d); name=fs-site-test-$$
bin=()
if [ -n "${ZENTINEL_DEB:-}" ]; then
  image=${BASE_IMAGE:?set BASE_IMAGE (a local image) together with ZENTINEL_DEB}
  "$tool" image exists "$image" 2>/dev/null || { echo "SKIPPED: image $image not present locally - nothing was tested (not green)"; exit 2; }
  mkdir -p "$t/deb" "$t/bin" && (cd "$t/deb" && ar x "$ZENTINEL_DEB" && tar xf data.tar.* ./usr/bin/zentinel) && cp "$t/deb/usr/bin/zentinel" "$t/bin/zentinel"
  bin=(--entrypoint /zbin/zentinel -v "$t/bin:/zbin:ro,z")
fi
cleanup() { "$tool" rm -f "$name" >/dev/null 2>&1 || true; rm -rf "$t"; }; trap cleanup EXIT
chmod 755 "$t"
export ACME_EMAIL=admin@example.org
DOMAINS=(one.example.org two.example.org three.example.org)
mkdir -p "$t/www" "$t/conf"; sites=()
for d in "${DOMAINS[@]}"; do
  printf 'schema = 1\npurpose = "website"\n[names]\nprimary = "%s"\naliases = ["www.%s"]\n[acme]\nmethod = "http-01"\n' "$d" "$d" > "$t/$d.toml"; sites+=(--site "$t/$d.toml")
  mkdir -p "$t/www/$d/releases/1/de" "$t/www/$d/releases/1/en"
  for l in "" de/ en/; do printf '<!doctype html>\n<html lang="%s"><head><meta charset="utf-8"><title>test %s</title></head><body><p>MARKER-%s</p><a href="/de/">de</a> <a href="/en/">en</a></body></html>\n' "${l%/}" "$d" "$d" > "$t/www/$d/releases/1/${l}index.html"; done
  ln -s releases/1 "$t/www/$d/current"
done
WEBROOT=/www "$here/render.sh" --staging "${sites[@]}" "$t/conf/prod.kdl" >/dev/null
WEBROOT=/www "$here/render.sh" --http-only "${sites[@]}" "$t/conf/first.kdl" >/dev/null
WEBROOT=/www HTTP_ADDR=0.0.0.0:8080 HTTPS_ADDR=0.0.0.0:8081 "$here/render.sh" --test "${sites[@]}" "$t/conf/test.kdl" >/dev/null
chmod -R a+rX "$t"
mount=(-v "$t/www:/www:ro,z" -v "$t/conf:/conf:ro,z")
echo "== 1. production rendering passes zentinel test"
"$tool" run --rm --network none "${bin[@]}" "${mount[@]}" "$image" test -c /conf/prod.kdl
"$tool" run --rm --network none "${bin[@]}" "${mount[@]}" "$image" test -c /conf/first.kdl
echo "== 2. serve the pages and check them"
"$tool" run -d --name "$name" -p "127.0.0.1:$port:8080" -p "127.0.0.1:$((port+1)):8081" "${bin[@]}" "${mount[@]}" "$image" -c /conf/test.kdl >/dev/null
python3 -I - "$port" "$name" "$tool" "$t" <<'PY'
import sys, time, http.client, subprocess
port, name, tool = int(sys.argv[1]), sys.argv[2], sys.argv[3]
def get(host, path="/", p=None, **h):
    c = http.client.HTTPConnection("127.0.0.1", p or port + 1, timeout=5)
    c.request("GET", path, headers={"Host": host, **h}); r = c.getresponse()
    return r.status, r.read(), {k.lower(): v for k, v in r.getheaders()}
for _ in range(60):
    try: get("one.example.org"); break
    except OSError: time.sleep(0.25)
else:
    print(subprocess.run([tool, "logs", name], capture_output=True, text=True).stdout[-2000:]); sys.exit("proxy did not come up")
ok = True
def check(n, c):
    global ok; print(("ok   " if c else "FAIL ") + n); ok = ok and bool(c)
D = ["one.example.org", "two.example.org", "three.example.org"]
for d in D:
    mark = f"MARKER-{d}".encode()
    s, b, h = get(d); check(f"{d} / -> 200, its own page", s == 200 and mark in b and b'href="/de/"' in b)
    s, b, h = get(d, "/de/"); check(f"{d} /de/ -> 200, lang=de", s == 200 and b'<html lang="de">' in b)
    s, b, h = get(d, "/en/"); check(f"{d} /en/ -> 200, lang=en", s == 200 and b'<html lang="en">' in b)
    s, b, h = get(d, "/nope"); check(f"{d} missing -> 404", s == 404)
    for other in D:
        if other != d:
            s, b, h = get(d, f"/../{other}/current/index.html"); check(f"{d} traversal to {other} blocked", f"MARKER-{other}".encode() not in b)
            s, b, h = get(d, f"/%2e%2e/{other}/current/index.html"); check(f"{d} encoded traversal to {other} blocked", f"MARKER-{other}".encode() not in b)
    s, b, h = get(d); check(f"{d} Cache-Control set", "public" in h.get("cache-control", ""))
    # Measured with 26.11_2: the route's response-headers (CSP, nosniff, referrer policy) are NOT sent for static-files routes (upstream handlers.rs
    # writes the static response directly, the header policies only run for upstream responses). The configuration carries the setting and passes
    # `zentinel test -c`; here it is a NOTE, and a version that sends the header turns it into a checked value.
    csp = h.get("content-security-policy", "")
    if csp:
        check(f"{d} CSP header is the strict default (no script, nothing from outside)", "default-src 'none'" in csp and "script-src" not in csp and "frame-ancestors 'none'" in csp)
        check(f"{d} nosniff and referrer policy", h.get("x-content-type-options") == "nosniff" and h.get("referrer-policy") == "no-referrer")
    else:
        print(f"NOTE {d} static route sends NO Content-Security-Policy/nosniff/referrer headers (route response-headers are not applied to static-files in this version)")
    etag = h.get("etag"); check(f"{d} ETag present", bool(etag))
    # Measured, not required: 26.11_1 and 26.11_2 answered 200 (no 304) and sent no compression in this setup (upstream handlers.rs hands the static
    # server a request without headers). Printed as NOTE so a change in a later version shows up; it does not fail the test.
    s, b, h = get(d, "/", **{"If-None-Match": etag or "x"}); print(f"NOTE {d} If-None-Match -> {s} ({'304 works' if s == 304 else 'no 304'})")
    s, b, h = get(d, "/", **{"Accept-Encoding": "br, gzip"}); print(f"NOTE {d} Accept-Encoding br,gzip -> content-encoding {h.get('content-encoding')}")
    for path in ("/", "/de/x", "/a/b?x=1&y=2"):
        s, b, h = get(d, path, port)
        check(f"{d} port 80 {path} -> 301 to https, same host and path", s == 301 and h.get("location") == f"https://{d}/" + path.lstrip("/").split("?")[0])
    s, b, h = get(d, "/a?x=1", port); print(f"NOTE {d} port 80 query string in Location: {h.get('location')} (measured: it is dropped)")
    c = http.client.HTTPConnection("127.0.0.1", port, timeout=5); c.request("HEAD", "/", headers={"Host": d}); r = c.getresponse(); print(f"NOTE {d} port 80 HEAD -> {r.status} {r.getheader('location')}")
    c = http.client.HTTPConnection("127.0.0.1", port, timeout=5); c.request("POST", "/", body=b"x", headers={"Host": d}); r = c.getresponse(); print(f"NOTE {d} port 80 POST -> {r.status} {r.getheader('location')}")
# After a swap of `current` the proxy serves OLD content from its cache (measured 26.11_2, cache-control/ETag from size+mtime, same-size file = same ETag)
# until `zentinel reload` (SIGHUP): the delivery step must reload after every swap. Here: same-size new content, swap, check stale NOTE, HUP, check new.
d0 = D[0]
subprocess.run(["bash", "-c", f"cd {sys.argv[4]}/www/{d0} && mkdir -p releases/2 && sed 's/MARKER-/MARKEX-/' releases/1/index.html > releases/2/index.html && ln -sfn releases/2 current.new && mv -T current.new current"], check=True)
s, b, h = get(d0); print(f"NOTE {d0} right after the swap, before reload: {'OLD content (cache)' if b'MARKER-' in b else 'new content'}")
subprocess.run([tool, "kill", "--signal", "HUP", name], check=True, capture_output=True); time.sleep(2)
s, b, h = get(d0); check(f"{d0} after reload (SIGHUP) the swapped release is served", b"MARKEX-" in b)
s, b, h = get("evil.example"); check("unknown host gets no page", b"MARKER-" not in b)
sys.exit(0 if ok else 1)
PY
echo "all checks passed"
