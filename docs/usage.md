# Using fs-release

fs-release offers two reusable workflows. A repo adds two tiny caller files; everything else lives here.

| Workflow | Purpose |
|----------|---------|
| `.github/workflows/ci.yml` | fmt, clippy, tests, our coding rules (ADR 0043), licence check |
| `.github/workflows/check.yml` | the push gate: fmt, clippy pedantic, tests, coding rules (below) |
| `.github/workflows/release.yml` | plan, build per platform, SBOM, signatures, provenance, **draft** release |

Internal building blocks (`_release-*.yml`, `actions/*`) are not meant to be called directly.
All third-party actions are pinned to commit SHAs (the version is in a comment); Dependabot
or a manual bump keeps them current (ADR 0026).

## CI

```yaml
# .github/workflows/ci.yml
name: ci
on:
  push: { branches: [main] }
  pull_request:
permissions:
  contents: read
jobs:
  ci:
    uses: FreeSynergy/fs-release/.github/workflows/ci.yml@v1
    with:
      msrv: "1.85"          # optional
```

Inputs: `working-directory` (default `.`), `features` (`all` = `--all-features`, `default`, or a list),
`msrv` (extra `cargo check` on that toolchain), `apt-packages`, `runs-on`, `rules` (default true),
`rules-url`, `licences` (default true).

What runs, in order: `cargo fmt --check` (no `--all`, coding.md section 7), `cargo clippy --all-targets
--all-features -- -D warnings`, `cargo test --all-features`, `scripts/check_rust_rules.py` fetched from
`FreeSynergy/fs-documentation` (`main`), then `cargo deny check licenses bans sources` with
[`configs/deny.toml`](../configs/deny.toml) (allow-list MIT, Apache-2.0, BSD, ISC, Zlib, Unicode, MPL-2.0 ...;
GPL/AGPL/LGPL/unknown are rejected) unless the repo has its own `deny.toml`. Advisories are reported as a warning only.
The rules checker is downloaded without a token, so `fs-documentation` must be readable (public) or `rules-url` must point elsewhere.

## Check (every push)

`check.yml` is the strict gate. Copy [`examples/callers/check.yml`](../examples/callers/check.yml) to `.github/workflows/check.yml`
of a repo; it runs on every push. It needs no secrets and only `contents: read`. In order:

1. `cargo fmt --check`
2. `cargo clippy --workspace --all-targets --all-features -- -D warnings -W clippy::pedantic`
3. `cargo test --workspace --all-features`
4. `python3 -I scripts/check_rust_rules.py <root>` (ADR 0043), taken from this repo at the same ref as the workflow (`actions/rust-rules`), no download

Inputs: `working-directory` (default `.`), `apt-packages`, `runs-on`, `test-env`, `software-gpu` (boolean, default false). Unlike
`ci.yml` there is no MSRV or cargo-deny step and no feature selection: everything is checked with all features. Repos can run both.

`software-gpu: true` is for repos with window tests (fs-texts, fs-tasks, fs-components, ...): it installs exactly
`mesa-vulkan-drivers libvulkan1 fonts-dejavu-core fonts-noto-core` (lavapipe is the software graphics adapter, the fonts make the
rendered text the same on every runner). It is merged with `apt-packages` into one list without duplicates, so a repo needs only
`software-gpu: true` instead of naming the packages.

## Release

Push a tag `vX.Y.Z` (SemVer; a pre-release such as `v1.0.0-rc.1` is marked as pre-release). The tag version must equal the
`version` in `Cargo.toml`, otherwise the plan job fails. The tag is the approval (we set it only after the picture OK and the review YES); the `publish` job then makes the draft a **pre-release** once every file and signature checks out.

```yaml
# .github/workflows/release.yml
name: release
on:
  push: { tags: ["v*"] }
permissions:
  contents: write
  id-token: write
  attestations: write
  packages: write   # required by the optional container job (GitHub checks it even when skipped)
jobs:
  release:
    uses: FreeSynergy/fs-release/.github/workflows/release.yml@v1
    with:
      binaries: capnp-compiler
      kind: cli
      app-id: net.freesynergy.CapnpCompiler
    secrets:
      GPG_PRIVATE_KEY: ${{ secrets.GPG_PRIVATE_KEY }}
      GPG_PASSPHRASE: ${{ secrets.GPG_PASSPHRASE }}
```

