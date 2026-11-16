# site.toml, schema 1 (fs-release stage 0; shared with the fs-text-site agent)

Status 2026-11-16, branch `unreviewed/site-stufe0` in fs-release. Implemented and tested in `scripts/site_manifest.py` (reader and validator), `scripts/site-pruefen.py` (folder check) and `templates/site/render.py` (Zentinel configuration); the selftest is `scripts/test-site.sh`.

## What it is
The ONE declarative place of the host names of a site. It sits at the root of the site folder (next to `index.html`), is therefore part of the tarball that is signed, and is read by (1) the folder check of `site-bauen` (required, validated), (2) `render.sh` (Zentinel config: certificates, routes, headers), (3) later a DnsProvider adapter of fs-server (ADR 0088): it reads names and wanted record types and manages only its own records. No template, script or test contains a host name (fixtures only).

## Fields
```toml
schema = 1                      # required, must be 1
purpose = "website"             # required, only "website" for now

[names]
primary = "example.org"         # required; lower-case DNS name; also the directory name /srv/www/<primary>/current
aliases = ["www.example.org"]   # optional; each lower-case DNS name; up to 9; none twice; none equal to primary

[dns]                           # optional; wanted record types (the later DnsProvider reads this; nothing sets DNS today)
records = ["A", "AAAA", "CAA"]  # non-empty subset of A, AAAA, CAA, each once; default ["A", "AAAA"]

[acme]
method = "http-01"              # required; "http-01" only; "dns-01" is RESERVED and rejected with a clear message until it is implemented

[csp]                           # optional
mode = "static"                 # static (default, strict, no script) | passthrough (own scripts of the site; build must run with --passthrough, and vice versa)
policy = "default-src 'self'"   # optional complete Content-Security-Policy that replaces the mode's preset
```

## Validation rules (red = the folder is refused)
- Valid TOML, UTF-8, at most 8 KB; top-level keys only `schema purpose names dns acme csp`; no other keys anywhere (no tokens, no secrets).
- Names match `^([a-z0-9]([a-z0-9-]{0,61}[a-z0-9])?\.)+[a-z]{2,63}$` (lower-case, no IDN, no wildcard, no trailing dot); at most 10 names per site; no name twice. `render.sh` additionally refuses a name used by two sites.
- `csp.mode` must equal the mode of the build (`static` or `--passthrough`).
- `csp.policy` only characters `A-Za-z0-9 ;:'*._/=+,%-` (no double quote, backslash, `@`, `&`, line break), 1..1000.
- The file type `toml` is on the allowlist; the file is served as a normal static file (it holds names only, no secret).

## Notes for fs-text-site
- Produce `site.toml` next to `index.html` in the output folder; the folder must otherwise pass `site-bauen check` (allowlist of file types, no dot files, no script/external resource in static mode, parsed HTML). A `<meta http-equiv="Content-Security-Policy" ...>` is allowed and recommended as a stopgap: Zentinel 26.11_2 sends no CSP header on static routes (measured). `<meta http-equiv=refresh>` is refused.
- Release time: the tarball gets the commit time of the approved source as mtime (`SITE_MTIME`); nothing to do in the generator.
- Question to the other side if the schema must change: add a field only with a `schema` bump; unknown keys are refused by design.

## Schema 2 (several sites on one domain)
Status 2026-11-16, proposal of the fs-text-site agent (`/srv/FreeSynergy/.scratch/site-toml-schema-v2.md`), taken over in `scripts/site_manifest.py`, `scripts/site-pruefen.py` and `templates/site/render.py`. Example: `site.example.toml`. **Schema 1 stays valid and unchanged**; it reads as `site.id = "main"`, `path_prefix = "/"`, `languages = ["en"]`, `default_language = "en"`, directory = `names.primary`. Schema 1 refuses `[site]` and `[[csp.area]]`; unknown keys are refused in both.

New in schema 2:
- `[site]` (required): `id` (a-z 0-9 -, 1..32, not starting/ending with -; names the directory `<webroot>/<id>/current`), `name` (optional, 1..80 characters, no control character), `path_prefix` (optional, default `/`), `languages` (optional, 1..8 tags `[a-z]{2,3}(-[a-z0-9]{2,8}){0,2}`, each once; default `["en"]`), `default_language` (optional, one of `languages`). The folder check demands `<lang>/index.html` for every language except the default one (which is at the root).
- There is **no `host` key**: the host is `names.primary`.
- `path_prefix` (and `csp.area.path`): `/` or `/seg(/seg)*`, up to 4 segments, 100 bytes, segment `[a-z0-9._-]`, not starting with `.`, no empty segment, no `.`/`..`, no `%`, `\`, `?`, `#`, `:`, no upper case; the first segment is never `_fs`. A prefix cannot climb out of its place.
- Host names: as schema 1, but the last label may also be an A-label `xn--...` (international names are given as A-label; the label must be real Punycode, there is no IDNA library). Total at most 253 characters, checked with `fullmatch`.
- `[[csp.area]]` (optional, up to 8, no path twice, not `/`): `path` (a folder of the site, must exist), `mode` (`static`|`passthrough`), `policy` (optional). Each area gets its own route and CSP in the Zentinel configuration. An area in `passthrough` makes the whole site a passthrough site (build with `--passthrough`, render with `--allow-passthrough`).
- Collisions between sites (`site_manifest.check_set`, used by `render.sh`, all errors listed before anything is written): the same id/directory; the same name with the same `path_prefix`; an inner site (longer prefix on a shared name) whose first segment below the outer prefix is something the outer site generates itself (`_fs`, `index.html`, `sitemap.xml`, `robots.txt`, `404.html`, `site-manifest.json`, `site.toml`, or one of its language folders). The same name in two sites is allowed when the prefixes differ (one certificate for the name; the routes differ by `path-prefix`, the longest first).
- **Measured (26.11_2):** Zentinel `static-files` does NOT remove the route's `path-prefix` before it looks for the file (`/wiki/` serves `<root>/wiki/index.html`). A site with a `path_prefix` other than `/` therefore gets the Zentinel root `<webroot>/<id>/mount`, and the delivery step links `mount/<prefix>` to the swapped `current` (`ln -s ../current mount/wiki`); a site at `/` has the root `<webroot>/<id>/current`. Open for the staging run: that Zentinel serves a link inside its root.
