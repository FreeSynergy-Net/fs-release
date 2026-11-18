//! Pattern: Value object (a file of the output with its class) + Strategy port (`Emitter`, one per kind of output).

use crate::error::Coded;
use crate::kit::KitCatalog;
use crate::model::Blueprint;

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
