# fs-crates: registry index `freesynergy`

Git index registry of FreeSynergy (cargo registry format). Written ONLY by the workflow `.github/workflows/index.yml` of this repo, never by hand.
Details, threat model and the click list: `docs/crate-index.md` in `FreeSynergy-Net/fs-release`.

## How these files get here (once)
1. Create the repo `FreeSynergy-Net/fs-crates` (public, default branch `main`) and do the click list in `docs/crate-index.md` (section "Click list for fs-crates").
2. Copy this folder (`templates/fs-crates/` of fs-release) into the repo root: `.github/workflows/index.yml`, `.github/CODEOWNERS`, `allowed-sources.txt`, `config.json`, this README.
3. Replace `FS_RELEASE_PIN` in `index.yml` (2 places: the checkout `ref` and the `uses`) by the commit of fs-release that contains `crates/fs-crate-index` (full 40 hex).
4. Commit `config.json` as the first commit (`cargo index init` writes the same file); push to `main`.
5. Press `Run workflow` once (Actions -> index) to see it green with an empty plan.

## Rules
- The workflow runs every 15 min and by `Run workflow`. It is idempotent: what is in the index is skipped.
- `allowed-sources.txt` and `.github/` are in CODEOWNERS: a change needs a review (branch protection: pull request + code owner review). That file is part of the trust set.
- GitHub switches scheduled workflows off after 60 days without activity in the repo (public repos). Every entry is a commit and counts as activity; with no release for 60 days the workflow may be off: Actions -> index -> `Enable workflow`, then `Run workflow`. A release that waits for its index line reports exactly this as a blocker (`fs-crate-index wait`).
