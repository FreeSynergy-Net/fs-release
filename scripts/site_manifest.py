#!/usr/bin/env python3
"""Pattern: none. Reader and validator of `site.toml`, the ONE declarative place of the host names of a site (schema 1 and 2: templates/site/site-toml-schema.md). Used by site-pruefen.py (the folder check) and templates/site/render.py (the Zentinel configuration is generated from it). A later
DnsProvider reads the same file (names, wanted record types); nothing here touches DNS.

Schema 1 (unchanged, stays valid): schema, purpose, names, dns, acme, csp; one site per file, at the root of its host, directory `<webroot>/<primary>`.
Schema 2 (several sites on one domain): the same plus `[site]` (id, name, path_prefix, languages, default_language) and `[[csp.area]]` (a policy per folder).
  `host` does not exist (the host is names.primary). The same name in two sites is allowed when path_prefix differs (check_set). Directory = `<webroot>/<id>`.
A schema 1 file reads as id "main" (directory = primary), path_prefix "/", languages ["en"], default_language "en".

usage: python3 -I site_manifest.py <site.toml>...     prints `primary alias...` of a valid file (several files: also the collision check), exit 1 and `reason` on stderr
"""
import re
import sys
import tomllib
import unicodedata

# fullmatch everywhere: `$` would let a trailing line break through
DOMAIN1 = re.compile(r"([a-z0-9]([a-z0-9-]{0,61}[a-z0-9])?\.)+[a-z]{2,63}")  # schema 1, unchanged
LABEL = re.compile(r"[a-z0-9]([a-z0-9-]{0,61}[a-z0-9])?")
TLD2 = re.compile(r"[a-z]{2,63}|xn--[a-z0-9-]{1,59}[a-z0-9]")  # schema 2: letters, or an A-label (international name as xn--...)
CSP_CHARS = re.compile(r"[A-Za-z0-9 ;:'*._/=+,%-]{1,1000}")
SITE_ID = re.compile(r"[a-z0-9]|[a-z0-9][a-z0-9-]{0,30}[a-z0-9]")
LANGTAG = re.compile(r"[a-z]{2,3}(-[a-z0-9]{2,8}){0,2}")
SEGMENT = re.compile(r"[a-z0-9_-][a-z0-9._-]*")  # not starting with `.`; `_` is allowed (`_fs` is refused by name below)
KEYS1 = {"schema", "purpose", "names", "dns", "acme", "csp"}
KEYS2 = KEYS1 | {"site"}
RECORDS = {"A", "AAAA", "CAA"}
# what a site generates itself at its root (fs-text-site): an inner site may not take one of these names as the first segment below the outer prefix
GENERATED = {"_fs", "index.html", "sitemap.xml", "robots.txt", "404.html", "site-manifest.json", "site.toml"}


def host_ok(n: str, schema: int) -> bool:
    if len(n) > 253:
        return False
    if schema == 1:
        return DOMAIN1.fullmatch(n) is not None
    labels = n.split(".")
    if len(labels) < 2 or not all(LABEL.fullmatch(x) for x in labels[:-1]) or not TLD2.fullmatch(labels[-1]):
        return False
    for x in labels:  # an A-label must be real Punycode (stdlib codec, no IDNA library); a wrong one is a name nobody resolves, not an injection
        if x.startswith("xn--"):
            try:
                x[4:].encode("ascii").decode("punycode")
            except (UnicodeError, ValueError):
                return False
    return True


def prefix_ok(p: object, what: str, allow_root: bool) -> str:
    """A path inside a site: `/` or `/seg(/seg)*`, up to 4 segments, 100 bytes; it cannot climb out (no `..`, no `.`, no empty segment, no `%`, `\\`, `?`, `#`, `:`, no upper case)."""
    if not isinstance(p, str) or not p.startswith("/") or len(p.encode()) > 100:
        raise ValueError(f"{what} must be a string starting with / (at most 100 bytes)")
    if p == "/":
        if not allow_root:
            raise ValueError(f"{what} must not be the root /")
        return p
    segs = p[1:].split("/")
    if len(segs) > 4 or not all(SEGMENT.fullmatch(x) for x in segs) or any(x in (".", "..") for x in segs):
        raise ValueError(f"{what} {p!r} must be /seg(/seg)* with at most 4 segments of [a-z0-9._-], not starting with a dot, no `..`")
    if segs[0] == "_fs":
        raise ValueError(f"{what}: the first segment must not be _fs (reserved)")
    return p


