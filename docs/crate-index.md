# Crate index: the registry `freesynergy` (repo `FreeSynergy-Net/fs-crates`)

Why: cargo knows version ranges only through a registry. With a registry, a new type in `fs-types` no longer forces ~25 repos to re-tag (decision ENTSCHEID-kaskade B2, 23 Nov. 2026). Consumers write `fs-types = { version = "~0.0.N", registry = "freesynergy" }`.

## What is in the repo `fs-crates`

- `config.json` (once, by the owner of the repo): `{ "dl": "https://raw.githubusercontent.com/FreeSynergy-Net/fs-crates/main/crates/{crate}/{crate}-{version}.crate" }` (no `api`: nobody runs `cargo publish` against it).
- the index: one file per crate in the layout of the cargo registry index (`fs/-t/fs-types`, one JSON line per version: name, vers, deps, features, `cksum` = SHA-256 of the `.crate`, yanked).
- `crates/<crate>/<crate>-<version>.crate`: the packaged crates.

Written by ONE job: `.github/workflows/crate-index.yml` (reusable; composite action `actions/crate-index`; tool `crates/fs-crate-index`). It runs only in a TAG run (`github.ref_type == 'tag'`, the tag commit is on main, tag = version of the package), packages with `cargo package --locked`, adds the index line and the `.crate` in ONE commit and pushes with the token `FS_CRATES_TOKEN` (contents write on `fs-crates` only). Nothing is written by hand. A version that is already in the index is never replaced. The format work is done by the library `reg-index` 0.6.0 (the library of `cargo-index`, MIT OR Apache-2.0, pinned `=0.6.0`); our tool is only the adapter with the guards. `reg-index` writes index format v1 only: a crate with `dep:` or `?/` in a feature is refused (it would be indexed wrongly).

Gate before the entry: the workflow has the place for `cargo-semver-checks` (input `semver-checks`, red = no entry). It is not wired yet (K-4 builds the call for fs-types); `true` is refused until then.

## Consumer

`.cargo/config.toml` of the repo (and of every generated program):

```toml
[registries.freesynergy]
index = "https://github.com/FreeSynergy-Net/fs-crates"
```

This string must be the same everywhere (it is also the `index-url` the tool writes into the entries: a dependency of a registry crate on the same registry is stored as "same registry" only if the URLs are equal) and it is the value of `[only_from] fs-types` in `scripts/lock-sources.toml`; the lock shows it as `registry+https://github.com/FreeSynergy-Net/fs-crates`.

## Threat model (honest)

The registry protects against MISTAKES (a wrong ref, a wrong version, a branch run, a `.crate` that was changed afterwards by accident: cargo refuses it because the checksum of the index does not match). It does NOT protect against someone who can change the workflow, the toolchain or the scripts, and not against someone who can write to `fs-crates`: the checksum lies in the same repo as the `.crate`, so whoever can push there can swap a crate and its checksum together. Therefore: write access to `fs-crates` only for the bot token of the release job, no other writers, branch protection on `main`. Cargo checks the checksum against the index, not against a signature.

## raw.githubusercontent.com as the download place

Only `dl` points there; the index itself is fetched by cargo as a git repo from `github.com`. Dependencies and limits: (1) every build of every consumer downloads each `.crate` once per cargo home (cargo caches), so a CI without cache downloads per run; raw.githubusercontent.com has no published contract for this use, its rate limits are not guaranteed and a block shows as a failed download; (2) it follows the default branch name `main`: a rename breaks every old lock-free build (a lock stores the checksum, not the URL, so existing locks keep working after the URL is fixed); (3) behind a proxy or on a Windows runner the host must be allowed; (4) for a PUBLIC repo it needs no token. Fallbacks, each a change of `config.json` only: release assets of `fs-crates` (cargo follows redirects), or a static host in front of Zentinel. Cargo's git index fetch from github.com uses a fast path through `api.github.com` (60 requests per hour without a token per IP) and falls back to a full git fetch if that is limited: slower, not red.
