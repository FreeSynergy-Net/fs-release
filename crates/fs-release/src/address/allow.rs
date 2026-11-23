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
    /// A missing or unreadable file, invalid JSON, or an entry without reason or without anything to match.
    pub fn load(path: &Path) -> Result<Self, String> {
        let raw = std::fs::read_to_string(path)
            .map_err(|e| format!("allow file {}: {e}", path.display()))?;
        let entries: Vec<Entry> = serde_json::from_str(&raw)
            .map_err(|e| format!("allow file {}: {e}", path.display()))?;
        for e in &entries {
            if e.reason.trim().is_empty() {
                return Err(format!("allow entry without reason: {}", e.label()));
            }
            if e.pattern.is_empty() && (e.scope.is_empty() || e.scope == "*") {
                return Err(format!(
                    "allow entry that would allow everything: {}",
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
    pub fn stale(&self, repos: &[String]) -> Vec<String> {
        self.entries
            .iter()
            .zip(&self.used)
            .filter(|(e, used)| !**used && repos.iter().any(|r| e.applies_to(r)))
            .map(|(e, _)| e.label())
            .collect()
    }
}
