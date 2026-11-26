//! Pattern: Specification (one invariant of the pins, derived from git: no pin of the release path may lie before a fix the path needs).
//!
//! Release-path fix gate, part of `check-pin-chain`. The tag run builds `fs-blueprint` (and runs every action) from the commit a pin names, so a
//! pin that lies BEFORE a fix of the release path ships the old code (13b680a: module RPMs without `Requires`, built from `job.workflow_sha`).
//! The fixes are the data list `scripts/release-path-fixes.txt` at the start head (one full commit SHA per line, `#` starts a comment, extend it
//! when a fix is made that the release path needs). Every internal `uses: .../fs-release/...@<sha>` in `release.yml`, `_release-*.yml` and the
//! actions must name a commit that has EVERY listed fix as an ancestor (`git merge-base --is-ancestor`). No list at the start = nothing required.

use std::path::Path;
use std::process::Command;

use crate::pin_chain::{files_at, show};
use crate::pin_chain_text::{is_sha, uses_of};

/// The list of the fix commits.
pub(crate) const FIXES_FILE: &str = "scripts/release-path-fixes.txt";

fn is_ancestor(repo: &Path, fix: &str, pin: &str) -> bool {
    Command::new("git")
        .arg("-C")
        .arg(repo)
        .args(["merge-base", "--is-ancestor", fix, pin])
        .status()
        .is_ok_and(|s| s.success())
}

fn on_release_path(f: &str) -> bool {
    f == ".github/workflows/release.yml"
        || f.starts_with(".github/workflows/_release-")
        || f.starts_with("actions/")
}

/// Findings (empty = green) for the pins at `start`.
pub(crate) fn findings(repo: &Path, start: &str) -> Vec<String> {
    let Some(list) = show(repo, start, FIXES_FILE) else {
        return Vec::new();
    };
    let all: Vec<&str> = list
        .lines()
        .map(|l| l.split('#').next().unwrap_or("").trim())
        .filter(|l| !l.is_empty())
        .collect();
    let mut red: Vec<String> = all
        .iter()
        .filter(|f| !is_sha(f))
        .map(|f| format!("{FIXES_FILE}: `{f}` is not a full commit SHA"))
        .collect();
    let fixes: Vec<&str> = all.into_iter().filter(|f| is_sha(f)).collect();
    let mut files = files_at(repo, start, ".github/workflows");
    files.extend(files_at(repo, start, "actions"));
    for f in files.iter().filter(|f| on_release_path(f)) {
        let Some(text) = show(repo, start, f) else {
            continue;
        };
        for u in uses_of(&text).into_iter().filter(|u| is_sha(&u.pin)) {
            for fix in fixes.iter().filter(|x| !is_ancestor(repo, x, &u.pin)) {
                red.push(format!(
                    "{f}: `{}@{}` lies before the release-path fix {} ({FIXES_FILE}): the tag run would build and run the code WITHOUT it; fix: bash scripts/pin-refs.sh <sha of a commit that contains the fix>",
                    u.path,
                    &u.pin[..8],
                    &fix[..8]
                ));
            }
        }
    }
    red
}
