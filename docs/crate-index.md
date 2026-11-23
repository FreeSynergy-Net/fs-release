# Crate index: the registry `freesynergy` (repo `FreeSynergy-Net/fs-crates`)

Why: cargo knows version ranges only through a registry. With a registry, a new type in `fs-types` no longer forces ~25 repos to re-tag (decision ENTSCHEID-kaskade B2, 23 Nov. 2026). Consumers write `fs-types = { version = "~0.0.N", registry = "freesynergy" }`.

## What is in the repo `fs-crates`

- `config.json` (once, by the owner of the repo): `{ "dl": "https://raw.githubusercontent.com/FreeSynergy-Net/fs-crates/main/crates/{crate}/{crate}-{version}.crate" }` (no `api`: nobody runs `cargo publish` against it).
- the index: one file per crate in the layout of the cargo registry index (`fs/-t/fs-types`, one JSON line per version: name, vers, deps, features, `cksum` = SHA-256 of the `.crate`, yanked).
- `crates/<crate>/<crate>-<version>.crate`: the packaged crates.
- `allowed-sources.txt`: the allow list `owner/repo <first version> <package>...` (packages in dependency order). It is part of the trust set (see below).
- `.github/workflows/index.yml`: the workflow that fills the registry (template: `templates/fs-crates/` in fs-release, with README on how it gets there).

## How a crate gets in: PULL, not push (decision of the strategy, review K-2/K-3)

No secret (`FS_CRATES_TOKEN` does not exist) and no write right across repos. The workflow runs IN `fs-crates` every 15 minutes (`schedule`) and by `Run workflow` (`workflow_dispatch`, one click for the user). Nobody dispatches it from outside, so the bot needs no write right on `fs-crates`.

- Job `package` (`contents: read`): `fs-crate-index scan`: `git ls-remote --tags` of the repos on the allow list; for every new tag `vX.Y.Z` (not older than the first version of the allow list) it checks: the tag is annotated, the tag commit is on `main` of the source repo, the tag equals the version of every listed package, the version is not in the index yet, and the tag was not moved after an earlier entry (the tag commit is in the commit message of the entry: `source: owner/repo vX.Y.Z commit <sha> tag-object <sha>`; a different commit now is red and nothing is entered again). Then `cargo package --locked` for all listed packages in ONE call (interdependent packages verify against each other, before any of them is in the index), then the semver gate: `cargo-semver-checks` v0.51.0 (pinned binary, sha256 checked) with `--release-type minor --baseline-root <the newest older .crate of THIS checkout>`; red = no entry. The first version of a crate has no baseline (the report says so; a missing `--semver-checks` is reported as NOT RUN). Output: `plan.txt` and the `.crate` files as an artifact, together with the prebuilt tool.
- Job `enter` (`contents: write`, `needs: package`): `fs-crate-index apply`: no cargo, no build. It re-checks every plan line against the allow list and the tag (a tag that moved between the jobs is red), enters the crates one after the other in the order of the allow list (S6: a later package needs the earlier one already in the index, else "dependency not found in index"), one commit each, then ONE push, never forced. The token lives only in the push step; it is never in the environment of `cargo build` or `cargo package` (a `build.rs` of a dependency cannot see it).
- Nothing is written by hand. A version already in the index is never replaced. A rerun does nothing new (idempotent).
- The format work is done by the library `reg-index` 0.6.0 (the library of `cargo-index`, MIT OR Apache-2.0, pinned `=0.6.0`); our tool is the adapter with the checks above. Index format: `reg-index` writes v1 lines; the adapter moves every feature whose values use `dep:` or `?/` into `features2` and sets `v: 2` (registry-index standard). A crate with an optional dependency (like `fs-records`) is therefore fine. Test: three crates that mirror fs-types, fs-conformance, fs-records, resolved by a fresh consumer with the feature switched on.
- The workflow cannot be called today: the old `uses:` pin `78992eea` does not know `crates/fs-crate-index`. The template carries the placeholder `FS_RELEASE_PIN`, which is replaced by the pin tool (`pin-refs`) or by hand after the merge (README of the template).

### What the release flow waits for

`ff-und-tag` / `release-beobachten` waits for the index line (a read, no right needed): `fs-crate-index wait --registry-url <fs-crates> --package fs-types --version <v> --minutes 20`. After the time it prints the blocker: "BLOCKED: ... The index workflow in fs-crates is switched off or late (GitHub switches scheduled workflows off after 60 days without activity in the repo; cron can be late). One click: open <workflow page>, press `Enable workflow` if it is off, then `Run workflow`." The tool exists and is tested. NOT YET DONE: the call in `workflows/ff-und-tag.yaml` step `release-beobachten` (summary line 223 ff.) and its Rust helper `release-watch`; that is a separate order (workflow-pflege) because it changes a workflow other flows depend on.

## Consumer

`.cargo/config.toml` of the repo (and of every generated program):

```toml
[registries.freesynergy]
index = "https://github.com/FreeSynergy-Net/fs-crates"
```

This string must be the same everywhere (it is also the `index-url` the tool writes into the entries: a dependency of a registry crate on the same registry is stored as "same registry" only if the URLs are equal) and it is the value of `[only_from] fs-types` in `scripts/lock-sources.toml`; the lock shows it as `registry+https://github.com/FreeSynergy-Net/fs-crates`.

