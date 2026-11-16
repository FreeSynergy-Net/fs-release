#!/usr/bin/env bash
# Local proof of the stage-0 sites with Zentinel in Podman. Nothing leaves this machine: ports are bound to 127.0.0.1, no ACME, no DNS,
# no server, no network access (the image must already be present; it is not pulled here).
#   test-podman.sh [<image>]           default image: ghcr.io/zentinelproxy/zentinel:26.11_1 (local test only; the server gets the signed mirror .deb)
# Env: CONTAINER_TOOL (default podman), PORT (default 18680; PORT and PORT+1 are used).
#   ZENTINEL_DEB=<zentinel_..._amd64.deb> BASE_IMAGE=<local image with glibc, e.g. an ubuntu>  tests the binary of the signed mirror .deb
#   (the pinned 26.11_2) instead of the maker's image. Verify the .deb first (fs-zentinel scripts/verify-mirror.sh); this script does not.
# 1. `zentinel test -c` on the PRODUCTION rendering (ACME staging): the file is valid.   2. serves the pages (test variant) and checks them over HTTP.
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
export SITE1=freesynergy.net SITE2=helfa.org SITE3=freeplanetearth.org ACME_EMAIL=admin@helfa.org WEBROOT=/www
mkdir -p "$t/www" "$t/conf"
WEBROOT="$t/www" "$here/render.sh" --staging --install-pages "$t/conf/prod.kdl" >/dev/null
sed "s#$t/www#/www#g" "$t/conf/prod.kdl" > "$t/conf/prod.tmp" && mv "$t/conf/prod.tmp" "$t/conf/prod.kdl"
WEBROOT=/www HTTP_ADDR=0.0.0.0:8080 HTTPS_ADDR=0.0.0.0:8081 "$here/render.sh" --test "$t/conf/test.kdl" >/dev/null
chmod -R a+rX "$t"
mount=(-v "$t/www:/www:ro,z" -v "$t/conf:/conf:ro,z")
echo "== 1. production rendering passes zentinel test"
"$tool" run --rm --network none "${bin[@]}" "${mount[@]}" "$image" test -c /conf/prod.kdl
echo "== 2. serve the pages and check them"
"$tool" run -d --name "$name" -p "127.0.0.1:$port:8080" -p "127.0.0.1:$((port+1)):8081" "${bin[@]}" "${mount[@]}" "$image" -c /conf/test.kdl >/dev/null
python3 -I - "$port" "$name" "$tool" <<'PY'
import sys, time, http.client, subprocess
port, name, tool = int(sys.argv[1]), sys.argv[2], sys.argv[3]
def get(host, path="/", p=None, **h):
    c = http.client.HTTPConnection("127.0.0.1", p or port + 1, timeout=5)
    c.request("GET", path, headers={"Host": host, **h}); r = c.getresponse()
    return r.status, r.read(), {k.lower(): v for k, v in r.getheaders()}
for _ in range(60):
    try: get("helfa.org"); break
    except OSError: time.sleep(0.25)
else:
    print(subprocess.run([tool, "logs", name], capture_output=True, text=True).stdout[-2000:]); sys.exit("proxy did not come up")
ok = True
def check(n, c):
    global ok; print(("ok   " if c else "FAIL ") + n); ok = ok and bool(c)
D = ["freesynergy.net", "helfa.org", "freeplanetearth.org"]
for d in D:
    s, b, h = get(d); check(f"{d} / -> 200, language choice", s == 200 and b'href="/de/"' in b and b'href="/en/"' in b)
    s, b, h = get(d, "/de/"); check(f"{d} /de/ -> 200, lang=de", s == 200 and b'<html lang="de">' in b)
    s, b, h = get(d, "/en/"); check(f"{d} /en/ -> 200, lang=en", s == 200 and b'<html lang="en">' in b)
    s, b, h = get(d, "/nope"); check(f"{d} missing -> 404", s == 404)
    s, b, h = get(d, "/../helfa.org/index.html"); check(f"{d} traversal blocked", b"<title>helfa.org</title>" not in b or d == "helfa.org")
    for path in ("/", "/de/", "/en/"):
        s, b, h = get(d, path); check(f"{d}{path} no script, no external resource", b"<script" not in b and b'src="http' not in b and b"stylesheet" not in b)
    s, b, h = get(d); check(f"{d} Cache-Control set", "public" in h.get("cache-control", ""))
    etag = h.get("etag"); check(f"{d} ETag present", bool(etag))
    # Measured, not required: 26.11_1 and 26.11_2 answered 200 (no 304) and sent no compression in this setup, although the source reading
    # (zentinel-static.md) says ETag/304 and br/gzip exist. Printed as NOTE so a change in a later version shows up; it does not fail the test.
    s, b, h = get(d, "/", **{"If-None-Match": etag or "x"}); print(f"NOTE {d} If-None-Match -> {s} ({'304 works' if s == 304 else 'no 304'})")
    s, b, h = get(d, "/", **{"Accept-Encoding": "br, gzip"}); print(f"NOTE {d} Accept-Encoding br,gzip -> content-encoding {h.get('content-encoding')}")
    s, b, h = get(d, "/", port); check(f"{d} port 80 page -> 200, meta refresh + link to https",
        s == 200 and f'http-equiv="refresh" content="0;url=https://{d}/"'.encode() in b and f'href="https://{d}/"'.encode() in b)
    s, b, h = get(d, "/de/x", port); check(f"{d} port 80, any path -> the redirect page, no site content", s == 200 and f"https://{d}/".encode() in b and b"Wir bauen um" not in b)
s, b, h = get("evil.example"); check("unknown host gets no page", b"rebuilding" not in b.lower() and b"umbau" not in b.lower())
sys.exit(0 if ok else 1)
PY
echo "all checks passed"
