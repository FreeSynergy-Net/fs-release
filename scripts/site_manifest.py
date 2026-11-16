#!/usr/bin/env python3
"""Pattern: none. Reader and validator of `site.toml`, the ONE declarative place of the host names of a site (schema: docs in
templates/site/README.md and /srv/FreeSynergy/.scratch/site-toml-schema.md). Used by site-pruefen.py (the folder check) and templates/site/render.py
(the Zentinel configuration is generated from it). A later DnsProvider reads the same file (names, wanted record types); nothing here touches DNS.

usage: python3 -I site_manifest.py <site.toml>     prints `primary alias...` of a valid file, exit 1 and `reason` on stderr otherwise
"""
import re
import sys
import tomllib

DOMAIN = re.compile(r"^([a-z0-9]([a-z0-9-]{0,61}[a-z0-9])?\.)+[a-z]{2,63}$")
CSP_CHARS = re.compile(r"^[A-Za-z0-9 ;:'*._/=+,%-]{1,1000}$")
KEYS = {"schema", "purpose", "names", "dns", "acme", "csp"}
RECORDS = {"A", "AAAA", "CAA"}


def load(path: str) -> dict:
    """Parse and validate; raises ValueError with the first reason. Returns a normalised dict."""
    raw = open(path, "rb").read()
    if len(raw) > 8192:
        raise ValueError("site.toml is larger than 8 KB")
    try:
        m = tomllib.loads(raw.decode("utf-8"))
    except (UnicodeDecodeError, tomllib.TOMLDecodeError) as e:
        raise ValueError(f"not valid TOML ({e})") from e
    if set(m) - KEYS or KEYS - {"csp", "dns"} - set(m):
        raise ValueError(f"top-level keys must be among {sorted(KEYS)} (csp and dns optional); found {sorted(m)}")
    if m["schema"] != 1:
        raise ValueError("schema must be 1")
    if m["purpose"] != "website":
        raise ValueError('purpose must be "website"')
    names = m["names"]
    if not isinstance(names, dict) or set(names) - {"primary", "aliases"} or "primary" not in names:
        raise ValueError("names needs primary and may have aliases")
    primary, aliases = names["primary"], names.get("aliases", [])
    if not isinstance(primary, str) or not isinstance(aliases, list) or not all(isinstance(a, str) for a in aliases):
        raise ValueError("names: primary is a string, aliases a list of strings")
    allnames = [primary] + aliases
    if len(allnames) > 10 or len(set(allnames)) != len(allnames):
        raise ValueError("names: at most 10, none twice")
    for n in allnames:
        if not DOMAIN.match(n):
            raise ValueError(f"'{n}' is not a plain lower-case domain name")
    dns = m.get("dns", {"records": ["A", "AAAA"]})
    if not isinstance(dns, dict) or set(dns) - {"records"}:
        raise ValueError("dns may only have records")
    rec = dns.get("records", ["A", "AAAA"])
    if not isinstance(rec, list) or not rec or not set(rec) <= RECORDS or len(set(rec)) != len(rec):
        raise ValueError(f"dns.records: a non-empty list from {sorted(RECORDS)}, each once")
    acme = m["acme"]
    if not isinstance(acme, dict) or set(acme) != {"method"}:
        raise ValueError("acme needs exactly method")
    if acme["method"] == "dns-01":
        raise ValueError('acme.method "dns-01" is reserved and not implemented yet')
    if acme["method"] != "http-01":
        raise ValueError('acme.method must be "http-01"')
    csp = m.get("csp", {})
    if not isinstance(csp, dict) or set(csp) - {"mode", "policy"}:
        raise ValueError("csp may only have mode and policy")
    mode = csp.get("mode", "static")
    if mode not in ("static", "passthrough"):
        raise ValueError("csp.mode must be static or passthrough")
    policy = csp.get("policy", "")
    if policy and not (isinstance(policy, str) and CSP_CHARS.match(policy)):
        raise ValueError("csp.policy holds a character that is not allowed (no double quote, backslash, @, & or line break)")
    return {"primary": primary, "names": allnames, "records": rec, "acme": acme["method"], "csp_mode": mode, "csp_policy": policy}


if __name__ == "__main__":
    if len(sys.argv) != 2:
        print("usage: site_manifest.py <site.toml>", file=sys.stderr)
        sys.exit(2)
    try:
        print(" ".join(load(sys.argv[1])["names"]))
    except (ValueError, OSError) as e:
        print(f"{sys.argv[1]}: {e}", file=sys.stderr)
        sys.exit(1)