The caller must grant the three permissions; a called workflow can never have more than its caller.
Full examples: [`examples/callers/`](../examples/callers/) (fs-capnpc as CLI, fs-registry as daemon with OCI image).

### Inputs

| Input | Default | Meaning |
|-------|---------|---------|
| `binaries` | empty | comma separated binary names; **empty only together with `kind: library`** (an empty list without it is an error) |
| `kind` | `cli` | `cli`, `daemon`, `app`: selects default formats (below); `library`: library mode (see below) |
| `app-id` | | reverse-DNS id; needed for AppImage, Flatpak, macOS bundle |
| `app-name`, `description`, `name` | Cargo values | display name, one-liner, artefact base name |
| `cargo-package` | | `-p` for workspaces; preferred owner of the binaries it provides (msi uses it too). Binaries of other packages are still built and packaged per their own package |
| `ignore-binaries` | | binaries of workspace packages under `crates/` that are deliberately not packaged (`*` = all). Without it, such a binary missing from `binaries` fails the plan |
| `working-directory`, `features` | `.`, | Cargo workspace dir, `--features` for release builds |
| `linux-targets` | `x86_64,aarch64` | empty disables Linux |
| `windows-targets` | `x86_64` | `aarch64` adds a zip only; empty disables Windows |
| `macos` | `true` | universal build |
| `linux-formats`, `windows-formats`, `macos-formats` | by kind | override, see [formats.md](formats.md) |
| `smoke`, `smoke-args` | `true`, `--version` | after the build, install every `.deb` in `debian:13` and `ubuntu:24.04` and the Windows packages (zip, msi, setup.exe) on the Windows runner, start each binary with `smoke-args` (must exit 0) and remove it again; a failure stops the release before the draft exists |
| `apt-packages` | | build dependencies on Linux |
| `flatpak`, `flatpak-manifest` | off | Flatpak bundle; the app repo owns the manifest |
| `container`, `containerfile`, `container-image`, `container-push` | off | OCI image from the Linux binaries; push only when explicitly set |
| `android`, `ios` | off | TODO placeholders (first Blinc app); see [fdroid.md](fdroid.md) |
| `attest` | `true` | SLSA provenance (private repos need GitHub Enterprise; set false there) |
| `tag` | pushed tag | release a specific tag |

Packaging metadata stays in the app repo: `[package.metadata.deb]` and `[package.metadata.generate-rpm]` in `Cargo.toml`
(without them sensible defaults are generated), optional `wix/main.wxs`, `packaging/<app-id>.desktop`,
`packaging/<app-id>.png|svg` (AppImage), `packaging/icon.icns` (macOS app).

### Library mode (`kind: library`, no binaries)

A pure library has nothing to build, package or sign. Set `kind: library` and leave `binaries` **empty**; the same `release.yml` then runs in library mode
(caller example: [`examples/callers/library-release.yml`](../examples/callers/library-release.yml), pinned to a full SHA, not `@main`):

- The mode must be asked for: an empty `binaries` WITHOUT `kind: library` stays an error (an app repo that forgot or lost its `binaries` must not publish a release without packages), and `kind: library` WITH `binaries` is an error too. `plan` checks (`actions/library-release/notes.py`) that the tag is `vX.Y.Z` (SemVer), that the tag version equals the
  `[workspace.package] version` of `Cargo.toml` (or `[package] version`) and that `CHANGELOG.md` has a **non-empty** section `## X.Y.Z` (or `## <name> X.Y.Z`).
  Any other state is red before a job starts.
- The platform jobs, `finalize` (SBOM, signatures, checksums, provenance) and `publish` are skipped; there are no artefacts, so **no binary packages and no
  signature are needed**, and no secret is passed. Only the job `library` runs, with `contents: write` and its own `GITHUB_TOKEN`.