def join(prefix: str, sub: str) -> str:
    return sub if prefix == "/" else prefix + sub


def load(path: str) -> dict:
    """Parse and validate; raises ValueError with the first reason. Returns a normalised dict (see the keys at the end)."""
    raw = open(path, "rb").read()
    if len(raw) > 8192:
        raise ValueError("site.toml is larger than 8 KB")
    try:
        m = tomllib.loads(raw.decode("utf-8"))
    except (UnicodeDecodeError, tomllib.TOMLDecodeError) as e:
        raise ValueError(f"not valid TOML ({e})") from e
    schema = m.get("schema")
    if schema not in (1, 2) or isinstance(schema, bool):
        raise ValueError("schema must be 1 or 2")
    keys = KEYS2 if schema == 2 else KEYS1
    must = keys - {"csp", "dns"}
    if set(m) - keys or must - set(m):
        raise ValueError(f"schema {schema}: top-level keys must be among {sorted(keys)} (csp and dns optional, `site` only in schema 2 and then required); found {sorted(m)}")
    if m["purpose"] != "website":
        raise ValueError('purpose must be "website"')
    names = m["names"]
    if not isinstance(names, dict) or set(names) - {"primary", "aliases"} or "primary" not in names:
        raise ValueError("names needs primary and may have aliases (the host of the site is names.primary; there is no `host` key)")
    primary, aliases = names["primary"], names.get("aliases", [])
    if not isinstance(primary, str) or not isinstance(aliases, list) or not all(isinstance(a, str) for a in aliases):
        raise ValueError("names: primary is a string, aliases a list of strings")
    allnames = [primary] + aliases
    if len(allnames) > 10 or len(set(allnames)) != len(allnames):
        raise ValueError("names: at most 10, none twice")
    for n in allnames:
        if not host_ok(n, schema):
            raise ValueError(f"'{n[:80]}' is not a plain lower-case domain name (at most 253 characters)")
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
    ckeys = {"mode", "policy"} | ({"area"} if schema == 2 else set())
    if not isinstance(csp, dict) or set(csp) - ckeys:
        raise ValueError("csp may only have mode and policy" + (" and [[area]]" if schema == 2 else " (`[[csp.area]]` needs schema 2)"))
    mode = csp.get("mode", "static")
    if mode not in ("static", "passthrough"):
        raise ValueError("csp.mode must be static or passthrough")
    policy = csp.get("policy", "")
    if policy and not (isinstance(policy, str) and CSP_CHARS.fullmatch(policy)):
        raise ValueError("csp.policy holds a character that is not allowed (no double quote, backslash, @, & or line break)")
    areas = []
    if schema == 2:
        raw_areas = csp.get("area", [])
        if not isinstance(raw_areas, list) or len(raw_areas) > 8:
            raise ValueError("csp.area: a list of at most 8 tables")
        for a in raw_areas:
            if not isinstance(a, dict) or set(a) - {"path", "mode", "policy"} or not {"path", "mode"} <= set(a):
                raise ValueError("csp.area: each has path and mode, and may have policy")
            if a["mode"] not in ("static", "passthrough"):
                raise ValueError("csp.area.mode must be static or passthrough")
            ap = a.get("policy", "")
            if ap and not (isinstance(ap, str) and CSP_CHARS.fullmatch(ap)):
                raise ValueError("csp.area.policy holds a character that is not allowed")
            areas.append({"path": prefix_ok(a["path"], "csp.area.path", False), "mode": a["mode"], "policy": ap})
        if len({a["path"] for a in areas}) != len(areas):
            raise ValueError("csp.area: a path twice")
    sid, sname, prefix, langs, deflang = "main", primary, "/", ["en"], "en"
    if schema == 2:
        site = m["site"]
        if not isinstance(site, dict) or set(site) - {"id", "name", "path_prefix", "languages", "default_language"} or "id" not in site:
            raise ValueError("site needs id and may have name, path_prefix, languages, default_language")
        sid = site["id"]
        if not isinstance(sid, str) or not SITE_ID.fullmatch(sid):
            raise ValueError("site.id: a-z 0-9 -, 1..32, not starting or ending with -")
        sname = site.get("name", sid)
        if not isinstance(sname, str) or not 1 <= len(sname) <= 80 or any(unicodedata.category(c) in ("Cc", "Cf", "Zl", "Zp") for c in sname):
            raise ValueError("site.name: 1..80 characters, no control character")
        prefix = prefix_ok(site.get("path_prefix", "/"), "site.path_prefix", True)
        langs = site.get("languages", ["en"])
        if not isinstance(langs, list) or not 1 <= len(langs) <= 8 or not all(isinstance(x, str) and LANGTAG.fullmatch(x) for x in langs) or len(set(langs)) != len(langs):
            raise ValueError("site.languages: 1..8 lower-case language tags (de, en, pt-br), each once")
        deflang = site.get("default_language", langs[0])
        if deflang not in langs:
            raise ValueError("site.default_language must be one of site.languages")
    eff = "passthrough" if mode == "passthrough" or any(a["mode"] == "passthrough" for a in areas) else "static"
    return {"schema": schema, "id": sid, "name": sname, "dir": sid if schema == 2 else primary, "path_prefix": prefix, "languages": langs,
            "default_language": deflang, "primary": primary, "names": allnames, "records": rec, "acme": acme["method"], "csp_mode": mode,
            "csp_policy": policy, "areas": areas, "mode": eff}