IMPORTANT for the publishing repo (K-4): the path dependencies between the published crates need `registry = "freesynergy"` next to `version` (`fs-types = { path = "../fs-types", version = "0.0.7", registry = "freesynergy" }`) and `.cargo/config.toml` with the registry. Without `registry`, cargo treats the dependency as one from crates.io and the index line points there. The test in `crates/fs-crate-index/tests` uses exactly this form. fs-types `unreviewed/k4-version-gate-r1` (1dc9b435) has `path` and `version` only: to be fixed there.

## Threat model (honest)

The registry protects against MISTAKES (a wrong tag, a replayed or moved tag, a tag not on main, a `.crate` that was changed afterwards by accident: cargo refuses it because the checksum of the index does not match). It does NOT protect against someone who can change the workflow, the toolchain or the scripts, and not against someone who can write to `fs-crates`: the checksum lies in the same repo as the `.crate`, so whoever can push there can swap a crate and its checksum together. Therefore the trust set is: the people who can write to `fs-crates` or change its workflow or `allowed-sources.txt`. Cargo checks the checksum against the index, not against a signature. The pull path makes the set smaller than the push path: no secret in the publisher repos, no right across repos; a wrong publisher repo can enter nothing but a real, valid tag on its own `main`.

## Click list for fs-crates (the user, once; one line per click)

Repo and files:
1. Create the repo `FreeSynergy-Net/fs-crates`: public, default branch `main`, no template.
2. Copy `templates/fs-crates/` of fs-release into it (README of the template lists the steps; replace `FS_RELEASE_PIN`).
3. Settings -> Actions -> General -> Workflow permissions: `Read repository contents` (the workflow asks for `contents: write` itself in the one job).
4. Settings -> Actions -> General -> Actions permissions: allow only actions created by GitHub and the pinned ones (as in the other repos).
5. Settings -> Actions -> General -> "Allow GitHub Actions to create and approve pull requests": OFF.
6. Create the team `FreeSynergy-Net/registry-owners` (the name used in `.github/CODEOWNERS`; the people who may change the allow list).
Protection of `main` (ruleset "main"): this is about NO force-push and NO deletion, as in ADR 0093; no "require pull request" on the whole branch, because the workflow pushes entries directly (a pull-request rule would turn every entry red):
7. Settings -> Rules -> Rulesets -> New branch ruleset, target `main`: tick "Restrict deletions".
8. Same ruleset: tick "Block force pushes".
9. Same ruleset: bypass list stays empty (the Actions token needs no bypass for a normal push).
Review of the allow list (CODEOWNERS): a path rule cannot be done by a ruleset that lets the workflow push directly, so the review happens at the pull request:
10. Settings -> Rules -> Rulesets -> second ruleset "reviewed files", target `main`, "Require a pull request before merging" + "Require review from Code Owners", restricted to the paths `allowed-sources.txt`, `.github/**`, `config.json` [U: GitHub path restrictions in rulesets were not checked for this repo type; if they are not available, the review of those files stays a rule for the owners and CODEOWNERS only marks the reviewers].
11. Do NOT put the bot account into `registry-owners` and give it no write right on `fs-crates` (a bot with write could push directly and the smaller trust set would be gone).
After the first run:
12. Actions -> index -> `Run workflow` once; it must be green with `entries=0`.
13. Add the calendar note: if no release happens for 60 days, the workflow may be switched off (Actions -> index -> `Enable workflow`).
Until these clicks are done the phase counts as "initial phase, not protected" (ADR 0093:111).

## raw.githubusercontent.com as the download place [U: not measured]

Only `dl` points there; the index itself is fetched by cargo as a git repo from `github.com`. [U] means untested: a live download, the content type, a Windows runner and a proxy were never measured; this is assumed from reading, not from a run. Dependencies and limits: (1) every build of every consumer downloads each `.crate` once per cargo home (cargo caches), so a CI without cache downloads per run; raw.githubusercontent.com has no published contract for this use, its rate limits are not guaranteed and a block shows as a failed download; (2) it follows the default branch name `main`: a rename breaks every old lock-free build (a lock stores the checksum, not the URL, so existing locks keep working after the URL is fixed); (3) behind a proxy or on a Windows runner the host must be allowed; (4) for a PUBLIC repo it needs no token. Fallbacks, each a change of `config.json` only: release assets of `fs-crates` (cargo follows redirects; `dl` then needs a FIXED release tag as the collecting place, and GitHub limits the number of assets per release), or a static host in front of Zentinel. Cargo's git index fetch from github.com uses a fast path through `api.github.com` (60 requests per hour without a token per IP) and falls back to a full git fetch if that is limited: slower, not red.

## Not verified

- No run on a real GitHub runner (the template, the pin, the artifact hand-over, the push). The tests use `file://` repos and a throw-away registry; workflow syntax is checked only as YAML here (actionlint is not installed locally).
- `cargo-semver-checks` itself was not run here: the tests use a stand-in program to prove the call (`--baseline-root`, `--release-type minor`, once per crate, red = no entry). Whether the baseline crate of `fs-conformance` / `fs-records` resolves its registry dependency in the real run is untested.