- `library` creates a draft whose notes are exactly that CHANGELOG section (not generated notes) and publishes it as **Pre-release** (0.0.N is always a pre-release,
  ADR 0064); `dry-run: true` creates nothing; a release that already exists is never touched. The job checks out the full history and refuses a tag whose commit is
  not an ancestor of `origin/main` (as the release job of fs-components does; also in `dry-run`).
- **Repeat after a failure:** the draft is created first and published second. If publishing fails after the draft exists, an unpublished DRAFT of the tag stays. A new run
  does NOT overwrite it: it recognises the draft, names it in the error (`an unpublished DRAFT of this tag is left over`) and stops. Publish it by hand
  (`gh release edit <tag> --draft=false --prerelease`) or delete it (`gh release delete <tag>`) and run again.
- Self test with counter-checks and mutations: `scripts/test_library_release.sh` (wrong tag name, version differs from the tag, missing or empty section, a platform or
  signing step in library mode all turn it red).
- The library actions arrive with the pin round after the merge (`scripts/pin-refs.sh <sha>`): a caller can use library mode only with a SHA that contains it.

### Approval for the signing job (GitHub environment)

The job that holds the signing key (`finalize` in `_release-finalize.yml`, the action `sign-files`) runs in the GitHub **environment** `release-signing`
(`environment: release-signing`, on that job only). With a required reviewer, a run stops there until the owner approves it, so no agent account can sign
alone, also after the agents work under their own account. The owner sets this up on GitHub (nothing in this repo does it):

1. Repository -> Settings -> Environments -> New environment `release-signing`.
2. **Required reviewers**: the owner's own account (never the account of an agent); switch on "Prevent self-review" only if the run is started by another account.
3. **Deployment branches and tags**: "Selected branches and tags" with the tag rule `v*` (no branch rule), so only a tag run can reach the key.
4. **Environment secrets**: `GPG_PRIVATE_KEY` and `GPG_PASSPHRASE` as secrets of THIS environment (not repository or organisation secrets); delete them from
   the repository secrets. `scripts/set-signing-secrets.sh --env` does steps 1 to 4 (see "Setting it up with the script" below).
5. The same for every repo that signs: the mirror workflow of a wrapper repo (template `mirror.yml`, for example fs-zentinel) and `site.yml` of the site repo
   get `environment: release-signing` on their signing job too, and nothing else.
6. **Every repo that calls `release.yml` needs its OWN environment `release-signing`** (steps 1 to 4 in THAT repo): the environment belongs to the repo in which the
   run happens, not to fs-release. If it is missing there, GitHub creates it at the first run **without any protection rule**, the repository secrets that are handed
   on still arrive, and the release is signed WITHOUT approval and without anyone noticing. Create it before the first run after the pin round. Callers found in the
   local clones of the organisation (read-only scan of `.github/workflows/release.yml`, state 2026-11-16; `gh search code` returned nothing, so the scan is local and
   must be repeated against GitHub before the pin round): fs-calendars, fs-capnpc, fs-files, fs-groups, fs-maps, fs-messages, fs-people, fs-packages, fs-broadcast,
   fs-tasks, fs-draw, fs-texts. Further callers: fs-components (library mode, planned), the mirror workflows of the wrapper repos and `site.yml`.
   When all callers are converted, remove the handing-on of `secrets.GPG_*` in `release.yml` and in the caller examples, so that only environment secrets act.

#### Setting it up with the script (step 6, per repository)

The person who sets it up must be an **admin of every target repository** (GitHub answers 404 otherwise) and holds the key passphrase file; no agent runs this
(rule: no agent access to the key or to servers). The reviewer is a login that is NOT an agent account and has access to the repository.

```
# 1. look first: only prints the gh calls (no secret values, no reading, no gpg), changes nothing
REVIEWER=<login> bash scripts/set-signing-secrets.sh --env --dry-run
# 2. for real; asks per repository before deleting the old repository secrets (--yes asks nothing)
REVIEWER=<login> bash scripts/set-signing-secrets.sh --env
# 3. tag protection of fs-release itself (no secrets, no REVIEWER)
bash scripts/set-signing-secrets.sh --ruleset-tags fs-release
```