def nested(inner: str, outer: str) -> bool:
    """inner is the same place as outer or below it, on a segment boundary (/wiki is not below /wik)."""
    return inner == outer or outer == "/" or inner.startswith(outer + "/")


def check_set(sites: list[dict]) -> list[str]:
    """Collisions between sites (all listed): the same directory; the same name with the same prefix; an inner site that takes a name the outer site generates itself."""
    err: list[str] = []
    for i, a in enumerate(sites):
        for b in sites[i + 1:]:
            if a["dir"] == b["dir"]:
                err.append(f"two sites with the same id/directory '{a['dir']}'")
            for n in set(a["names"]) & set(b["names"]):
                pa, pb = a["path_prefix"], b["path_prefix"]
                if pa == pb:
                    err.append(f"the name {n} is used by two sites with the same path_prefix {pa}")
                    continue
                same = ({join(pa, x["path"]) for x in a["areas"]} | {pa}) & ({join(pb, x["path"]) for x in b["areas"]} | {pb})
                if pa != pb and same:
                    err.append(f"the name {n}: sites {a['dir']} and {b['dir']} both own the place {sorted(same)[0]} (two equal routes: a site prefix lies exactly on a csp.area of the other site, or both sites have an area at the same place)")
                    continue
                outer, inner = (a, b) if len(pa) < len(pb) else (b, a)
                if not nested(inner["path_prefix"], outer["path_prefix"]):
                    continue
                below = inner["path_prefix"][len(outer["path_prefix"].rstrip("/")):].lstrip("/").split("/")[0]
                if below in GENERATED | set(outer["languages"]):
                    err.append(f"site {inner['dir']} at {inner['path_prefix']} takes '{below}', a name that site {outer['dir']} generates itself")
    return list(dict.fromkeys(err))


if __name__ == "__main__":
    if len(sys.argv) < 2:
        print("usage: site_manifest.py <site.toml>...", file=sys.stderr)
        sys.exit(2)
    loaded = []
    try:
        for f in sys.argv[1:]:
            loaded.append(load(f))
            print(" ".join(loaded[-1]["names"]))
    except (ValueError, OSError) as e:
        print(f"{f}: {e}", file=sys.stderr)
        sys.exit(1)
    errs = check_set(loaded)
    for e in errs:
        print(e, file=sys.stderr)
    sys.exit(1 if errs else 0)
