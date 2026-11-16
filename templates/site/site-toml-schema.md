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
