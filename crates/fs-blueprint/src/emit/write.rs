//! Pattern: Adapter (the store the output goes to) + Template method (check everything first, write only when nothing is refused).
//!
//! Only `Generated` is written, and not over a hand-written or hand-changed file. A `Seed` is created when it is missing and never touched again.
//! An `Exception` is never written. Paths have `/` and are turned into native paths only here.

use super::artifact::{Artifact, Class};
use super::header;
use crate::error::Coded;
use std::path::PathBuf;

/// Where the output goes: a directory on disk, or a map in a test.
pub trait Store {
    fn read(&self, path: &str) -> Option<String>;
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
    fn read(&self, path: &str) -> Option<String> {
        std::fs::read_to_string(self.native(path)).ok()
    }

    fn put(&mut self, path: &str, content: &str) -> Result<(), String> {
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

/// Writes the output. Nothing is written when any file is refused.
///
/// # Errors
/// `generated-changed{path}` for a refused file, `write-failed{path}` when the store fails.
pub fn write(artifacts: &[Artifact], store: &mut dyn Store) -> Result<Outcome, Vec<Coded>> {
    let refused: Vec<Coded> = artifacts
        .iter()
        .filter(|a| a.class == Class::Generated)
        .filter(|a| {
            store
                .read(&a.path)
                .is_some_and(|old| hand_changed(&old, &a.content))
        })
        .map(|a| Coded::new("generated-changed").with("path", a.path.as_str()))
        .collect();
    if !refused.is_empty() {
        return Err(refused);
    }
    let mut out = Outcome::default();
    let mut failed = Vec::new();
    for a in artifacts {
        let put = |store: &mut dyn Store| store.put(&a.path, &a.content);
        match a.class {
            Class::Exception => out.skipped.push(a.path.clone()),
            Class::Seed if store.read(&a.path).is_some() => out.kept.push(a.path.clone()),
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
