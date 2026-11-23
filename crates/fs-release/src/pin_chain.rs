//! Pattern: Specification (one invariant, derived from git, no marker file kept by hand).
//!
//! Chain gate. fs-release pins itself: `release.yml` says `uses: .../_release-publish.yml@Z`, that file says `uses: .../actions/publish-release@Y`.
//! A commit cannot contain its own hash, so every self-pin lags one round behind and the caller pin of a round shows OLD code (two rounds deep
//! for the chain release.yml -> _release-publish.yml -> publish-release). The invariant that closes this, for ALL chains:
//!
//! > the code reached through the pins from the start commit S equals the code at S, except for the self-pins (the 40-hex ref of every
//! > `<owner>/fs-release/<path>@<sha>` is normalised away).
//!
//! (S is the start head.)
//!
//! Roots are every workflow and every action file at S. A pin to a workflow reaches that file, a pin to an action reaches the directory of
//! the action; every reached file is compared with S and followed with ITS pins, so each level is checked on its own.
//! `fs-release check-pin-chain [--repo <dir>] [<start>]`: rc 0 green, 1 red, 2 bad input; `--melden` only reports (lint), rc 0 on red.
//! THREAT MODEL: protects against OVERSIGHT, not against someone who can change workflow, toolchain or scripts.

use std::collections::BTreeSet;
use std::path::Path;
use std::process::Command;

use crate::pin_chain_text::{foreign_uses, is_sha, normalize, uses_of, Use};

/// `git -C <repo> <args>` stdout when it succeeds.
fn git(repo: &Path, args: &[&str]) -> Option<String> {
    let o = Command::new("git")
        .arg("-C")
        .arg(repo)
        .args(args)
        .output()
        .ok()?;
    o.status
        .success()
        .then(|| String::from_utf8_lossy(&o.stdout).into_owned())
}

fn files_at(repo: &Path, commit: &str, path: &str) -> Vec<String> {
    git(repo, &["ls-tree", "-r", "--name-only", commit, "--", path])
        .map_or_else(Vec::new, |s| s.lines().map(str::to_string).collect())
}

fn is_yaml(f: &str) -> bool {
    Path::new(f)
        .extension()
        .is_some_and(|e| e.eq_ignore_ascii_case("yml") || e.eq_ignore_ascii_case("yaml"))
}

fn show(repo: &Path, commit: &str, file: &str) -> Option<String> {
    git(repo, &["show", &format!("{commit}:{file}")])
}

fn short(s: &str) -> &str {
    &s[..s.len().min(8)]
}

struct Walk<'a> {
    repo: &'a Path,
    start: String,
    seen: BTreeSet<(String, String)>,
    red: Vec<String>,
}

impl Walk<'_> {
    /// Follow every pin of `text` (a file as it stands at `at`, reached from `from`).
    fn follow(&mut self, from: &str, text: &str) {
        for t in foreign_uses(text) {
            self.red.push(format!(
                "{from}: `uses: {t}` names an owner of fs-release that is not spelled exactly FreeSynergy-Net or FreeSynergy (unknown owner spelling; the chain would not follow it)"
            ));
        }
        for u in uses_of(text) {
            self.reach(from, &u);
        }
    }

    /// The code under `u.path` at `u.pin` against the same path at the start, then the pins found there.
    fn reach(&mut self, from: &str, u: &Use) {
        if !is_sha(&u.pin) {
            self.red.push(format!(
                "{from}: `{}@{}` is not a full commit SHA",
                u.path, u.pin
            ));
            return;
        }
        if !self.seen.insert((u.path.clone(), u.pin.clone())) {
            return;
        }
        if git(
            self.repo,
            &["cat-file", "-e", &format!("{}^{{commit}}", u.pin)],
        )
        .is_none()
        {
            self.red.push(format!(
                "{from}: pin {} of {} is no commit of this repository",
                short(&u.pin),
                u.path
            ));
            return;
        }
        let at_pin = files_at(self.repo, &u.pin, &u.path);
        let at_start = files_at(self.repo, &self.start, &u.path);
        if at_pin.is_empty() {
            self.red.push(format!(
                "{from}: {} does not exist at pin {}",
                u.path,
                short(&u.pin)
            ));
            return;
        }
        let names: BTreeSet<&String> = at_pin.iter().chain(at_start.iter()).collect();
        for f in names {
            let old = show(self.repo, &u.pin, f).map(|t| normalize(&t));
            let new = show(self.repo, &self.start, f).map(|t| normalize(&t));
            if old != new {
                self.red.push(format!(
                    "{from}: {f} at pin {} differs from the start head {}",
                    short(&u.pin),
                    short(&self.start)
                ));
            }
        }
        for f in at_pin.iter().filter(|f| is_yaml(f)) {
            if let Some(t) = show(self.repo, &u.pin, f) {
                self.follow(&format!("{f}@{}", short(&u.pin)), &t);
            }
        }
    }
}

