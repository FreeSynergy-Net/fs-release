//! Pattern: Template method + Strategy (`generate` runs every emitter, stamps the heads, sorts by path and sets the class from the blueprint; one emitter per kind of output).
//!
//! The same blueprint gives byte-identical output: no time stamp, no machine path, no map with a random order, `/` in every path on every platform.

pub mod check;
pub mod entry;
pub mod header;
pub mod slot;
pub mod write;

use crate::check_exc::covers;
use crate::error::Coded;
use crate::form::is_repo_path;
use crate::kit::KitCatalog;
use crate::model::Blueprint;
use std::collections::BTreeMap;

/// Who owns a file.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Class {
    /// Written by the generator on every run; a head line says so; changing it by hand is drift.
    Generated,
    /// Created once by the generator, then the program's: never overwritten.
    Seed,
    /// Named by a `[[exception]]` of the blueprint: hand-written, never written, never compared.
    Exception,
}

/// One file of the output: a repo path (relative, with `/`), its class and its full content (head line included).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Artifact {
    pub path: String,
    pub class: Class,
    pub content: String,
}

impl Artifact {
    #[must_use]
    pub fn new(path: impl Into<String>, class: Class, content: impl Into<String>) -> Self {
        Self {
            path: path.into(),
            class,
            content: content.into(),
        }
    }
}

/// A repo path from its parts, always with `/` (also on Windows).
#[must_use]
pub fn join(parts: &[&str]) -> String {
    parts.join("/")
}

/// One kind of output. It returns bodies without head line; `generate` stamps them. The catalog is for emitters that need building-block facts (tags, kits).
pub trait Emitter {
    /// # Errors
    /// A code when a template cannot be filled.
    fn emit(&self, bp: &Blueprint, catalog: &dyn KitCatalog) -> Result<Vec<Artifact>, Coded>;
}

/// The emitters of the generator, in a fixed order.
#[must_use]
pub fn emitters() -> Vec<Box<dyn Emitter>> {
    entry::all()
}

/// Generates the whole output of a blueprint, sorted by path.
///
/// # Errors
/// The first code of a failing emitter, `path-invalid`, `artifact-duplicate{path}`, `header-unsupported{path}` or `exception-undeclared{path}`.
pub fn generate(bp: &Blueprint, catalog: &dyn KitCatalog) -> Result<Vec<Artifact>, Coded> {
    let list = emitters();
    let refs: Vec<&dyn Emitter> = list.iter().map(AsRef::as_ref).collect();
    generate_with(bp, catalog, &refs)
}

/// [`generate`] with the emitters given (tests put their own in).
///
/// # Errors
/// As [`generate`].
pub fn generate_with(
    bp: &Blueprint,
    catalog: &dyn KitCatalog,
    emitters: &[&dyn Emitter],
) -> Result<Vec<Artifact>, Coded> {
    let hash = header::blueprint_hash(bp)?;
    let mut sorted: BTreeMap<String, Artifact> = BTreeMap::new();
    for e in emitters {
        for a in e.emit(bp, catalog)? {
            if !is_repo_path(&a.path) {
                return Err(Coded::new("path-invalid").with("path", a.path));
            }
            let covered = bp
                .exception
                .iter()
                .any(|x| x.paths.iter().any(|p| covers(p, &a.path)));
            let class = match (covered, a.class) {
                (true, _) => Class::Exception,
                (false, Class::Exception) => {
                    return Err(Coded::new("exception-undeclared").with("path", a.path));
                }
                (false, c) => c,
            };
            let content = header::stamp(&a.path, class, &hash, &a.content)?;
            let art = Artifact::new(a.path.clone(), class, content);
            if sorted.insert(a.path.clone(), art).is_some() {
                return Err(Coded::new("artifact-duplicate").with("path", a.path));
            }
        }
    }
    Ok(sorted.into_values().collect())
}
