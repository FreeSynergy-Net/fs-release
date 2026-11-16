# Site schema: where fs-text-site and fs-release differ

Source: Opus re-review 4 of `unreviewed/site-stufe0` (fs-release) against fs-text-site `unreviewed/text-site` (bdd0eee). fs-release is the stricter side everywhere it differs; the reference for the schema is `templates/site/site-toml-schema.md`. This list is the work order for adapting fs-text-site (nothing was changed in fs-text-site here).

## Blocker 3: the chain is broken (must change in fs-text-site)

fs-text-site writes **schema 1** into its output folder (`config.rs:236 to_schema1`, `write_site.rs:63`). `id` and `path_prefix` are lost on the way. Measured: the built outputs `multi/landing` and `multi/wiki` rendered together by `render.py` are red ("same id/directory 'example.test'", "same path_prefix /"), while the wiki output links to `https://example.test/wiki/...`. So schema 2 of fs-release does not reach fs-text-site content today.

Wanted: fs-text-site writes schema 2 (`[site] id, name, path_prefix, languages, default_language`, `[[csp.area]]`) into the package root `site.toml`, or fs-release gets the source `site.toml` as a second input.

## Differences

| # | Topic | fs-text-site | fs-release | Safe? |
|---|---|---|---|---|
| 1 | Chain | writes schema 1 (see above) | reads schema 2 | no: blocker |
| 2 | Same `id` on different hosts | allowed (`sites.rs:96`, `out_name` = host_prefix, commit 792dcd7) | refused: the directory is the `id`, "unique over all sites" | fs-release stricter; decide: `id` unique, or directory = host + id |
| 3a | Source keys | knows `[build]` (dead_links, folder_index) and `csp.area.allow_loosen` | refuses both | stricter; they belong to the source, not to the package |
| 3b | `csp.area.mode` | optional | required | stricter |
| 3c | Loosening guard | has one (`csp_rel.rs`) | has none | fs-release has no loosening at all |
| 4a | Host `xn--` | only checks characters | decodes as Punycode (the schema text says "not decoded": fix the text) | stricter |
| 4b | Minimum length of the TLD A-label | 6 | 5 | align |
| 4c | `site.name` | refuses `is_control` | additionally refuses Cf, Zl, Zp (bidi) | stricter |
| 5 | Languages | ALL languages become folders, the root is the language chooser | `site.example.toml:11` says "default language at the root, every other one is a folder"; the check only demands folders for non-default languages and accepts both | text wrong: fix the sentence |
| 6 | `csp.policy` | the example `default-src 'self'` is written as a meta CSP (`write.rs:30`) | the folder check refuses it (the four `'none'` directives are missing) | safe, but the two documents contradict each other |
| 7 | `.well-known/security.txt` | passthrough output carries it (fixture) | refused as a dot file; the schema text names only `.well-known/acme-challenge` as reserved | decide: allow exactly this file, or drop it in the exporter |
| 8 | Delivery of a site with a prefix | the output has its files at the folder root | Zentinel 26.11_2 does not cut the `path-prefix`: root `<webroot>/<id>/mount`, link `mount/<prefix>` -> `../current` | measured; the link is made by the delivery step |

## Equal on both sides

id rule, prefix rules, language tag, at most 8 areas, area not `/`, the GENERATED name list, collision rules for the same name and prefix and for inner sites (fs-release also refuses an inner site exactly on a `csp.area` of the outer one).

## Note

`scripts/site-kdl-hash.sh` forms the hash per single site. With schema 2 the server renders all sites of a domain into ONE configuration, so comparing two single-site hashes no longer matches (open, review r4 H4).
