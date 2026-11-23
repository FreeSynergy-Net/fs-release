//! Pattern: Specification (an `allow` entry says which hits are meant to stay, and why).
//!
//! File: a JSON array of `{"pattern": "...", "reason": "...", "scope": "docs/", "repo": "fs-release"}`.
//! `pattern`: text that must occur in the line (any case; empty = every line); `scope`: path prefix (empty or `*` = all paths);
//! `repo`: optional, the entry then counts for that repo only; `reason` is required. An entry that matches nothing is stale = red.

use std::path::Path;

use serde::Deserialize;

#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Entry {
    pub pattern: String,
    pub reason: String,
    #[serde(default)]
    pub scope: String,
    #[serde(default)]
    pub repo: Option<String>,
}

impl Entry {
    fn is_unscoped(&self) -> bool {
        self.scope.is_empty() || self.scope == "*"
    }

    fn too_broad(&self, owner: &str) -> bool {
        let (p, o) = (
            self.pattern.to_ascii_lowercase(),
            owner.to_ascii_lowercase(),
        );
        p.len() < 4 || p.contains(&o) || o.contains(&p)
    }

    fn applies_to(&self, repo: &str) -> bool {
        self.repo.as_deref().is_none_or(|r| r == repo)
    }

    fn covers(&self, repo: &str, path: &str, text: &str) -> bool {
        self.applies_to(repo)
            && (self.scope.is_empty() || self.scope == "*" || path.starts_with(&self.scope))
            && text
                .to_ascii_lowercase()
                .contains(&self.pattern.to_ascii_lowercase())
    }

    #[must_use]
    pub fn label(&self) -> String {
        format!(
            "pattern {:?} scope {:?} repo {:?} ({})",
            self.pattern, self.scope, self.repo, self.reason
        )
    }
}

/// The entries and which of them matched something.
#[derive(Debug, Default)]
pub struct Allow {
    entries: Vec<Entry>,
    used: Vec<bool>,
}

impl Allow {
    /// No `allow` file given: nothing is allowed.
    #[must_use]
    pub fn none() -> Self {
        Self::default()
    }

    /// # Errors
    /// A missing or unreadable file, invalid JSON, an entry without reason, or an unscoped entry that is too broad (empty or under 4 characters, or the owner name or a part of it: it would cover every hit).
    pub fn load(path: &Path, owner: &str) -> Result<Self, String> {
        let raw = std::fs::read_to_string(path)
            .map_err(|e| format!("allow file {}: {e}", path.display()))?;
        let entries: Vec<Entry> = serde_json::from_str(&raw)
            .map_err(|e| format!("allow file {}: {e}", path.display()))?;
        for e in &entries {
            if e.reason.trim().is_empty() {
                return Err(format!("allow entry without reason: {}", e.label()));
            }
            if e.is_unscoped() && e.too_broad(owner) {
                return Err(format!(
                    "allow entry without scope that would allow (nearly) everything, give it a scope or a narrower pattern: {}",
                    e.label()
                ));
            }
        }
        let used = vec![false; entries.len()];
        Ok(Self { entries, used })
    }

    /// True (and the entry is marked as used) when an entry allows this line.
    pub fn allows(&mut self, repo: &str, path: &str, text: &str) -> bool {
        let mut hit = false;
        for (i, e) in self.entries.iter().enumerate() {
            if e.covers(repo, path, text) {
                self.used[i] = true;
                hit = true;
            }
        }
        hit
    }

    /// Entries for one of the checked repos that matched nothing.
    #[must_use]
    pub fn stale(&self, repos: &[String], whole_run: bool) -> Vec<String> {
        self.entries
            .iter()
            .zip(&self.used)
            .filter(|(e, used)| {
                !**used
                    && (if e.repo.is_some() {
                        repos.iter().any(|r| e.applies_to(r))
                    } else {
                        whole_run
                    })
            })
            .map(|(e, _)| e.label())
            .collect()
    }
}