Default list of `--env` (the 9 callers of `release.yml`): fs-texts, fs-people, fs-calendars, fs-tasks, fs-groups, fs-maps, fs-capnpc, fs-packages, fs-files;
`--repos texts,packages` (program names, with or without `fs-`) overrides it. Per repository, in this order: (1) PUT the environment `release-signing` with the
required reviewer (`REVIEWER`, a required argument or variable: there is no default; the login becomes a numeric id) and deployment only for named refs
(creates a missing environment and hardens an existing unprotected one); (2) tag rule `v*` (listed first, never created twice); (3) `GPG_PRIVATE_KEY` and
`GPG_PASSPHRASE` as environment secrets (values only on standard input, never an argument, never printed); (4) only after BOTH are set and after your
confirmation: delete the repository secrets of the same names (an error in between deletes nothing); (5) a read-only check (environment, reviewer, tag rule,
secret names, repository secrets gone): any deviation ends with rc 1. `--repo-secrets` is the old mode (repository secrets), deprecated and unprotected,
kept only on request.

`prevent_self_review` is ON by default: the account that starts a run cannot approve it, so a second pair of eyes is real. The script compares the reviewer with the
logged-in account by numeric id (logins are case-insensitive) and checks that the reviewer is a `User` (an organisation is refused, rc 2). `--ruleset-tags <repo>`
creates (or updates) the ruleset "tags v* immutable": target `tag`, `refs/tags/v*`, rules `deletion`, `non_fast_forward` and `update` (a tag cannot be deleted or moved,
not even to a successor commit; there is NO `creation` rule, so a new tag can still be created). It has no bypass actor: to remove a wrong tag, the owner must switch
the ruleset off first.

**Variant b (reviewer FreeSynergy = logged-in gh account = the account that pushes the tags): only with `--allow-self-review`.** Without it every run waits
forever (the script refuses with rc 2, rightly). RISK: everyone who holds the FreeSynergy token (agents too) can give the approval; until the accounts are
separated, the only technical protection is the tag rule `v*` (plus the ruleset with `update`). The protection stays ORGANISATIONAL, not technical: GitHub cannot tell
who approves with the FreeSynergy token; the hard bar in the Claude Code settings (Claude and agents NEVER give a deployment or environment approval) only binds
Claude and agents and is no GitHub protection rule.

**WARNING, unsigned draft:** the action `sign-files` does NOT sign when the key is empty; it only warns (rc 0). If the environment secret does not arrive in the nested call
(`release.yml` -> `_release-finalize.yml`), then after the repository secrets are deleted a draft is created silently unsigned. Check after the first test tag: the log of
`finalize` shows "Signing key fingerprint" and NO line "NOT signed". Only then convert further repositories or delete repository secrets.

Checklist before the first real run: (1) `REVIEWER=FreeSynergy bash scripts/set-signing-secrets.sh --env --allow-self-review --dry-run` and read it;
(2) for real only `--repos texts`, answer `n` to the delete question (repository secrets stay, rc 1 expected); (3) test tag in fs-texts: the approval appears, the log shows
"Signing key fingerprint: ...372A583D..." without "NOT signed"; (4) then fs-texts again, answer `y`; (5) the rest of the list without `--yes`, the check output without DEVIATION.

The signing job also checks that it runs for a tag (`github.ref_type == 'tag'`) and that the checkout is the commit of that tag (`git rev-parse HEAD` = `github.sha`).
A mirror workflow should check the same two things before it signs. Not proven until the first real run: that the environment secrets reach the job inside the
reusable workflow (a job with `uses:` cannot carry an `environment`, so the environment sits on the called job); the first run is a `dry-run` with the owner watching.

Structure test: `scripts/check-signing-env [files...]` (also run by `scripts/test_check_signing_env.sh`, which has counter-checks): a job with `sign-files` without
`environment:` is RED, and so is any other job that touches `GPG_*` secrets (only the call of `release.yml` / `_release-finalize.yml` may hand them on). Pass the mirror
and site workflows as arguments to check them as well.

