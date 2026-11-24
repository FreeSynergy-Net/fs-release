# Using fs-release

fs-release offers two reusable workflows. A repo adds two tiny caller files; everything else lives here.

| Workflow | Purpose |
|----------|---------|
| `.github/workflows/ci.yml` | fmt, clippy, tests, our coding rules (ADR 0043), licence check |
| `.github/workflows/check.yml` | the push gate: fmt, clippy pedantic, tests, coding rules (below) |
| `.github/workflows/_windows-check.yml` | the Windows gate: fmt, clippy pedantic, tests on an explicit Windows runner (below); called through `examples/callers/windows.yml` |
| `.github/workflows/release.yml` | plan, build per platform, SBOM, signatures, provenance, **draft** release |

Internal building blocks (`_release-*.yml`, `actions/*`) are not meant to be called directly; the one exception is `_windows-check.yml` (a caller of its own, see "Windows gate").
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
    uses: FreeSynergy-Net/fs-release/.github/workflows/ci.yml@v1
    with:
      msrv: "1.85"          # optional
```

Inputs: `working-directory` (default `.`), `features` (`all` = `--all-features`, `default`, or a list),
`msrv` (extra `cargo check` on that toolchain), `apt-packages`, `runs-on`, `rules` (default true),
`rules-url`, `licences` (default true).

What runs, in order: `cargo fmt --check` (no `--all`, coding.md section 7), `cargo clippy --all-targets
--all-features -- -D warnings`, `cargo test --all-features`, `scripts/check_rust_rules.py` fetched from
`FreeSynergy-Net/fs-documentation` (`main`), then `cargo deny check licenses bans sources` with
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
   Frozen folder (the only exemption, rule "module doc has a `Pattern:` line" only): a file `locks/<folder name>.sha256` at the repo root,
   checked in, with the lines `ordner: <repo-relative path>` (its last part = `<folder name>`) and `sha256: <folder hash>` (the output of
   `tools/hash-ordner.sh <folder>`); other lines are comments. Example `locks/groups_v1.sha256`:
   `ordner: crates/fs-group-directory/src/groups_v1` / `sha256: 91c0f8b0849d85b1038b8b09f511e27134c1ddfeb0ef2eae42dc8579b3d2faf3`.
   Valid only while the folder exists and its hash matches: any change to the folder, a missing lock file, a wrong name or a path outside the folder turns the rule back on.

Inputs: `working-directory` (default `.`), `apt-packages`, `runs-on`, `test-env` (KEY=VALUE lines; the keys CI, GITHUB_*, RUNNER_*, FS_*, GATES_*, GIT_*, LD_*, PATH, BASH_ENV and ENV are refused: red), `software-gpu` (boolean, default false), `full` (boolean, default false), `profile` (text, default empty, see below). Unlike
`ci.yml` there is no MSRV or cargo-deny step and no feature selection: everything is checked with all features. Repos can run both.

**Scope of the test gate (CI cost).** On a branch `unreviewed/*` step 3 runs only the crates that own a file changed against main (merge base with
`origin/main`) plus every workspace crate that depends on one of them (dev-dependencies and transitive ones included): `cargo test -p a -p b ...`
(`scripts/affected_crates.py`, handed over by the `diff-base` input of `actions/gates` as `FS_GATES_DIFF_BASE`). fmt, clippy, the rules, the lock
check and the file checks always run on the whole workspace. `main`, tags, every other ref and `full: true` test the whole workspace, and so does
every unclear case (Cargo.lock, a workspace manifest, toolchain or cargo config, a file that no crate owns, an error): the gate output says which
(`gates: test NARROWED ...` / `gates: test runs the WHOLE workspace (...)`). Locally `scripts/gates.sh` is unchanged.

**Test profiles (input `profile`).** The test gate runs `cargo nextest run --profile <name>` with a profile out of the CALLER's `.config/nextest.toml` (names: `quick`, `time`, `security`, `platform`, `picture`, `release`; see `workflows/testlauf.yaml` [NEXTEST-PROFILES]). Empty `profile` = by ref: a tag runs `release`, every other ref (`unreviewed/*`, `main`, `workflow_dispatch`) runs `quick`; the narrowing to changed crates on `unreviewed/*` and the input `full` work as before and combine with the profile. A caller that does not define the profile is not red: the log says `test gate: profile quick not defined here (...), running default` and everything runs as before, so repos without profiles stay green. The name is checked (`[a-z][a-z0-9_-]{0,31}`), anything else is refused (rc 2). Doc tests do not use a profile. On `main` a second pass with profile `security` runs over the whole workspace when the caller defines it (else skipped with a log line); `--no-fail-fast` is always set, so it wins over `fail-fast = true` in a profile. Switch on in a caller: nothing to do for the default; to force a profile: `with: { profile: release }`. A nightly full run belongs in the caller, not here, for example (NOT built in):

```yaml
# .github/workflows/nightly.yml in the caller repo
# on:
#   schedule: [{ cron: "17 2 * * *" }]
# jobs:
#   nightly:
#     uses: FreeSynergy-Net/fs-release/.github/workflows/check.yml@<pin>
#     with: { profile: release, full: true }
```

**Release plan probe.** A repo with its own `.github/workflows/release.yml` that calls the fs-release release workflow gets the real `actions/plan`
run on the inputs of that call (`binaries`, `ignore-binaries`, `cargo-package`, `working-directory`, `kind`, ...; plain values only, the tag is the Cargo
version of the package that owns the binaries). A new binary crate under `crates/` that is neither in `binaries` nor in `ignore-binaries` is red on the
branch, not first at the tag. Needs the check workflow of a pin that contains it.

`software-gpu: true` is for repos with window tests (fs-texts, fs-tasks, fs-components, ...): it installs exactly
`mesa-vulkan-drivers libvulkan1 fonts-dejavu-core fonts-noto-core` (lavapipe is the software graphics adapter, the fonts make the
rendered text the same on every runner). It is merged with `apt-packages` into one list without duplicates, so a repo needs only
`software-gpu: true` instead of naming the packages.

## Windows gate (`_windows-check.yml`)

One reusable workflow instead of a copied `windows.yml` per repo. A repo copies [`examples/callers/windows.yml`](../examples/callers/windows.yml)
(`on: push` without a branch filter, so `unreviewed/*` is covered, plus `workflow_dispatch`; `uses: ...@<tag>`; no secrets, so no `secrets: inherit`).
The pure-Rust crate is linted and tested on Windows, never cross-built. Steps: checkout, `setup-rust` (toolchain from `RUST_VERSION`), `cargo fmt --check`,
`cargo clippy ... --all-targets --locked -- -D warnings -W clippy::pedantic`, `cargo test ... --locked --no-fail-fast` (same flags as `check.yml`).

| Input | Default | Meaning |
|-------|---------|---------|
| `runner` | `windows-2022` | runner label, always explicit (no `*-latest`) |
| `packages` | empty = `--workspace` | space separated crate names, each becomes `-p <name>` |
| `features` | `--all-features` | feature flags for clippy and test (`--features a,b`, empty = defaults) |
| `clippy-extra` | `-W clippy::pedantic` | lint flags after `-D warnings` |
| `no-fail-fast` | `true` | `cargo test --no-fail-fast` |
| `cache` | `true` | Rust cache on; `false` = cold run (passed to the new input `cache` of `actions/setup-rust`; a tag proof needs `workflow_dispatch` with `cache: false`) |
| `timeout-minutes` | `90` | job timeout (a hanging clippy or test must not run for hours) |

Pins: inside the workflow `setup-rust` is pinned to the same commit SHA as every other internal reference (`scripts/pin-refs.sh` raises it at release time); the CALLER pins the
workflow to a fs-release TAG that contains the file (the first one is the tag after v0.0.10), never `@main`. The inputs are checked for plain words before they reach a command line.
Test: `tests/packaging/windows_check.sh` (rules plus 16 mutants: `*-latest`, `@main`, missing `--locked`, caller on `@main`, caller with secrets, no `permissions`, no check step, no runner check, no job `timeout-minutes`, `${{ inputs.` in a run block (multi-line and one-line), `cache` without effect, the `if:` of `actions/setup-rust` gone, caller without `cache` pass-through, caller without the `workflow_dispatch` input).

The `runner` input must match `windows-<number>`; `packages`, `features` and `clippy-extra` may hold only letters, digits and `_ , / = . : -` and spaces (checked with a bash regex, a line break is refused too).
The Rust cache comes from `setup-rust` (Swatinem/rust-cache, pinned SHA; key = cache-key `windows` plus rust-cache's own hash of Cargo.lock, toolchain and runner OS). The cache only speeds a run up: the evidence for a tag is a run that is green without a cache hit (a cold run). The input `cache` (default true) switches the cache off for a cold run: the evidence for a tag comes ONLY from such a run (run the caller by hand: `workflow_dispatch` offers the input `cache`, the caller passes it on), never from a run that is green with a cache hit. `actions/setup-rust` got the input `cache` (default true; false skips the rust-cache step); since `actions/` changed, the SHA in `_windows-check.yml` must be raised by a pin round to a commit that contains it (before that the pinned action ignores `cache`). Changes under `actions/` need two pin rounds (see "Mandatory after every merge" below: version commit, round 1 onto the SHA of the ff head, round 2, then the tag exactly `v0.0.11`); that is a release step.
`features` defaults to `--all-features`; a repo whose optional features pull heavy dependencies (GPU, Blinc, e.g. fs-packages) sets `features` in its caller.
The caller example pins `@v0.0.11`: `pin-refs.sh` does not change `examples/`, so the release tag must be exactly `v0.0.11` (else the example is changed by hand).

Follow-up runs (text only here, the repos themselves are not touched by this run; source `.scratch/katalog-plan/INVENTAR-ci-workflows.md`, 2026-11-17):
- Windows callers (replace the copied `windows.yml` by the caller): fs-types, fs-components, fs-files, fs-texts; fs-packages (branch `unreviewed/packages-windows-ci`, variant with `windows-2022`). New callers (Rust repos without a Windows gate): fs-activities, fs-broadcast, fs-calendars, fs-capnpc, fs-draw, fs-forms, fs-groups, fs-identity, fs-interfaces, fs-jscalendar, fs-maps, fs-messages, fs-objects, fs-people, fs-registry, fs-release, fs-resources, fs-tasks.
- Rolling runner `ubuntu-latest` to a fixed version (maintenance item 10): fs-components `check.yml` (line 21), fs-icons `check.yml` (line 10); `release.yml` of Blinc is not ours (ticket via git-bug, if at all).
- `@main` pins of fs-release to a tag (v0.0.10 or the new one, `scripts/pin-refs.sh --dir <repo> <tag>`): fs-identity, fs-interfaces, fs-registry (`check.yml`), fs-tasks (`check.yml`; its `release.yml` is already on v0.0.10), fs-types (`check.yml` and the `setup-rust@main` of its `windows.yml`). SHA pins (fs-files, fs-components) move to the tag at the next lift.

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
    uses: FreeSynergy-Net/fs-release/.github/workflows/release.yml@v1
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
| `kind` | `cli` | `cli`, `daemon`, `app`: selects default formats (below); `module`: ONE Linux RPM `fs-module-<name>` (`name` = host module name, exactly one binary = the service; built by `fs-blueprint package module`; Windows is a TODO, WIN-b, no Windows job yet); `data`: ONE noarch RPM of `scaffold.toml` by `fs-blueprint package rpm` (no `binaries`, no cargo build; the tag must equal the blueprint version); `library`: library mode (see below) |
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

Optional hook `packaging/prepare.sh`: when it exists and is executable (`-x`), `package-linux` and `package-windows` run it in the working folder of the program repo BEFORE the packages are built (log group `prepare`), so the repo can fetch what the packages need from its one source (fs-starter: the host icon from fs-icons). Make it executable in git (`git update-index --chmod=+x`): on Windows the check `-x` is also made by Git Bash, and a file without the executable bit is skipped silently. The hook runs in the job `build` (`contents: read`, no secrets, no signing key; those exist only in `finalize`), i.e. with the rights of `cargo build`.

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
`scripts/check_release_env.sh --org FreeSynergy-Net --env release-signing --tag v0.0.10 --sha a084f8b96ecac9c1eccbeca9244ae7fa2bd9a38a [--repos texts,packages,...] [--strict] [--out file]`.

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

Inside fs-release, nested workflows and composite actions are referenced as `FreeSynergy-Net/fs-release/...@<full commit SHA>` (a reviewed state of `main`)
(GitHub cannot say "same ref as my caller" across repositories). Before tagging a release of fs-release run
`scripts/pin-refs.sh v1.0.0`, commit, tag. Callers may also use `@main` while developing. Inside fs-release itself (`.github/workflows/*.yml`) the references are pinned to the full commit SHA of a reviewed state of `main` (the jobs with the signing key must never run a moving ref); raise the SHA on purpose when the actions change.

In ANOTHER repository (a program or library that calls fs-release): `scripts/pin-refs.sh --dir <repo> v0.0.10` rewrites only the `uses: FreeSynergy-Net/fs-release/<path>@<ref>` lines of `<repo>/.github/workflows/*.yml` (`@main`, an old tag or a commit become the given tag or SHA), leaves other actions (`actions/checkout@...`), comments and all other files alone, prints the changed files and exits 0 (rc 2 without `.github/workflows`); a second run changes nothing. A pin is never lowered: before anything is written, every existing pin (tag or SHA; `@main` is raised) must resolve in the fs-release clone (`FS_RELEASE_REPO`, default: the repository of the script) and `<ref>` must be equal to it or a descendant (`git merge-base --is-ancestor`); a higher, unrelated or unknown pin, or a `<ref>` the clone does not know, makes the whole run red (rc 3) and nothing is written. The owner (`FreeSynergy` or `FreeSynergy-Net`, with a left boundary: `Not-FreeSynergy/fs-release/...` is a foreign owner and stays untouched) is rewritten to `FreeSynergy-Net`. The lift runs it as the step `pins` (`workflows/lift.yaml`) with the target `v<version of the fs-release crate>` (the release tag; the environment variable `FS_LIFT_PIN_TAG=vX.Y.Z` overrides it). `check.yml@v0.0.10` together with `strict = true` in `.fs-lock-sources.toml` is required only at the V1+ lift.

Lift libraries (V1+, ADR 0092/0093): `fs-release lift <repo> --components vX --interfaces vY --types vZ` or, general, `--repo NAME=vX.Y.Z` (repeatable; NAME is one of fs-components, fs-interfaces, fs-types, fs-objects, fs-rpc, fs-jscalendar; `--interfaces` and `--types` are aliases, a name given twice or an unknown name exits with 2). Step `deps` runs `scripts/set_components_tag.py <tag> <repo> <name>` for each; the lift of `fs-interfaces` never touches a dependency named `fs-types` (or with `package = "fs-types"`), `fs-types` goes through `scripts/set_components_tag.py --move fs-types <tag> <repo>`, which sets `git = "https://github.com/FreeSynergy-Net/fs-types"` and the tag (a fs-types with another URL is an error, nothing is guessed). Step `lock` checks that `Cargo.lock` holds `github.com/FreeSynergy-Net/<name>?tag=<tag>#` for every asked library; with fs-types asked, `lock-sources` runs the lock check with `FS_LOCK_ONLY_FROM=1` (fs-types only from the repo FreeSynergy-Net/fs-types). Step `commit` (with `--commit`) sets the identity `FreeSynergy <support@freesynergy.net>` for author and committer explicitly (also against `GIT_AUTHOR_*` in the environment) and ends the message with `Co-Authored-By: <FS_LIFT_COAUTHOR>` (default `Claude Sonnet 5.5 <noreply@anthropic.com>`; a value that is not `Name <mail@host>` stops the step before anything is committed). The pin target of `pins` is `v<crate version>` or `FS_LIFT_PIN_TAG`.

V1+ lift of a program (`lift-v1plus`, ADR 0092/0093, decision b7): three steps, in this order. (1) `api-umstellung` (specification, no script): from fs-interfaces v0.0.27 there is no Cargo feature `rpc` any more; the rpc helpers are in fs-rpc (git `https://github.com/FreeSynergy-Net/fs-rpc`, tag v0.0.2). A program that uses `fs-interfaces` with feature `rpc` (servers and clients) deletes the feature in Cargo.toml, adds `fs-rpc = { git = "https://github.com/FreeSynergy-Net/fs-rpc", tag = "v0.0.2" }` and changes the paths as in the README of fs-rpc v0.0.2: `fs_interfaces::rpc::X` becomes `fs_rpc::X`, `fs_types::describe::CatalogServer` becomes `fs_rpc::catalog::CatalogServer`; the generated Cap'n Proto modules still come from fs-interfaces. There is NO `cargo check --locked` here: the lock has to change. (2) The normal `lift` with `--repo fs-rpc=v0.0.2` and the other V1+ tags (fs-types, fs-interfaces, fs-objects, fs-jscalendar, fs-components), called with the inputs `commit: false`, `online: true` and `FS_LIFT_V1PLUS=1`: it updates the lock, checks it without the feature `rpc` (steps `lock`, `clippy`, `tests`) and does NOT commit (its step `commit` takes no `*.rs` files). (3) The own step `umstellung-commit` (specification, human or agent): after a green lift it stages by name `Cargo.toml`, the changed `*.rs`, `Cargo.lock` (and changed `.github/workflows/*.yml`) - never `git add -A`, only the paths of `git diff --name-only` that are expected - and commits them in ONE commit as `FreeSynergy <support@freesynergy.net>` with the footer `Co-Authored-By: <model> <noreply@anthropic.com>`; build and tests are green before the commit. The switch comes first because `lift` would otherwise run with the new fs-interfaces and the old feature and be red before the switch; `lift.yaml` stays equal to the code, `release-flow` has no switch step. Order of the libraries: fs-types -> {fs-interfaces, fs-objects, fs-jscalendar, fs-components} -> fs-rpc -> programs.

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

## Licence gate (`licence` in `scripts/gates.sh`)

Every repo that runs `check.yml` (through `actions/gates` and `scripts/gates.sh`) gets the gate `licence` (rc line `gate licence: rc=<n>`): `cargo-deny check licenses bans sources` over the whole workspace (`--workspace --locked`), with ONE policy for all repos, `configs/deny.toml`. A `deny.toml` of the repo is NOT read by the gate (no policy per repo; `actions/licence-check`, used only by `ci.yml`, still lets a repo file win). The allow list is not widened for convenience: a red finding (a dependency under AGPL, GPL, LGPL, SSPL, BUSL ... or without a licence) is a decision of the strategy, never an entry. `[sources]` allows exactly crates.io (git index and sparse form), our registry `freesynergy` (`https://github.com/FreeSynergy-Net/fs-crates`) and git repos of the organisations `FreeSynergy-Net` and `FreeSynergy` (the old name is still in locks until the address wave is through; `lock-sources` reports it on its own). Advisories are not part of the gate (moving database; `actions/licence-check` only warns).

The tool is pinned in ONE place, `scripts/cargo-deny.pin`: version `0.22.0` and the sha256 of the release asset `cargo-deny-0.22.0-x86_64-unknown-linux-musl.tar.gz` (the digest GitHub shows for the asset, same way as `cargo-semver-checks` in `actions/semver-baseline`). `scripts/licence-gate.sh` downloads that asset (Linux x86_64, to `$FS_TOOLS_DIR`, else `$RUNNER_TEMP`, else `~/.cache`), checks the sha256 on EVERY run, unpacks it fresh and requires `--version` to be exactly the pinned one; `FS_CARGO_DENY=<path>` names another binary that must still report the pinned version. There is no `cargo install` and no unpinned tool. NOT RUN is never green: no binary, a failed download, a wrong sha256 or version, no policy file or no `Cargo.toml` is exit 64 with the own line `licence gate NOT RUN: <reason>`; a finding of cargo-deny keeps its rc (a bit mask: advisories 1, bans 2, licenses 4, sources 8; hence NOT RUN is 64, never one of these). The `nachpruefung:` line of the list is `bash {SCRIPTS}/licence-gate.sh {ROOT}`.

Threat model: the gate protects against mistakes (a dependency with a wrong licence or from a wrong source), not against a hostile caller. It runs after the `build.rs` and tests of the caller (the `licence` step comes after the cargo steps in the same job), so code of the caller could already have run. `FS_CARGO_DENY` checks only the version of the given binary, not its sha256.

Self test `scripts/test_licence_gate.sh` (lint job): stub cases for NOT RUN, the exact call and the rc; with the REAL pinned cargo-deny a workspace with a path dependency under `AGPL-3.0-only` (`tests/fixtures/licence-gate/agpl`) is red and one under `MIT OR Apache-2.0` (`.../ok`) is green; mutants: AGPL on the allow list, the call without `licenses`, the step removed from `gates.sh`. Not covered: the three mirror repos `fs-kanidm`, `fs-zentinel`, `fs-tuwunel` (no Rust code of our own) get no cargo-deny; the licence of the upstream program is named in the release text. The gate reaches a caller only when its `check.yml` pins an fs-release state that contains it (the next pin round).

## Gate output and branch CI (workflow maintenance run 8)

Rule [GATE-PIPEFAIL]: gates always run with `set -o pipefail` (or print their rc); a gate output is never shortened through `tail` or `head` (the pipe swallows the exit code of the gate); `check_rust_rules` is filtered only with `grep -E 'ERROR|errors'` AND a visible rc; check scripts come from the fs-release tag that the CI file of the repo pins. `scripts/gates.sh` follows it (`set -euo pipefail`, one `gate <name>: rc=0` line per gate, a failing line and rc on stderr) and a test pins it (`tests/workflows_pflege12.rs`).

Rule [GATES-WAECHTER] (technical enforcement of [GATE-NICHT-AENDERN]: nobody changes files while `gates.sh` runs): `scripts/gates.sh` holds the lock `.gates.lock` in the worktree root (PID inside; a second run in the same worktree stops with "gates läuft bereits"; a lock whose PID is dead is replaced; the lock is removed on exit and on abort) and hashes the working tree at the start and at the end (`git status --porcelain=v1 --untracked-files=all`, `git rev-parse HEAD^{tree}`, sha256 of every tracked and untracked file; `target/` and other ignored files and the lock are left out; tools: git, sha256sum, diff). If the two states differ, gates is red (rc != 0) with the fixed text "Arbeitsbaum während der Prüfung geändert" and the full list of changed files. Fixes go into a new commit and a new gates run, never into the tree of a running one. A test of the gates script itself runs a copy of it or waits for the commit. The snapshots live in `<git dir>/gates-guard/` (not in /tmp); the next run sweeps snapshots of dead PIDs there; a lock whose PID is alive but not a `gates.sh` (reused PID) is replaced. Test mode skips only the cargo and check steps and runs the same end lines (snapshot, compare, `gate guard: rc=0`) as the real run. Code: `scripts/lib_gates_guard.sh`; test `scripts/test_gates_guard.sh` (test mode `GATES_ONLY_GUARD=1`, no cargo).

Branch CI = `lint` + `branch-ci`: a push to `unreviewed/**` runs the Rust gates of fs-release itself (`.github/workflows/branch-ci.yml`, calls `scripts/gates.sh`; the reusable `ci.yml` / `check.yml` are unchanged). Before a tag, read both runs at the exact head.

## Scaffold of the kit (`geruest-heben`, `modul-neu`, `bausatz-neu`, `ausnahme-gespraech`)

Run 9, specification (the `@fs` calls have no script yet, a real run is not proven; source `katalog-plan/PFLEGE9-AUFTRAG-workflows.md`, plan section c2).
`workflows/geruest-heben.yaml` sorts code of the programs into scaffold crates: state (heads by `git ls-remote`), trait draft, review through the own Opus step `trait-review` (not `trait-delta`),
crate scaffold, help part, then the sort-in step WITHOUT a build ("Zunächst NICHT kompilieren (spart Slots)"; every copied file starts with `//! Herkunft: <repo>@<kopf>:<pfad>`, the programs stay unchanged),
the unclear list `UNKLAR-<crate>.md`, `talk-vorbereiten`, the conversation `talk` (user and Opus, gate `human`) and only then, when the input `schicht_fertig` is true, the bundled build through `build-slot.sh`
(`set -o pipefail`, rc per gate, no output cut with `tail`/`head`). `modul-neu` and `bausatz-neu` create a module or a data kit only when nothing exists (existing first),
enough programs need it (module 2, specialist module 1, kit 2) and every field has a standard. `ausnahme-gespraech` (replaces `ausnahme-beleg`): an `[[exception]]` exists only with `talk`,
the protocol of a conversation of the user and Opus; no agent decides it alone. Every check step is `<step>-check` plus `<step>-again` (at most two repeats, then `eskalation`).
The step `doku-hilfe` (help = docs) proves a marked help part of the program page in fs-documentation (en and de, input `programmseite`); it stands in `modul-neu`, `geruest-heben` and `grundfassung`.
`lift` and `lift-v1plus` stay as they are (they raise library tags; `geruest-heben` is the sorting in). `geruest-lift` is created INACTIVE (until `fs-blueprint` exists): it raises `skeleton`, generates, calls `lift` unchanged and checks.

## Pin cascade (`pin-kaskade`)

`workflows/pin-kaskade.yaml` (specification, the `@fs` calls have no script yet, a real run is not proven; test `tests/workflows_pin_kaskade.rs`): ONE pin cascade after a new fs-types tag (input `ausloeser-tag`, here v0.0.3) in DAG order (ADR 0092/0093; inputs `glieder`, `ab`, `workdir`): fs-interfaces (capnp regenerated, `catalog_roundtrip`) -> fs-objects -> fs-jscalendar -> fs-rpc (fs-starter-L1 `coded_stub`) -> fs-components (fs-settings lift, F1 catalog, `WITHOUT_COMPONENT`) -> fs-people -> fs-tasks (pilot) -> fs-calendars; the order is DERIVED from the real Cargo.toml pins (table with evidence in the workflow header, test keeps it); `rev` pins to FreeSynergy repos (`fs-conformance`, `fs-people-directory`) are lifted onto tags in the same Glied (step `rev-pins`), an fs-types still coming from the repo fs-interfaces is moved by the lift; the end check covers `fs-conformance` too. Per Glied: the existing `lift` unchanged (pin, Cargo.lock by cargo), strict lock check, gates in both environments (CARGO_BUILD_JOBS=4, BUILD_PRIORITY=0, [GATE-*]), branch CI at the exact head, Opus review, ff and tag only through the existing `ff-und-tag`, then the next Glied. A red Glied ABORTS (`eskalation` to the strategy, no skipping, no parallel build of dependent Glieder), resume with `ab`; end check: the same fs-types version in every lock, one source in the graph. Rule [KEINE-KALTEN-FLACHPROBEN-BIS-AUFRAEUMEN] noted.
`pin-kaskade` scripts (run 10, b7/c0 orders; the other `@fs` calls of the flow stay specification): `scripts/kaskade-lage` (trigger tag, fs-release v0.0.11, heads, Windows caller and Windows run per head, resume `ab`), `kaskade-plan` (order from the real Cargo.toml pins incl. `rev`, wrong sources, cycle), `kaskade-glied` (current Glied, `libs`, ensures `.fs-lock-sources.toml` with `strict = true` and `only_from_enabled = true` in the Glied worktree, a raise of `strict = false` is reported), `kaskade-glied-fertig` (ls-remote proof, `offen_vorher`), `kaskade-fremde-pins` (`rev` pins onto owner tags, only Cargo.toml, known foreign sources from `workflows/kaskade-fremdquellen.txt`: `blinc-app-fix`, `blinc-layout-fix`), `lock-gleich` (end check at the tags, also the tool file), `kaskade-zyklen` (cycles between repos from the pins of the checkouts, self test `test_kaskade_zyklen.sh`; scope: `--repos`/`--list`, default `plan.offen` plus every pinned repo with a checkout, read as `--root/<prefix>repo`; the step passes root = `input.worktrees` and `--prefix pk-`), `gates-komplett` (gates.sh from the fs-release tag the CI pins, local + CI=true + flat clone without tags, through `build-slot.sh`). FLAT RUN OF fs-release: the test `released_changelog_sections_equal_their_tags` demands tags and a full history, so after the flat clone the run of the repo `fs-release` (only this repo) fetches them in an own step (`git fetch --unshallow --tags origin`, `--unshallow` only while the clone is shallow); the test runs and must be green, there is NO exception any more. The list of named exceptions (`AUSNAHMEN` in the script) is EMPTY since v0.0.12; the MECHANISM stays (an entry = repo + run + test + `bis`: the run gets `FS_GATES_SKIP_TEST` -> `scripts/gates.sh` runs `cargo test ... -- --skip <test>` and says so; printed on every run as `SKIPPED test ... by named exception ...`; listed under `ausnahmen` with `gruen_fuer_test: false`; a `bis` that is tagged in fs-release or a remote that cannot be read = red) and is tested with a sample entry in the own copy of the self test. Every other repo keeps its flat clone WITHOUT tags. `cargo fmt --all` writes into the work tree (a tree it changes is red: commit it first); the flat target is `.scratch/target/<repo>/<folder>-flach` with `<repo>` = the repo name of the `origin` URL and `<folder>` = the folder of the checkout (every clone has its own flat target, [EIGENES-TARGET]). `[patch]` tables of a Cargo.toml are NOT read by `kaskade-plan`, `kaskade-fremde-pins` and `lock-gleich` (a patch source is not a dependency edge); `workflows/kaskade-fremdquellen.txt` also lists `FreeSynergy/fs-capnpc` (capnp-compiler, in fs-interfaces) and `FreeSynergy/Blinc` (blinc_* crates, in fs-components): pinned by tag, not Glieder, no findings. Library `scripts/kaskade_lib/`, JSON on stdout, rc 0 / 1 red / 2 bad input; self tests `scripts/test_kaskade_plan.sh`, `test_kaskade_pins.sh`, `test_kaskade_gates.sh` (local bare repos, mutants). `FS_KASKADE_BASE` points the remote at local repos. Rule B4 (no exceptions): every Glied gets the caller `_windows-check.yml@v0.0.11` before its tag (step `windows-aufrufer`, specification), the cascade starts only after fs-release v0.0.11.

Follow-up runs (not in this bundle, v0.0.12 and later): (1) the central lock check in fs-release `check`/`gates` with inputs `strict` / `only_from` and a STRICT default (today every Glied carries its own `.fs-lock-sources.toml`); (2) the Windows caller run for the pins that still stand on `@main` (fs-identity, fs-interfaces, fs-registry, fs-tasks (check), fs-types) and on `@v0.0.9` (fs-files); (3) the script `windows-aufrufer`.


## Review template frontend (`frontend-nur-komponenten`)

Review template frontend (rule of the user, lane 3): a frontend holds ONLY the plugging together of components from fs-components plus theme, style, menu tree and texts; logic, own widgets or data conversion are a finding. Step `frontend-nur-komponenten` (input `frontend_crate`, Opus reads only the crate; a finding goes to the conversation `frontend-befund-talk`, user + Opus, at most two repeats, then `eskalation`) before the picture acceptance in `grundfassung` (before `picture-ok`), `frontend-round` (before `acceptance`) and `frontend-umbau` (before `release`); the description is in `review`. A missing component is built in fs-components (or the nearest existing one stands in); logic belongs into the program or the modules through the contract.

## Look first (`vorhanden-pruefen`)

Mandatory first step `vorhanden-pruefen` (rule of the user: before anything is built, look whether it already exists and whether it is only extended): the first step in `geruest-heben`, `modul-neu`, `bausatz-neu`, `grundfassung`, `frontend-umbau` and `stil-neu`; it searches all repos, `unreviewed/` branches, `.scratch` and the inventories (`katalog-plan/INVENTUR-*.md`); output `befund` = `vorhanden` ("extend X") or `neu` WITH a reason and proof; a `befund` without proof is red (`vorhanden-pruefen-check`, at most two repeats); the next step reads `befund`. For scaffold crates (e.g. fs-conformance) the existing `Report` copies and conformance frames are looked at and extended first. `doku-hilfe` follows ADR 0072 [akz.] (manual in the wiki, block with purpose `help.<topic>`, one source).

## Trait delta (`trait-delta`)

`workflows/trait-delta.yaml` drafts and fixes the data interface of ONE program (capnp schema, Rust trait folder, capability) before its release. Inputs: `programm`, `trait` (`<programm>_v1`), `standard` (AS2, JSCalendar, JSContact, ... or `eigen`), `vorbild` and `features` as the reference, `ordner` (draft folder), `probe` (probe crate under `.scratch/probe/`), plus `repo`, `branch` and the docs inputs. Order of the steps (26): standard research, Opus draft, compile probe (through `build-slot.sh`, own target), Opus review (line 1 `JA` or `NEIN`, bound to the folder hash, up to three rounds), `hash-weiche` (the fixpoint applies only to the hash of the JA), the strategy sets the fixpoint (`tools/hash-ordner.sh` into `sperren/<trait>.sha256`), conformance test and fake against the fixed folder, documentation, `release-tor`. Place (binding): the trait folder `<trait>_v<n>/` lies in the program repo in the crate that carries the trait as public API (input `ziel_crate`, example `fs-people-directory/src/person_directory_v1`); `draft/` of the testdata crate only while it is a draft, and when the fixpoint is set it moves to the fixed place; the capnp contract always lies in `fs-interfaces/schema/`. The hash lock is a working fixpoint; the feature freeze is set only by the user. A locked trait is not changed by this flow: a new capability is a `<x>_v2` or a part trait (before the first release a locked trait is replaced by a v2 without further ado). The basics (`grundfassung`, `programm-grundgeruest`) may start with a minimal contract and run in parallel. A BASIC VERSION (`grundfassung`: only window + "what is coming" + docs, stores no data through the contract) may release as v0.0.1 without the trait lock, the contract is visibly marked "draft" in README/catalog; `release_frei` is mandatory before the first feature release (data through the contract) (decision b7). `kern-umbau` changes the inside of a program and keeps the trait. The `@fs` calls have no script yet (specification, a real run is not proven).

## Feature list of a program (`feature-liste`)

`workflows/feature-liste.yaml` writes the COMPLETE feature list of one program (rule [FEATURE-LISTE]) to `todo/features/<x>.md` in
fs-documentation: all functions of the models from the dossier, sorted, each marked `heute | als Nächstes | später` (field "Stufe",
format of `todo/features/people.md`), plus a bridge table `Standard | Vertrag | Was fehlt`. Inputs: `programm` (name after the naming
rule, ADR 0087), `dossier` (path; without one the step `dossier-kurz` researches a short dossier from official sources, nothing
copied, ADR 0013), `repo` (read only), `docs`, `docs_branch`. An Opus SAMPLE checks lines against code, trait fixpoint and plan
before anything is published; the references (user documentation, catalog diff) come after it, the fast-forward of the docs main
branch is the human step `veroeffentlichen`. The list is German only, not published and not checked by `check-docs.sh`; the user
documentation stays short with a picture and takes its "today" and "what comes" from the stages of the list. `grundfassung` calls the
flow as step 1b; it also runs alone. The `@fs` calls have no script yet (specification, a real run is not proven).

## Workflow maintenance run 1 (`ff-und-tag`, `branch-neu-schreiben`, `katalog`)

`workflows/ff-und-tag.yaml`: the branch CI of ALL workflows for the head (a job without progress for more than 30 minutes is cancelled and started ONCE again, then escalation), the signing entries GPG_PRIVATE_KEY and GPG_PASSPHRASE (names only), the expected runs per repo (step `erwartete-laeufe`: `check`, `typing-test` or the repo-specific ones on the branch, `ci` and `msrv` on main after the ff, `release.yml` only at the tag; the push helper checks with `gh run list` that EVERY expected run exists and is green for exactly this head; a missing run is red, never green), a fresh clone with the checklist (head, fast-forward, only FS_ALLOWED_IDENTITY, version = Cargo = lock = first CHANGELOG heading, tag free, diff only expected paths), the approval for exactly this head, push without force, annotated tag, `git ls-remote` proof, release watch (all jobs green, published and not a draft, signature `signed` with fingerprint, attestation). `workflows/branch-neu-schreiben.yaml`: identity (author and committer), WIP, equal content, `--force-with-lease` with the old head as lease SHA only on an own `unreviewed/*` branch, a new head needs a new JA; mode chain: branches one after the other onto the new main (CHANGELOG: all entries of main first under ONE `## Unreleased`; file counter counted again; committer identity checked after the rebase). `workflows/katalog.yaml`: entry new, raised or removed in packages.toml; asset name HTTP 200, release published and never a draft, binary in the tarball (kind `data`/`module`: the RPM holds the module folder or `programs/<name>/`, `rpm -qpl`); `gen_catalog.py --check`, unittest, py_compile; fresh review; ff by the strategy; sign run (the path filter starts it only for packages.toml and scripts/*.py, otherwise cron or `workflow_dispatch`); proof in catalog.xml (`fs-sha256`, renewed `.asc`, verify green); the message `does not match the given sha256` means the raw cache: after 5 minutes start sign again. Several catalog branches go one after the other (both `## Unreleased` sections of the CHANGELOG kept, the new one on top). `catalog-bump` stays the event path after a release; `katalog` does not call it. In `release-flow` the step `version-abgleich` (before the review request) compares version, lock, CHANGELOG heading and the pin of the `uses:` lines to the newest chain; `review` reads the branch CI (`ci-lesen`) and has a delta mode. All `@fs` calls are specification (no script yet).

## Identity check before ff (`adresse-pruefen`)

Before a branch goes to `main` by fast-forward, the steps `adresse-pruefen` of `release-flow` (after `lock-pruefung`, before `review-request`) and `agent-run` (after `fmt`, before `review`) check the range `main..HEAD`: `git log --format='%h%x09%ae%x09%ce' main..HEAD` lists author and committer address of every commit, and only the identity in the key `FS_ALLOWED_IDENTITY` (value `support@freesynergy.net`) is allowed. Every other address, as author or as committer, is a hit without exception (also machine and bot addresses): the step is red, the branch is rewritten first (author and committer = the allowed identity) and the check runs again. The step names the commit, never an address. The check sits before the review because a rewrite makes a new head and a JA is bound to exactly one head (a new head makes a JA invalid); after the tag a rewrite would leave a public tag on the old commits. Agents and bots must commit with the allowed identity (FS_ALLOWED_IDENTITY); any other author or committer address in `main..HEAD` counts as a finding and the branch is rewritten before the review. Implementation: `scripts/identity-check [<root>] [<base>]` (shell caller, no Python; base `$FS_IDENTITY_BASE`, default `origin/main`; rc 0 green, 1 hit, 2 missing base; the one exception is a remote without `main`, the first check of decision 9f, see "Identity gate without main" below, and there the token of `GH_TOKEN` is sent only to an origin `https://github.com/`) is the gate `identity` of `scripts/gates.sh`, so the local gates and `branch-ci` of this repo are red the same way. NOT yet in the callers: their `check.yml` runs `actions/gates@e596ed65` (v0.0.18), which has no identity gate; it takes effect in the CI of the callers only after the pin of `actions/gates` is raised to a tag of fs-release that contains `scripts/identity-check` (follow-up after the next fs-release tag: raise all caller pins). Until then `fetch-depth: 0` in `check.yml` only costs time. Fields are separated by TAB, so an address with a blank, a TAB or a line break cannot shift them (the line and field count is checked); only the ADDRESSES of author and committer are checked, NOT the names (the name `FreeSynergy` is set by the worktree and clone step, not enforced by the gate); the log is read with `git --no-replace-objects` (a local `git replace` cannot hide a commit); in CI (CI case: `CI` or `GITHUB_ACTIONS` set and not false, `GIT_DIR` and related variables refused) `FS_IDENTITY_BASE` other than `origin/main` and `FS_ALLOWED_IDENTITY` other than the default are rc 1 (an error, not a warning; `check.yml` takes `test-env` lines ONLY as `TEST_<NAME>=<value>` (allowlist, bytes: a BOM, non-ASCII, a blank before the name, `<<` or any other name such as CI, GITHUB_*, FS_*, GATES_*, GIT_*, BASH_FUNC_*, SHELLOPTS, PS4, BASHOPTS, HOME, XDG_*, RUST*, CARGO_*, PYTHON* is red and nothing is written; logic in `scripts/check-test-env`, copied into the step, self test `scripts/test_check_test_env.sh`). What this does NOT close, honestly: a caller can still change its OWN code (build.rs, tests, its workflow files) and the pins in its own `check.yml`; the identity gate is the FIRST gate of `scripts/gates.sh`, and that order protects ONLY inside `gates.sh` (before the first cargo call); code of the caller that runs EARLIER in the same job (a committed `rust-toolchain.toml`, apt packages) is not covered by it, but by the own job `identity` of `check.yml` (N1d: `check` has `needs: identity`; the job has only the checkout and the check, no setup-rust, no rust-cache, no apt, no input of the caller except `runs-on`), by `RUSTUP_TOOLCHAIN` fixed in `actions/setup-rust` before rust-cache, and by the allowlist of the `apt-packages` input (package names only, `scripts/check-apt-packages`, self test `scripts/test_check_apt_packages.sh`); a `runs-on` that names a SELF-HOSTED runner of the caller is not covered at all (the runner belongs to the caller; `runs-on` is a free input); and `GATES_ONLY_GUARD` (skips every gate) is refused when `GITHUB_ACTIONS` is set. All of this only works in the callers after the pin of `actions/gates` is raised, only fixture repos of the self tests name another base through the local repo config key `freesynergy.identity-test-base`; self test `scripts/test_identity_check.sh` (allowed identity green, foreign author or committer red, mutant). Cause fix: the worktree and clone step (`agent-run` step `worktree`, `ff-und-tag` step `klon`) sets `user.name=FreeSynergy` and `user.email=support@freesynergy.net` so author and committer are right from the first commit; the re-look of the tag chain (`ff-und-tag` step `nachblick`) repeats the identity check and demands `git diff --exit-code` between the reviewed and the checked head. EXCEPTION (stock): history up to the base SHA of each repo partly carries the machine identity (`agents@Beelink.hnc`, `agent@localhost`); `main` and public tags are never rewritten; from there the rule holds hard (the range `origin/main..HEAD` starts at the base, so the stock is not checked). Base SHAs: fs-interfaces `main` e92ef7b027cbb4f8145c5e8fa92c81603ce6d4d7, fs-release `main` 1932e8edafbe4b17c86675a2379cda1a9cedff51, other repos still to be added.


### Threat model of the gates

The identity gate protects against MISTAKES (a wrong commit identity such as `agents@Beelink`, a forgotten setting, CI that differs from local), NOT against someone who can change the workflow, the toolchain or the scripts of the repo: who can do that needs no gate to get around, against that stand the review and the landing only after a JA. (German original of the decision 9f: "Das Identitäts-Gate schützt vor VERSEHEN, NICHT vor jemandem, der Workflow, Toolchain oder Skripte des Repos ändern kann.") Every security gate names its threat model in this documentation; a finding counts as MUST only when it lets a mistake through or contradicts this text, "a malicious caller in the same job" is expressly outside. Live in the callers only after the pin of `actions/gates`, `actions/setup-rust` and the `check.yml` pin are raised to a tag that contains these files; until then nothing of this runs there. A `pull_request` event is handled in the `identity` job (checkout of the PR head commit: the merge commit of GitHub has the committer `GitHub <noreply@github.com>` and would be red by mistake, looked at in `refs/pull/99/merge` of rust-lang/log); the gate `identity` in `gates.sh` (`scripts/identity-check`) checks the same range on `pull_request`: when `GITHUB_EVENT_NAME` is `pull_request` (CI case only), HEAD is `GITHUB_SHA` (the merge commit of GitHub) and HEAD has two parents, the tip is `HEAD^2` (the PR head), not the merge commit; if HEAD is not `GITHUB_SHA` (the PR head itself is a merge, e.g. "Update branch"), the tip stays HEAD, so the range is the same as in the job; the checkout of `check` is NOT changed (`ref:` stays, the test base stays the merge result).

**Branch rules and skipped jobs.** When `identity` is red, `check` is SKIPPED (`needs: identity`), and GitHub branch rules count a skipped job as successful. A caller whose branch rule requires only the status `check` must therefore ALSO require `identity` as a required status, or the landing step must check the overall result of the run (as `ff-und-tag` and the branch CI do). Both belong in the caller setup; `check.yml` cannot enforce it.

## Menus (`picture-acceptance`, dossier template)

Menus count as looks. `picture-acceptance` has the objective step `menue-bilder` (specification, `@fs`, no script yet): input is the
menu tree of the dossier, result `menue_bilder` = one pair model | ours per main menu (`<programm>-menue-<hauptmenue>-vorbild|unser.png`)
and per context menu (`<programm>-kontext-<ort>-vorbild|unser.png`). Class `ohne Vorbild` (dossier gate `[offen]`): a menu without a model is no missing pair; only our `unser.png` counts, never a foreign picture. No menu tree in the dossier is red (`dossier-menue-fehlt`: first
complete the dossier), a missing pair is red (`menue-bild-fehlt`). Order: picture package model | ours first, THEN build. "Rebuild"
means structure, naming, order, shortcuts and behaviour; the style comes through the style layers (frontends / style layers), it is
not copied (ADR 0013).

Dossier template (flow `dossier`, step `research`; the short dossier of `feature-liste` has the same sections): required sections
`Menübaum (vollständig)` (every main menu opened: entries, order, separators, submenus, check marks / radio groups, disabled entries,
shortcuts), `Kontextmenüs` (right click on an element and on empty space, per place), `Werkzeugleiste` (buttons in order) and
`Tastenkürzel`. Source: official documentation screenshots or the real program in a container (concept only, no code, no graphics).
Rule: what is not proven is written `nicht belegt`, never invented. `feature-liste` takes the section `Menübaum je Vorbild` into the list.

Capabilities `fs-components` must offer, so that every program defines its menus freely (a program that follows its model needs all of them):

- menu bar and menus per program, freely defined (entries, order, names)
- submenus (any depth)
- separators and groups
- shortcut display (and the binding itself)
- check marks and radio groups
- disabled entries (and hidden ones)
- context menus (on an element and on empty space)
- keyboard operation (open with Alt / F10, arrow keys, mnemonic letters, Enter, Esc)

If one of these is missing, the round does not work around it in the program: it starts a component run in fs-components (the
workflow of the component care; none exists as a file yet, so `workflow-pflege` writes it first) and builds on the result.
## Basics of a program (`grundfassung`)

`workflows/grundfassung.yaml` is the slim flow for the first version of a program (scaffold, window with name, icon and a
"what comes" list, package metadata, user documentation stub, callers, gates, picture OK, catalog proposal). Inputs: `programm`
(name after the naming rule, ADR 0087), `beschreibung`, `icon` (path; missing: the nearest existing icon, marked "vorläufig"), `frontend` (frontend id, e.g. `simple`), `art` (`grundfassung` or `feature`), `repo`. A basics release (window, "what comes", documentation, no data over the contract) may go out as v0.0.x without the trait lock (decision b7, 17 Nov.); the contract is then a draft, marked "Contract: draft" in the README and the catalog description. A feature release needs the lock `release_frei` (file `sperren/<programm>_v1.sha256`), mandatory BEFORE the first feature release; `release-sperre` and `vor-tag` check it. `was_kommt` is a RESULT of `feature-liste` (no own input) and part of the `zusammenfassung`. Placeholder rule: no PNG placeholder; a missing component is replaced by the nearest EXISTING component, honestly marked "vorläufig" and listed in the gap list ("Lücken") in DESIGN; the real component is built in the background and replaces it. Versions may currently jump (e.g. the basics directly v0.0.2); later no longer (the user announces it); version = tag. No user picture OK: the STRATEGY sees the pictures (`picture-ok`, with `dossier` and `waechter`); only `accepted == true` goes on to `katalog-vorschlag` (`bild-weiche`), otherwise back to `fenster` or `bild-eskalation`. `lehren-pruefen` carries the checks of the six basics (capabilities result type, `CONTRACT_VERSION`, all `uses` at `@v0.0.10`, `--version`/`-V`/`--help` before window and socket with `env -i`, deb `$auto` and `fs-<x>-simple (>= v)`, AppStream `compare="ge"`, Mutex and ETXTBSY protection in tests, "Linux (Windows/macOS follow)", error texts through `message_key()`/`params()` and MF2 de/en, data folder only through `create_private`/`prepare_data_dir`); `vor-tag` demands the branch CI of all workflows and no open Opus condition before the tag (push and tag through `ff-und-tag`).
The user creates the repository; the flow prepares the scaffold in an own worktree and makes no `gh` call. The callers are
`check.yml` and `release.yml` at `@v0.0.10` (`scripts/pin-refs.sh --dir <repo> v0.0.10`), `.fs-lock-sources.toml` has
`strict = true`, the AppStream metadata is schema 1 without `<icon>` until fs-packages 0.0.19. The catalog entry is only a diff
for fs-catalog (not pushed); the tag is set through the strategy after the picture OK and the review YES. The frontend is data, not an own crate: the step `frontend-crate` (older name) writes the manifest `packaging/frontends/<frontend>.metainfo.xml` and the forms description under `forms/`; the host that opens it is `fs-starter` (ADR 0105 (a)), `fs-forms-run` is never started by hand (ADR 0094 draft, ADR 0099 point 12). `vertrag-standard` (flow `trait-delta`) and `feature-liste` (flow `feature-liste`, list in fs-documentation `todo/features/<programm>.md`) run before the scaffold; the release waits for the locked trait. First version: Linux only (`macos: false`, `windows-targets: ""`). The `@fs` calls have no
script yet (specification, a real run is not proven). `programm-grundgeruest` stays for the feature work.

### Look at once, test of a hanging instance, bundle (workflow maintenance run 3)

`grundfassung` shows more than "what comes" (rule [AUSSEHEN-SOFORT], b7, 17 Nov.): the step `hauptansicht` (after `fenster`, `main-view-ai:0.0.18@fs`, specification) adds the first real main view in the look of the model (draw: board with tool bar and canvas, messages: conversation list, history and input, resources: list and detail, forms: form list and editor, activities: timeline, broadcast: dashboard; decisions, workflows, data, paint likewise). Only existing fs-components blocks; example data only in `fs-<x>-simple/src/preview.rs` (typed, `preview:` ids, no real personal data), switched off by `mode(capabilities)`; the preview marker appears once (an Alert plus the title). Lessons of the model picture: navigation as a left bar with symbols, large headings, rows without badges, readable font, an empty state with symbol. The step `bildpaar` demands the picture pair model | ours (`main`, `main-empty`) with the same filter and state; without the pair and the picture OK of the strategy (b7) there is no tag (`vor-tag`). A missing component is replaced by the nearest existing one (no PNG placeholder, "vorläufig", gap list in DESIGN); versions may jump (first free number, version = tag); slot rule: write code without a slot, build only after the release of a slot (at most 2 builds).

`binary-test` has the mandatory test `server-unresponsive` ("hanging instance"): a server that does not answer ends the start in bounded time with an error (code and parameters); a missing test is red, never green (pattern: fs-broadcast `a_hanging_instance_ends_the_start_with_an_error_in_bounded_time`). Before `release` the steps `buendel-frage` and `buendel-weiche` ask the strategy "bundle complete? otherwise keep collecting" (functions, fixes, dependencies of the bundle); not complete or no answer: `buendel-sammeln`, no tag; a single release only for a blocker ([BUENDEL-RELEASE]).

`workflows/bahnen.yaml` is the lanes round of the coordinator (specification, the `@fs` calls have no script yet): the lanes (projects `bahn1-komponenten` ... `bahn5-crates`) and the board are led through `timeout 15 fs-tasks ...` only ([ARBEIT-IN-FS-TASKS]: one card per run, status current, never change the order of the cards; a hanging CLI is reported, never a server restart), remote states are proven by `git ls-remote`, at most 2 builds at the same time (night order), and the report to the strategy is bundled per lane (line 1 of the Opus verdict, the conditions word for word, the file path). Loops are counted: from the third round `eskalation`. It complements `koordinator-runde`.

### Fault diagnosis, clean-up list, onboarding, upstream request (workflow maintenance run 2)

`workflows/stoerung-diagnose.yaml` finds the cause of a fault (hang, crash, a program that does not start) read only and only in the agent's own directory (own worktree, temporary HOME and XDG, headless): never on a real server (no SSH, no root, no AI on servers) and never in the user's session ([KEIN-ZUGRIFF-AUF-NUTZERSITZUNG]); if that is not possible the flow ends with a safe instruction for the person in `fuer-nutzer.md`. Order: symptom, place check, logs, hypotheses (each with what proves and what refutes it), reproduction, proven cause, a check that no product code was changed, report with cause, proof and a proposal. The fix is a NEW run through `bugfix`, never in the same run; loops stop after three rounds in `eskalation`. Example: the hang of a second program start (BEFUND-einzelinstanz.md): the first hypothesis "handed over and ended" was refuted by the code, the cause (a wait without a deadline) was proven by a second start in the own directory.

`workflows/branches-aufraeumen.yaml` makes the clean-up LIST (branches proven merged by `git ls-remote` and `git merge-base --is-ancestor`, clones under `.scratch/review-*`, work trees, `target` folders, podman images and volumes; anonymous volumes of other runs are never touched) and a user script with `--dry-run` as the default, single names and no `rm -rf` on an unchecked variable, with an entry in `fuer-nutzer.md`. The agents delete NOTHING and never work around a refusal; the flow ends with `nutzer-fuehrt-aus`.

`workflows/menschen-aufnahme.yaml` takes a new person into the project: only data the person gave themselves, the way in by the rules of the group (gatekeeper bot without AI, vote, admin or open), welcome material and privacy note prepared by the agent, every consent given by the person themselves in a `human` step ([KEINE-ZUSTIMMUNG-IM-NAMEN-DES-NUTZERS]); without consent nothing is stored.

`workflows/upstream-anfrage.yaml` got the Blinc permission rule 98: before the draft come the kind check (only an issue, never a pull request or a comment), the proof, the duplicate search (read only) and the git-bug ticket of our repo; the text gate checks English, no AI words and no addresses; for Blinc the channel is `git-bug` without asking the user, other upstreams keep the approval of the person; the helper of the strategy sends. The `@fs` calls have no script yet (specification, a real run is not proven).

### Upstream watch (workflow maintenance run 5)

`workflows/upstream-watch.yaml` is the fixed watch step of `koordinator-runde` and `bahnen` ("always answer at the upstream"). In both flows the step `upstream-faellig` sends the round to `upstream-watch` when the last check (`upstream.letzte_pruefung`, UTC) is older than 2 hours or missing, so the watch runs every 2 to 3 hours. It reads with GET only (`gh api repos/<repo>/issues/<n>/comments`, `.../pulls/<n>/comments`, `.../pulls/<n>/reviews`, `.../pulls/<n>`) the open pull requests and issues of the git-bug list `Reported upstream` (for example project-blinc/Blinc #73, #76, #77 and the issues). A comment is new when its author is another person (our accounts are compared without upper/lower case) and it is later than the last check or its id is not remembered. For every new comment: one card in the board (`timeout 15 fs-tasks add "Blinc: Antwort auf #<n>" --project bahn5-crates --as upstream-watch`, after `fs-tasks list` so there is no double) and a short English draft of at most 5 sentences, without AI attribution, in `katalog-plan/blinc-prs/ANTWORT-<n>-<date>.md`, reported to b7. The flow sends nothing: only the helper of b7 writes the answer, after the YES of the user (`freigabe-nutzer` stands before `senden`). Without `gh`, network or a valid answer the step ends YELLOW with "Upstream nicht erreichbar" and does not move the time, never silently green. The rule of `upstream-anfrage` stays: issues short, no new issues while our own pull requests are open. The `@fs` calls have no script yet (specification, a real run is not proven). Run 6: the three calls `read-upstream-list`, `fetch-upstream` and `filter-new-comments` are real scripts (`scripts/read-upstream-list`, `scripts/fetch-upstream`, `scripts/filter-new-comments`, each with `scripts/test_*.sh`, a fake `gh` in the PATH, no network in the test). List: a text file `upstream-liste` (workflow input `liste_datei`, default `katalog-plan/blinc-prs/upstream-liste`), one line per item `<owner>/<name>#<number> pull|issue` (`#` at the start of a line = comment; repo only `[A-Za-z0-9_.-]+/[A-Za-z0-9_.-]+`, number digits only; one bad line refuses the whole list). `fetch-upstream` calls only `gh api --paginate repos/<repo>/issues/<n>/comments` per item (for pull requests also `pulls/<n>/comments`, `pulls/<n>/reviews`, `pulls/<n>`), without a method (= GET), every path checked against a fixed pattern; a missing `gh`, no network or an HTTP error (403: stop at once) = exit 3 and `Upstream nicht erreichbar` (the yellow end). `filter-new-comments`: default `eigene` = `["FreeSynergy"]` (also for `[]`), accounts ending on `[bot]` are never foreign; FIRST RUN = baseline (only fill `gemerkt`, no drafts for old comments); `gelb-melden`, `eskalation` and `merken` carry the WHOLE state (`repo`, `eigene`, `gemerkt`, `letzte_pruefung`) on unchanged and the parents merge it (merge, not replace). Tests: `tests/workflows_pflege11.rs`.

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

## Blueprint check and program from a blueprint (S5, ADR 0099)

`check.yml` has the input `blueprint` (`off` by default, `warn` or `block`) and `blueprint-rev` (the commit of fs-release the command is built from; a full SHA). With `warn` or `block`, a repo that has a `scaffold.toml` gets the step `fs-blueprint check --drift` after the gates: structure of the blueprint, the building blocks (catalog), the files of the repo, the talk records of the exceptions (a flat checkout of fs-documentation main) and the generated files (a hand change is `generated-changed`, a file under `crates/` without a head or an exception is `file-unowned`). `warn` shows the findings and stays green (for a repo that is not converted yet), `block` fails the job. Callers pinned to an older tag keep what they have. The step `generieren` of `workflows/grundfassung.yaml` replaces the AI step `geruest`: `fs-blueprint new <bauplan> --out <worktree>` checks the blueprint, generates the program and writes it; on an error nothing is written. The workflow `blueprint-e2e` proves on Linux and Windows that a blueprint becomes a program that builds and starts, that the check finds a hand change, and that the install line of `check.yml` works.

The default of `blueprint-rev` is EMPTY: the command is built from the commit of the called `check.yml` itself (the context `job.workflow_sha` of a reusable workflow, only when `workflow_repository` is fs-release of the old or the new owner), so a caller pinned to `check.yml@v0.0.18` gets the command of the tag commit. A fixed SHA in the file would have to name the tag commit, which is the commit that contains it (chicken and egg), and would fall back to an old commit by mistake. A full commit SHA (40 lowercase hex digits) as input wins; anything else, or an empty input outside fs-release, ends the step with exit code 2 before anything is installed. `scripts/test_blueprint_check_step.sh` (lint job) runs the step with a stub cargo and a stub `fs-blueprint`: modes, revisions, the default, exit codes and 23 mutants. From v0.0.13 the caller `check.yml` that `fs-blueprint new` generates passes `with: blueprint: warn`: the drift of generated repos is shown and the job stays green. `block` is not the default because every hand-written file under `crates/` is `file-unowned` (274 findings for fs-tasks, 161 for fs-people when checked) and would turn the job red; a repo switches to `block` itself when its exceptions are complete.

`fs-blueprint fmt [BAUPLAN] [--check]` (S5b, ADR 0099 point 14): writes the blueprint in its canonical form (the author's own text with the tables and keys in the order of the schema; the form of every value and the order of a list stay; the blueprint hash, which is made of the serialization of the parsed value, stays the same); atomic; `--check` only says whether the file is canonical (`fmt-changed`); comments are kept (S5c, `toml_edit`: the text is only moved into the order of the schema; a comment belongs to the line below it; the result must be the same blueprint, else `fmt-unsafe`). The generated `main` of a program holds only `PROGRAM` and `BLUEPRINT_SHA256`; `scaffold.toml` is read when the program starts (`FS_BLUEPRINT` names another file), it is not embedded.

### Gates: the error list (Sammelliste) and the partial check / Gates: die Fehlerliste und die Teilprüfung

`scripts/gates.sh` and `scripts/gates-komplett` no longer stop at the first red ([GATES-SAMMELLISTE]): every gate runs (`cargo test --no-fail-fast`, `cargo clippy --keep-going`), the rc of every step is kept and printed (`gate <name>: rc=<n>`), and ALL findings land in ONE list `fehlerliste-<branch>.md` (folder `$FS_GATES_FEHLERLISTE_DIR`, default the scratch folder, in CI `$RUNNER_TEMP` plus the job summary; never in the repo tree). Each item: step, severity, rc, the exact `nachpruefung:` command(s) and the first error lines. The gate stays HARD: one red step = exit 1; no finding = `0 findings`, exit 0. `gates-komplett` writes the merged list of its three runs and names every finding once with its runs. `gates-komplett --repo-dir DIR --nur-fehlerliste [FILE]` (TÜV principle) runs ONLY the `nachpruefung:` commands, ticks the green items `[x]` (UTC time), keeps the red ones with rc and error lines and exits 1 while anything is open; it runs only commands that pass an ARGUMENT WHITELIST per program and sub-command (`cargo test|clippy|fmt` with a few fixed options and plain crate/test names, `bash {SCRIPTS}/gates-lock.sh|gates-files.sh {ROOT}`, `python3 -I {SCRIPTS}/check_rust_rules.py {ROOT}`; no `=` form, no `-Z`/`--config`/`--manifest-path`/`--target-dir`, no path, no environment assignment; anything unknown is REFUSED, fail closed) as an argument list (no shell), and reads the list only from the list folder as a regular file that the caller owns and nobody else can write: it is opened once with `O_NOFOLLOW` (a symlink is refused) and checked on the open file; clippy needs its whole tail `-D warnings -W clippy::pedantic`, test filters stand only behind `--`. It is a TEILPRÜFUNG, never a gate result: its JSON says `status: teilpruefung`, `red: true`, has no head and no runs, and the last line is `TEILPRÜFUNG: kein Gate-Ergebnis, volle CI nötig`; its exit code is never 0 (1 = something is open, 3 = everything ticked, the full CI is still needed). Tag and ff need the complete green CI at the exact head plus the Opus JA; an Opus delta order carries the list and reviews the conditions of the list and the diff since the last verdict (docs/prompt-template.md). In the CI the reusable `check.yml`, `_windows-check.yml`, `branch-ci.yml` and `lint.yml` run each finding step with `continue-on-error: true` and end with a collecting step (`if: always()`) that writes every `steps.<id>.outcome` into the job summary and turns the job red when one step was red (`scripts/test_ci_sammelschritt.sh`); the caller files generated by `fs-blueprint new` only call these workflows, so they need no change. Self tests: `scripts/test_gates_sammelliste.sh`, `scripts/test_ci_sammelschritt.sh`.


### Local gates: `scripts/gates.sh --like-ci` and the Beleg ([GATE-BELEG])

"Green here, red in CI" came from gaps: no rustup on the machine (the pin `RUSTUP_TOOLCHAIN` has no effect, local rustc 1.100.0 vs CI 1.99.1: lints differ, e.g. `clippy::format_collect` in a new test file was dismissed as "toolchain", `assert-is-empty` in tests), single cargo commands instead of the whole gate (the `rust-rules` file-length rule on a test file never ran), the CI job `lint` (shellcheck SC2164, actionlint, the self tests such as test_affected_crates) and MSRV not being local, and Windows-only code (`cfg(windows)`, "unused self") that a Linux run never sees.

One source: `RUST_VERSION`, `scripts/gates.sh` and `.github/workflows/lint.yml`. `scripts/gates.sh --like-ci <repo>` (flag anywhere; an unknown option is rc 2) runs fmt, clippy (`--all-targets`, tests included), test, rust-rules (all `.rs` incl. tests), lock, files, shellcheck with the file list of lint.yml, actionlint when present, EVERY self-test/compile command of lint.yml (single-line `run:` and `run: |` blocks whose lines are all of the allowed forms; other blocks are listed as not run), MSRV with rustup, and clippy for a Windows target when one is installed.

A local run NEVER says "ja" in its CI-gleich verdict. Every run ends with `gates-beleg:` lines: head (full SHA) and UTC time, tool versions, the clippy flags, `CI-gleich: NEIN - Zweig-CI am genauen Kopf (inkl. windows) ist der Beleg`, `lokal gelaufen: <step=rc ...>` and one `lokal NICHT gelaufen: <what>` line per thing that did not run (foreign toolchain `Toolchain 1.99.1 nicht verfuegbar`, the Windows job incl. `cargo test`, `cfg!(windows)`, unix-only cfg and target dependencies, origin/main missing, a missing tool, a skipped or narrowed test, a multi-line CI step). The exit code stays that of the steps. The worker report MUST carry these lines; the proof for landing is the branch CI at the exact head incl. windows (the report names the run id). A clippy finding in a file the branch changed is never dismissed as "toolchain".

Under a foreign toolchain a clippy counter-probe re-runs clippy with `-A` ONLY for lints whose every location in the first run is in a file unchanged against origin/main (a lint that also occurs in a changed file is never allowed), so test targets hidden behind lib errors are checked (AW18); files with findings are listed, changed ones marked `CHANGED file`. Without origin/main the Beleg says so and lists no files.

Self test: `scripts/test_gates_like_ci.sh` (a fixture and a mutant for each case: AW18, assert-is-empty, Windows, SC2164, 301-line test file, self-test block of lint.yml).

**Identity gate without main (first check, decision 9f).** A new repository has no `main` until its first review. Then `scripts/identity-check` (and the byte-equal copy in the job `identity` of `check.yml`) does NOT skip: only when `git ls-remote --exit-code origin main` says rc 2 (main does not exist on the remote) it checks ALL commits of the branch from its root commit (same rules: author and committer address, TAB fields, count check) and prints `kein main: Erstpruefung` (in CI also `::notice::`). Everything else stays red with rc 2: a fetch or network error (rc 128, timeout 124), a main that exists but was not fetched, a shallow clone (the root is unknown), no root commit, a base other than `origin/main` that does not exist. Threat model, literally: the gate protects against MISTAKES, not against someone who can change the workflow, the toolchain or the scripts. New repositories (`modul-neu`, `programm-neu`, repo creation) create a `main` with a birth commit (LICENSE, README, scaffold only) at once, so the first check is the exception.