/// Run the gate over `repo` from `start`. `Ok(findings)` (empty = green) or `Err` for bad input (rc 2).
///
/// # Errors
/// The start does not resolve, a shallow clone cannot be completed, or no internal pin exists at the start.
pub fn check(repo: &Path, start: &str) -> Result<Vec<String>, String> {
    if git(repo, &["rev-parse", "--is-shallow-repository"]).is_some_and(|s| s.trim() == "true") {
        let ok = Command::new("timeout")
            .args(["120", "git", "-C"])
            .arg(repo)
            .args(["fetch", "--quiet", "--unshallow"])
            .status()
            .is_ok_and(|s| s.success());
        if !ok {
            return Err("cannot unshallow the clone".into());
        }
    }
    let sha = git(
        repo,
        &["rev-parse", "--verify", &format!("{start}^{{commit}}")],
    )
    .map(|s| s.trim().to_string())
    .ok_or_else(|| format!("start '{start}' is no commit"))?;
    let mut w = Walk {
        repo,
        start: sha.clone(),
        seen: BTreeSet::new(),
        red: Vec::new(),
    };
    let mut roots = 0;
    for f in files_at(repo, &sha, ".github/workflows")
        .into_iter()
        .chain(files_at(repo, &sha, "actions"))
        .filter(|f| is_yaml(f))
    {
        if let Some(t) = show(repo, &sha, &f) {
            roots += uses_of(&t).len();
            w.follow(&format!("{f}@{}", short(&sha)), &t);
        }
    }
    if roots == 0 {
        return Err("no internal `uses: .../fs-release/...@<sha>` at the start".into());
    }
    Ok(w.red)
}

/// Arguments of `check-pin-chain`.
#[derive(clap::Args, Clone)]
pub struct PinChainArgs {
    /// The checkout.
    #[arg(long, default_value = ".")]
    repo: std::path::PathBuf,
    /// Only report (lint): rc 0 also when red; without it the check is hard (before the re-pin wave and the tag).
    #[arg(long)]
    melden: bool,
    /// The start head.
    #[arg(default_value = "HEAD")]
    start: String,
}

/// The command: prints one line per finding, rc as documented above.
#[must_use]
pub fn run(a: &PinChainArgs) -> i32 {
    let (start, report_only) = (a.start.as_str(), a.melden);
    match check(&a.repo, start) {
        Err(e) => {
            eprintln!("pin-chain: {e}");
            2
        }
        Ok(red) if red.is_empty() => {
            println!(
                "pin-chain green: the code reached through the pins equals the code at {start}"
            );
            0
        }
        Ok(red) => {
            for r in &red {
                println!("pin-chain RED: {r}");
            }
            println!(
                "pin-chain: fix = `bash scripts/pin-refs.sh <40-char sha of the start head>`, commit, run again (at most 2 rounds: release.yml -> _release-publish.yml -> publish-release)"
            );
            i32::from(!report_only)
        }
    }
}
