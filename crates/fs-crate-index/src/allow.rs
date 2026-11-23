//! Pattern: none (the allow list of the registry repo and the tags of a source repo).

use std::process::Command;

use anyhow::{ensure, Context, Result};
use semver::Version;

use crate::guard::version_of_ref;

/// One line of the allow list.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Source {
    pub repo: String,
    pub from: Version,
    pub packages: Vec<String>,
}

/// One tag of a source repo as `git ls-remote --tags` shows it.
#[derive(Debug, Clone)]
pub struct TagInfo {
    pub name: String,
    pub version: Version,
    pub tag_object: String,
    /// The commit the tag points to (`^{}` line); `None` = a lightweight tag.
    pub commit: Option<String>,
}

/// Parses the allow list.
///
/// # Errors
/// A line has no repo, no first version or no package, or a name has unexpected characters.
pub fn parse_allow(text: &str) -> Result<Vec<Source>> {
    let mut out = Vec::new();
    for (n, raw) in text.lines().enumerate() {
        let line = raw.split('#').next().unwrap_or("").trim();
        if line.is_empty() {
            continue;
        }
        let mut it = line.split_whitespace();
        let repo = it.next().unwrap_or_default();
        let from = it
            .next()
            .with_context(|| format!("line {}: first version missing", n + 1))?;
        let packages: Vec<String> = it.map(str::to_string).collect();
        let name_ok = |s: &str| {
            !s.is_empty()
                && s.bytes()
                    .all(|b| b.is_ascii_alphanumeric() || b"-_.".contains(&b))
        };
        let repo_ok = repo
            .split_once('/')
            .is_some_and(|(o, r)| name_ok(o) && name_ok(r) && !r.contains('/'));
        ensure!(repo_ok, "line {}: `{repo}` is not owner/repo", n + 1);
        ensure!(!packages.is_empty(), "line {}: no package", n + 1);
        ensure!(
            packages.iter().all(|p| name_ok(p)),
            "line {}: bad package name",
            n + 1
        );
        out.push(Source {
            repo: repo.to_string(),
            from: Version::parse(from).with_context(|| format!("line {}: `{from}`", n + 1))?,
            packages,
        });
    }
    Ok(out)
}

pub(crate) fn url_of(base: &str, repo: &str) -> String {
    format!("{}/{repo}.git", base.trim_end_matches('/'))
}

/// All release tags `vX.Y.Z` of a repo, oldest first (other tags are ignored).
///
/// # Errors
/// `git ls-remote` fails.
pub fn list_tags(url: &str) -> Result<Vec<TagInfo>> {
    let out = Command::new("git")
        .args(["ls-remote", "--tags", url])
        .output()
        .context("cannot start git")?;
    ensure!(
        out.status.success(),
        "git ls-remote {url} failed: {}",
        String::from_utf8_lossy(&out.stderr)
    );
    let text = String::from_utf8_lossy(&out.stdout);
    let mut tags: Vec<TagInfo> = Vec::new();
    for line in text.lines() {
        let Some((sha, r)) = line.split_once('\t') else {
            continue;
        };
        let (r, peeled) = r.strip_suffix("^{}").map_or((r, false), |x| (x, true));
        let Ok(v) = version_of_ref(r) else { continue };
        let Ok(version) = Version::parse(&v) else {
            continue;
        };
        let name = r.trim_start_matches("refs/tags/").to_string();
        if peeled {
            if let Some(t) = tags.iter_mut().find(|t| t.name == name) {
                t.commit = Some(sha.to_string());
            }
        } else {
            tags.push(TagInfo {
                name,
                version,
                tag_object: sha.to_string(),
                commit: None,
            });
        }
    }
    tags.sort_by(|a, b| a.version.cmp(&b.version));
    Ok(tags)
}
