#!/usr/bin/env python3
"""Pattern: none. Generates the Zentinel configuration FROM the site.toml files; no host name is written in a template, a script or a test.
usage: render.py [--staging|--production|--http-only|--test] --site <site.toml> [--site <site.toml> ...] <output.kdl>
Environment: ACME_EMAIL (not needed with --http-only/--test), WEBROOT (default /srv/www), HTTP_ADDR (default 0.0.0.0:80), HTTPS_ADDR (default 0.0.0.0:443).
Per site: certificate for primary name plus aliases (acme.method http-01), one route per name, root <WEBROOT>/<primary>/current, CSP from csp.mode/csp.policy.
Also writes <output.kdl>.sha256. Same input, same bytes. Does no DNS and touches no server. Details: README.md.
"""
import hashlib
import os
import re
import sys

sys.path.insert(0, os.path.join(os.path.dirname(os.path.abspath(__file__)), "..", "..", "scripts"))
import site_manifest  # noqa: E402

CSP_STATIC = "default-src 'none'; img-src 'self'; style-src 'self' 'unsafe-inline'; font-src 'self'; base-uri 'none'; form-action 'none'; frame-ancestors 'none'"
CSP_PASS = ("default-src 'self'; script-src 'self'; style-src 'self' 'unsafe-inline'; img-src 'self' data:; font-src 'self'; connect-src 'self'; "
            "object-src 'none'; base-uri 'none'; form-action 'self'; frame-ancestors 'none'")
ADDR = re.compile(r"^[0-9]{1,3}(\.[0-9]{1,3}){3}:[0-9]{1,5}$")
EMAIL = re.compile(r"^[A-Za-z0-9._+-]+@([a-z0-9-]+\.)+[a-z]{2,63}$")


def die(msg: str) -> None:
    print(f"error: {msg}", file=sys.stderr)
    sys.exit(2)


def main(argv: list[str]) -> None:
    staging, tpl, variant, out, paths = "true", "zentinel-static.kdl.in", "acme", None, []
    it = iter(argv)
    for a in it:
        if a == "--staging":
            staging, tpl, variant = "true", "zentinel-static.kdl.in", "acme"
        elif a == "--production":
            staging, tpl, variant = "false", "zentinel-static.kdl.in", "acme"
        elif a == "--http-only":
            tpl, variant = "zentinel-static-http-only.kdl.in", "plain"
        elif a == "--test":
            tpl, variant = "zentinel-static-test.kdl.in", "plain"
        elif a == "--site":
            paths.append(next(it, ""))
        elif a.startswith("-"):
            die(f"unknown option {a}")
        elif out is None:
            out = a
        else:
            die("only one output file")
    if out is None or not paths or not all(paths):
        die("usage: render.py [--staging|--production|--http-only|--test] --site <site.toml> [--site ...] <output.kdl>")
    email = os.environ.get("ACME_EMAIL", "")
    if variant == "acme" and not email:
        die("ACME_EMAIL is not set")
    if variant == "plain":
        email = "test@example.org"
    webroot = os.environ.get("WEBROOT", "/srv/www")
    http_addr, https_addr = os.environ.get("HTTP_ADDR", "0.0.0.0:80"), os.environ.get("HTTPS_ADDR", "0.0.0.0:443")
    if not EMAIL.match(email):
        die("ACME_EMAIL is not a plain e-mail address")
    if not (re.match(r"^/[A-Za-z0-9._/-]*$", webroot) and not webroot.endswith("/") and ".." not in webroot):
        die("WEBROOT must be an absolute path of plain characters, without trailing slash")
    if not (ADDR.match(http_addr) and ADDR.match(https_addr)) or http_addr == https_addr:
        die("HTTP_ADDR and HTTPS_ADDR must look like 0.0.0.0:80 and differ")
    sites, seen = [], set()
    for p in paths:
        try:
            s = site_manifest.load(p)
        except (ValueError, OSError) as e:
            die(f"{p}: {e}")
        if seen & set(s["names"]):
            die(f"{p}: a name is already used by another site (one domain, one acme block)")
        seen |= set(s["names"])
        sites.append(s)
    acme_blocks, routes = [], []
    for i, s in enumerate(sites):
        doms = " ".join(f'"{n}"' for n in s["names"])
        body = (f'acme {{\n    email "{email}"\n    domains {doms}\n    staging #{staging}\n    storage "/var/lib/zentinel/acme"\n    challenge-type "{s["acme"]}"\n}}')
        body = "\n".join("            " + ln for ln in body.split("\n"))
        acme_blocks.append(body if i == 0 else "            sni {\n" + "\n".join("    " + ln for ln in body.split("\n")) + "\n            }")
        csp = s["csp_policy"] or (CSP_PASS if s["csp_mode"] == "passthrough" else CSP_STATIC)
        if s["csp_mode"] == "passthrough":
            print(f"WARNING: site {s['primary']} is in passthrough mode (its own scripts run). Zentinel 26.11_2 does NOT send route response-headers for static-files routes "
                  "(measured by test-podman.sh), so the CSP is written into the configuration but not delivered yet. Do not go live with passthrough pages before a version "
                  "that sends it (or another wall) is proven.", file=sys.stderr)
        for j, n in enumerate(s["names"]):
            routes.append(f'''    route "site{i + 1}-{j + 1}" {{
        matches {{
            host "{n}"
            path-prefix "/"
        }}
        service-type "static"
        static-files {{
            // `current` is a link to releases/<version>, swapped atomically by the person (site-signieren, step deliver)
            root "{webroot}/{s["primary"]}/current"
            index "index.html"
            cache-control "public, max-age=300"
            compress #true
        }}
        policies {{
            response-headers {{
                set {{
                    "Content-Security-Policy" "{csp}"
                    "X-Content-Type-Options" "nosniff"
                    "Referrer-Policy" "no-referrer"
                }}
            }}
        }}
    }}''')
    here = os.path.dirname(os.path.abspath(__file__))
    text = open(os.path.join(here, tpl)).read()
    for k, v in {"ACME": "\n".join(acme_blocks), "ROUTES": "\n".join(routes)}.items():
        text = text.replace(f"@{k}@", v)
    for k, v in {"ACME_EMAIL": email, "WEBROOT": webroot, "HTTP_ADDR": http_addr, "HTTPS_ADDR": https_addr}.items():
        text = text.replace(f"@{k}@", v)
    if re.search(r"@[A-Z0-9_]+@", text):
        die("unreplaced marker left in the template")
    data = (text.rstrip("\n") + "\n").encode()
    with open(out, "wb") as f:
        f.write(data)
    with open(out + ".sha256", "w") as f:
        f.write(f"{hashlib.sha256(data).hexdigest()}  {os.path.basename(out)}\n")
    print(f"wrote {out} and {out}.sha256 (variant={variant}, staging={staging}, sites={len(sites)}, webroot={webroot})")


if __name__ == "__main__":
    main(sys.argv[1:])
