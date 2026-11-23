//! Pattern: Adapter (git grep on a ref in, typed hits out; every failure is an `Err`, never "0 hits").

use std::path::Path;
use std::process::Command;

use super::class::Class;
use super::mention::{mentions, Kind, Mention};

/// One line with at least one owner mention.
#[derive(Debug, Clone)]
pub struct Hit {
    pub repo: String,
    pub path: String,
    pub line: u32,
    pub text: String,
    pub class: Class,
    pub mentions: Vec<Mention>,
}

impl Hit {
    /// Only `ghcr.io/` mentions: reported on its own.
    #[must_use]
    pub fn ghcr_only(&self) -> bool {
        self.mentions.iter().all(|m| m.kind == Kind::Ghcr)
    }
}

fn git(dir: &Path, args: &[&str]) -> Result<std::process::Output, String> {
    Command::new("git")
        .arg("-C")
        .arg(dir)
        .args(args)
        .output()
        .map_err(|e| format!("git in {}: {e}", dir.display()))
}

/// Name of the repo (last part of the top-level directory).
///
/// # Errors
/// `dir` is not inside a git repo.
pub fn repo_name(dir: &Path) -> Result<String, String> {
    let out = git(dir, &["rev-parse", "--show-toplevel"])?;
    if !out.status.success() {
        return Err(format!("no git repo: {}", dir.display()));
    }
    let top = String::from_utf8_lossy(&out.stdout);
    Ok(top.trim().rsplit('/').next().unwrap_or("").to_string())
}

/// All lines of `git_ref` that mention `owner` (broad scan, see [`mentions`]).
///
/// # Errors
/// Not a repo, `git_ref` does not exist (for example a repo without `main`), or git failed.
pub fn scan(dir: &Path, git_ref: &str, owner: &str) -> Result<Vec<Hit>, String> {
    let repo = repo_name(dir)?;
    let spec = format!("{git_ref}^{{commit}}");
    let ok = git(dir, &["rev-parse", "--verify", "--quiet", &spec])?;
    if !ok.status.success() {
        return Err(format!("{repo}: FEHLT ref {git_ref}"));
    }
    let out = git(
        dir,
        &["grep", "-n", "-I", "-i", "-z", "-F", "-e", owner, git_ref],
    )?;
    match out.status.code() {
        Some(0) => {}
        Some(1) => return Ok(Vec::new()),
        _ => {
            return Err(format!(
                "{repo}: git grep failed: {}",
                String::from_utf8_lossy(&out.stderr).trim()
            ))
        }
    }
    let text = String::from_utf8_lossy(&out.stdout);
    let prefix = format!("{git_ref}:");
    let mut hits = Vec::new();
    for rec in text.lines() {
        let mut parts = rec.splitn(3, '\0');
        let (Some(p), Some(n), Some(t)) = (parts.next(), parts.next(), parts.next()) else {
            continue;
        };
        let path = p.strip_prefix(&prefix).unwrap_or(p).to_string();
        let ms = mentions(t, owner);
        if ms.is_empty() {
            continue;
        }
        hits.push(Hit {
            repo: repo.clone(),
            class: Class::of(&path),
            path,
            line: n.parse().unwrap_or(0),
            text: t.to_string(),
            mentions: ms,
        });
    }
    Ok(hits)
}
