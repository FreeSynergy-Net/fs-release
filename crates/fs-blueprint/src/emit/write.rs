//! Pattern: Adapter (the store the output goes to) + Template method (check everything first, write only when nothing is refused).
//!
//! Only `Generated` is written, and not over a hand-written or hand-changed file. A `Seed` is created when it is missing and never touched again.
//! An `Exception` is never written. Paths have `/` and are turned into native paths only here.

use super::artifact::{Artifact, Class};
use super::header;
use crate::error::Coded;
use crate::form::is_repo_path;
use std::path::PathBuf;

/// What a store knows about a path: not there, a text, or there but not readable as text (not UTF-8, a folder, no permission).
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Found {
    Missing,
    Text(String),
    Unreadable,
}

/// Where the output goes: a directory on disk, or a map in a test.
pub trait Store {
    fn read(&self, path: &str) -> Found;
    /// False when a part of the path leads out of the store (a link). Checked before anything is written.
    fn path_ok(&self, path: &str) -> bool {
        let _ = path;
        true
    }
    /// # Errors
    /// The reason as text; `write` turns it into `write-failed{path}`.
    fn put(&mut self, path: &str, content: &str) -> Result<(), String>;
}

/// A directory on disk; repo paths with `/` are put below it.
pub struct DirStore {
    root: PathBuf,
}

impl DirStore {
    #[must_use]
    pub fn new(root: impl Into<PathBuf>) -> Self {
        Self { root: root.into() }
    }

    fn native(&self, path: &str) -> PathBuf {
        path.split('/').fold(self.root.clone(), |p, c| p.join(c))
    }
}

impl Store for DirStore {
    fn read(&self, path: &str) -> Found {
        match std::fs::read(self.native(path)) {
            Ok(bytes) => String::from_utf8(bytes).map_or(Found::Unreadable, Found::Text),
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => Found::Missing,
            Err(_) => Found::Unreadable,
        }
    }

    /// No part of the path below the root may be a symbolic link (not the file, not a folder on the way).
    fn path_ok(&self, path: &str) -> bool {
        let mut at = self.root.clone();
        path.split('/').all(|c| {
            at.push(c);
            std::fs::symlink_metadata(&at).map_or(true, |m| !m.file_type().is_symlink())
        })
    }

    fn put(&mut self, path: &str, content: &str) -> Result<(), String> {
        if !self.path_ok(path) {
            return Err("link in path".to_string());
        }
        let file = self.native(path);
        if let Some(dir) = file.parent() {
            std::fs::create_dir_all(dir).map_err(|e| e.to_string())?;
        }
        std::fs::write(file, content).map_err(|e| e.to_string())
    }
}

/// What `write` did, by path.
#[derive(Debug, Default, PartialEq, Eq)]
pub struct Outcome {
    pub written: Vec<String>,
    /// Seeds that were there already and were left alone.
    pub kept: Vec<String>,
    /// Exceptions: not written.
    pub skipped: Vec<String>,
}

/// An existing file at the path of a generated one: refused when it is hand-written (no head) or the same head (same tool, same blueprint) with other content.
fn hand_changed(old: &str, new: &str) -> bool {
    header::class_of(old) != Some(Class::Generated)
        || (header::first_line(old) == header::first_line(new) && old != new)
}

/// Why a file is refused before anything is written: a path that leaves the repo, a link in the path, a generated file that is hand-written, changed or
/// not readable (a seed that is there is never looked into).
fn refusal(a: &Artifact, store: &dyn Store) -> Option<Coded> {
    let path = || a.path.as_str();
    if !is_repo_path(&a.path) {
        return Some(Coded::new("path-invalid").with("path", path()));
    }
    if !store.path_ok(&a.path) {
        return Some(Coded::new("path-symlink").with("path", path()));
    }
    if a.class != Class::Generated {
        return None;
    }
    match store.read(&a.path) {
        Found::Missing => None,
        Found::Unreadable => Some(Coded::new("file-unreadable").with("path", path())),
        Found::Text(old) => hand_changed(&old, &a.content)
            .then(|| Coded::new("generated-changed").with("path", path())),
    }
}

/// Writes the output. Nothing is written when any file is refused.
///
/// # Errors
/// `path-invalid`, `path-symlink`, `file-unreadable` or `generated-changed` (all `{path}`) for a refused file; `write-failed{path}` when the store fails.
pub fn write(artifacts: &[Artifact], store: &mut dyn Store) -> Result<Outcome, Vec<Coded>> {
    let refused: Vec<Coded> = artifacts.iter().filter_map(|a| refusal(a, store)).collect();
    if !refused.is_empty() {
        return Err(refused);
    }
    let mut out = Outcome::default();
    let mut failed = Vec::new();
    for a in artifacts {
        let put = |store: &mut dyn Store| store.put(&a.path, &a.content);
        match a.class {
            Class::Exception => out.skipped.push(a.path.clone()),
            Class::Seed if store.read(&a.path) != Found::Missing => out.kept.push(a.path.clone()),
            Class::Generated | Class::Seed => match put(store) {
                Ok(()) => out.written.push(a.path.clone()),
                Err(_) => failed.push(Coded::new("write-failed").with("path", a.path.as_str())),
            },
        }
    }
    if failed.is_empty() {
        Ok(out)
    } else {
        Err(failed)
    }
}