Check of the setup on GitHub (report only): workflow `release-umgebung-pruefen`, script `scripts/check_release_env.sh` (self test `scripts/test_check_release_env.sh` with a mock `gh`;
it makes GET calls of `gh api` only and refuses every write flag; secrets appear by name only). The helper of the strategy runs it (agents do not call `gh`):
`scripts/check_release_env.sh --org FreeSynergy --env release-signing --tag v0.0.10 --sha a084f8b96ecac9c1eccbeca9244ae7fa2bd9a38a [--repos texts,packages,...] [--strict] [--out file]`.

| Column | Meaning |
| --- | --- |
| `env` | the environment exists (`environments/release-signing`; only a 404 means `fehlt`) |
| `reviewers` | reviewers in the protection rule `required_reviewers` (must be at least 1) |
| `tag_rule` | deployment branch policy with a tag policy `v*` |
| `env_sec` / `repo_sec` | `GPG_PRIVATE_KEY` and `GPG_PASSPHRASE` among the environment secrets / repository secrets (2/2; both places complete is a hint "double holding") |
| `release` | the repo calls `release.yml` of fs-release (info; `unbekannt` without access) |
| `result` | `OK` (environment, reviewer, tag rule, both environment secrets), `WARN` (environment without protection or secrets only in the repository), `FEHLT` (404; the repo calls `release.yml` or `release` is `unbekannt`), `nicht nötig` (404 and no workflow calls `release.yml`; not counted as not OK) |

Section fs-release: `rulesets` (evaluated only on status ok and an array; a 404 or an empty/odd answer = "Tag-Schutz unbekannt"; `[]` = "Tag-Schutz fehlt") and, with `--tag` and `--sha`, whether the tag points at the expected commit. Exit code: 0 = report made (WARN and FEHLT are the
finding), 1 = with `--strict` at least one repo is not OK (`nicht nötig` excluded) or the tag is a MISMATCH, 2 = call or gh error (401, 403, 5xx, network: never reported as "fehlt").

### Pipeline

1. `plan`: validates the tag (SemVer) and Cargo version, builds one JSON plan.
2. `linux`, `windows`, `macos`, `flatpak`, `container`, `mobile`: one job (matrix) per platform, artefacts uploaded as `dist-*`.
3. `finalize` (the only job that sees the secrets): SBOM, deb/rpm signatures, `FreeSynergy-release-key.pgp`,
   `SHA256SUMS.txt`, detached `.asc` for every file, SLSA provenance, draft release.

### Versioning the workflows

Inside fs-release, nested workflows and composite actions are referenced as `FreeSynergy/fs-release/...@<full commit SHA>` (a reviewed state of `main`)
(GitHub cannot say "same ref as my caller" across repositories). Before tagging a release of fs-release run
`scripts/pin-refs.sh v1.0.0`, commit, tag. Callers may also use `@main` while developing. Inside fs-release itself (`.github/workflows/*.yml`) the references are pinned to the full commit SHA of a reviewed state of `main` (the jobs with the signing key must never run a moving ref); raise the SHA on purpose when the actions change.

In ANOTHER repository (a program or library that calls fs-release): `scripts/pin-refs.sh --dir <repo> v0.0.10` rewrites only the `uses: FreeSynergy/fs-release/<path>@<ref>` lines of `<repo>/.github/workflows/*.yml` (`@main`, an old tag or a commit become the given tag or SHA), leaves other actions (`actions/checkout@...`), comments and all other files alone, prints the changed files and exits 0 (rc 2 without `.github/workflows`); a second run changes nothing. A pin is never lowered: with a tag as `<ref>`, a `uses:` line that is already on a higher tag stays (`release.yml@v0.0.11` against v0.0.10), checked line by line. The lift runs it as the step `pins` (`workflows/lift.yaml`) with the target `v<version of the fs-release crate>` (the release tag; the environment variable `FS_LIFT_PIN_TAG=vX.Y.Z` overrides it). `check.yml@v0.0.10` together with `strict = true` in `.fs-lock-sources.toml` is required only at the V1+ lift.

