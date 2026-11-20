//! Pattern: Adapter (the store the output goes to) + Template method (check everything first, write only when nothing is refused).
//!
//! Only `Generated` is written, and not over a hand-written or hand-changed file. A `Seed` is created when it is missing and never touched again.
//! An `Exception` is never written. Paths have `/` and are turned into native paths only here.

use super::append_only;
use super::artifact::{Artifact, Class};
use super::header;
use crate::error::Coded;
use crate::form::is_repo_path;

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
    /// Puts one file.
    ///
    /// # Errors
    /// The reason as text.
    fn put(&mut self, path: &str, content: &str) -> Result<(), String>;
    /// Puts all files or, when the store can, none: a store on disk writes every file next to its place first and moves them into place afterwards, so a failure while
    /// writing leaves nothing new behind. The default puts one after the other (a store in memory cannot fail half way).
    ///
    /// # Errors
    /// The path of the file that failed.
    fn put_all(&mut self, files: &[(&str, &str)]) -> Result<(), String> {
        files
            .iter()
            .try_for_each(|(p, c)| self.put(p, c).map_err(|_| (*p).to_string()))
    }
}

pub use super::dirstore::DirStore;

/// What `write` did, by path.
#[derive(Debug, Default, PartialEq, Eq)]
pub struct Outcome {
    pub written: Vec<String>,
    /// Seeds that were there already and were left alone.
    pub kept: Vec<String>,
    /// Exceptions: not written.
    pub skipped: Vec<String>,
}

/// An existing file at the path of a generated one is refused unless it is generated and untouched: the hash of its body is the one its head line names
/// (the blueprint behind it may be an older one, then it is replaced). A hand-written file, a changed body or a head without hash is refused.
fn hand_changed(old: &str) -> bool {
    !header::untouched(old)
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
        Found::Text(old) if hand_changed(&old) => {
            Some(Coded::new("generated-changed").with("path", path()))
        }
        // the contract text is append-only against the one that is there
        Found::Text(old) if append_only::is_contract(&a.path) => append_only::broken(&old, &a.content)
            .map(|what| {
                Coded::new("contract-append-only")
                    .with("path", path())
                    .with("what", what)
            }),
        Found::Text(_) => None,
    }
}

/// Writes the output. Nothing is written when any file is refused.
///
/// # Errors
/// `path-invalid`, `path-symlink`, `file-unreadable` or `generated-changed` (all `{path}`) for a refused file; `write-failed{path}` when the store fails (one error: nothing new was written, for a store on disk).
pub fn write(artifacts: &[Artifact], store: &mut dyn Store) -> Result<Outcome, Vec<Coded>> {
    let refused: Vec<Coded> = artifacts.iter().filter_map(|a| refusal(a, store)).collect();
    if !refused.is_empty() {
        return Err(refused);
    }
    let mut out = Outcome::default();
    let mut files: Vec<(&str, &str)> = Vec::new();
    for a in artifacts {
        match a.class {
            Class::Exception => out.skipped.push(a.path.clone()),
            Class::Seed if store.read(&a.path) != Found::Missing => out.kept.push(a.path.clone()),
            Class::Generated | Class::Seed => files.push((a.path.as_str(), a.content.as_str())),
        }
    }
    match store.put_all(&files) {
        Ok(()) => {
            out.written = files.iter().map(|(p, _)| (*p).to_string()).collect();
            Ok(out)
        }
        Err(path) => Err(vec![Coded::new("write-failed").with("path", path)]),
    }
}
