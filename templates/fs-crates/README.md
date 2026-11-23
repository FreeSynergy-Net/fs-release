# fs-crates: registry index `freesynergy`

Git index registry of FreeSynergy (cargo registry format). Written ONLY by the workflow `.github/workflows/index.yml` of this repo, never by hand.
Details, threat model and the click list: `docs/crate-index.md` in `FreeSynergy-Net/fs-release`.

## How these files get here (once)
1. Create the repo `FreeSynergy-Net/fs-crates` (public, default branch `main`) and do the click list in `docs/crate-index.md` (section "Click list for fs-crates").
2. Copy this folder (`templates/fs-crates/` of fs-release) into the repo root: `.github/workflows/index.yml`, `.github/CODEOWNERS`, `allowed-sources.txt`, `config.json`, this README.
3. Replace every `FS_RELEASE_PIN` in `index.yml` BY HAND (all `uses:` lines of fs-release actions; `pin-refs.sh` refuses the placeholder with rc 3) by the commit of fs-release that contains `crates/fs-crate-index` (full 40 hex).
4. Commit `config.json` as the first commit (`cargo index init` writes the same file); push to `main`.
5. Press `Run workflow` once (Actions -> index) to see it green with an empty plan.

## Rules
- The workflow runs every 15 min and by `Run workflow`. It is idempotent: what is in the index is skipped.
- `allowed-sources.txt` and `.github/` are in CODEOWNERS: a change needs a review (CODEOWNERS marks the reviewers; the team `registry-owners` needs Write). A ruleset with "require a pull request" on all of `main` is NOT set: the workflow pushes entries directly and every entry would turn red. Whether a ruleset can require a review only for these paths is [U, not checked]; if not, the review of these files is a rule of the owners (click list, point 10). That file is part of the trust set.
- A bad tag (lightweight, off `main`, semver red, moved) makes every run red and blocks all LATER tags of ALL source repos (the scan stops at the first one). Way out: delete the tag in the source repo, or raise the first version of that line in `allowed-sources.txt` above it by a reviewed pull request.
- The CONTENT of the `.crate` files of a run is decided by the code that runs in job `package` (build scripts of the source crates); job `enter` only runs the tool of job `tool`, checked by its sha256.
- GitHub switches scheduled workflows off after 60 days without activity in the repo (public repos). Every entry is a commit and counts as activity; with no release for 60 days the workflow may be off: Actions -> index -> `Enable workflow`, then `Run workflow`. A release that waits for its index line reports exactly this as a blocker (`fs-crate-index wait`).