**Mandatory after every merge that changes `.github/workflows/_release-*.yml` or `actions/`:** a commit cannot contain its own SHA, so `release.yml`, `ci.yml` and `check.yml` still point at the older commit and the signing job (`GPG_PRIVATE_KEY`) runs whatever refs that older commit holds. Two levels exist (`release.yml` -> `_release-*.yml` -> `actions/`): a change in `actions/` needs two pin rounds (first `_release-*` onto the new state, then `release.yml` onto that pin commit). The SHA is a fast-forward commit, not a merge commit, so say so in the commit text. Raise the SHA to that commit in a second, reviewed commit (`scripts/pin-refs.sh <sha>`) and check that `git grep '@main' .github/workflows` is empty at the commit the SHA names. The flows `release-flow` and `umbenennen` name this step in their release steps.

### Local checks

`actionlint`, `shellcheck -x -S warning scripts/*.sh tests/packaging/*.sh actions/*/*.sh` and `scripts/test-packaging.sh`
(run it under `limited.sh`; the `lint` workflow runs all three). The packaging tests run the real `plan.py` and `package.sh` on
throwaway workspaces; the golden files in `tests/golden/` pin the output for the single-package fs-capnpc.

## Workspaces with several binary packages (one deb and one rpm per package)

`binaries` may list binaries that live in different Cargo packages, e.g. `binaries: fs-tasks-app,fs-tasks-server`
with `fs-tasks-app` in package `tasks-app` and `fs-tasks-server` in `tasks-server`. fs-release then

- builds each package on its own: `cargo build -p <package> --bin <binary>` (never one package for all binaries),
- builds **one `.deb` and one `.rpm` per package** (`cargo deb -p`, `cargo generate-rpm -p <package dir>`), named by
  the package manager's own rules (`tasks-app-<ver>-1.x86_64.rpm`, `tasks-server-<ver>-1.x86_64.rpm`),
- keeps **one** `tar.gz`, AppImage, Arch package, zip and macOS artefact with all binaries, named after `name`
  (set `name`, otherwise the first package's name is used).

A repo with one package behaves exactly as before (no `-p`, same file names). Set `name` when a repo has several packages.

**App depends on server.** fs-release does not invent dependencies; the app repo declares them in the manifest of the
app package, where the rpm and the deb are described anyway:

```toml
# app/Cargo.toml
[package.metadata.generate-rpm]
assets = [{ source = "target/release/fs-tasks-app", dest = "/usr/bin/fs-tasks-app", mode = "755" }]
[package.metadata.generate-rpm.requires]
tasks-server = "*"                 # or ">= 0.1.0"

[package.metadata.deb]
depends = "$auto, tasks-server"    # cargo-deb
```

A package without a `generate-rpm` table gets a default one (its binaries to `/usr/bin`); a package with its own table is
left untouched, so add `assets` there (the default is not merged in). Note: `features` is passed to every package build, filtered to the
features each package declares; a feature no package declares fails the plan run.

## Required package metadata

The plan step fails early unless every package that owns binaries declares `authors`, `description`,
`license` and `repository` in `Cargo.toml` — `cargo-deb`, `cargo-wix` and the rpm builder need them.
Declare `rust-version` too; the CI `msrv` input should match it.


## After the draft: Pre-release (automatic), catalog (pulled)

When `finalize` succeeds, `release.yml` runs `publish`: draft -> Pre-release, only when SHA256SUMS.txt, every file and every
`.asc` are attached; an already published release is left alone. It uses the repo's own `GITHUB_TOKEN` (callers already grant
`contents: write`); no secret, nothing cross-repo. `dry-run: true` runs the guards only.
The catalog is raised by fs-catalog itself, within 15 minutes (see DESIGN.md "Release to catalog").

Recommendation: switch on "Immutable releases" (repo Settings, General) in every program repo, so a published release and its
assets can never be changed or replaced, not even by an owner. Not switched on by this workflow; the repo owner does it.
