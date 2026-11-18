//! Pattern: Strategy (the emitters of this step: the entry point `main`, the seed of the own core, the line-ending file).
//!
//! The entry point is purely generated and exchangeable (user decision 45): its template has one slot, the program name, which is a reference into
//! the blueprint. Nothing else of the program may stand in it; the test compares it across many blueprints.

use super::slot::{fill, Values};
use super::{join, Artifact, Class, Emitter};
use crate::error::Coded;
use crate::kit::KitCatalog;
use crate::model::Blueprint;

const MAIN: &str = include_str!("../../templates/main.rs.tmpl");
const CORE: &str = include_str!("../../templates/core_lib.rs.tmpl");
const ATTRIBUTES: &str = include_str!("../../templates/gitattributes.tmpl");

/// `crates/fs-<name>-bin/src/main.rs`: the generated entry point.
pub struct Entry;

impl Emitter for Entry {
    fn emit(&self, bp: &Blueprint, _: &dyn KitCatalog) -> Result<Vec<Artifact>, Coded> {
        let path = join(&[
            "crates",
            &format!("fs-{}-bin", bp.program.name),
            "src",
            "main.rs",
        ]);
        let body = fill(MAIN, &Values::of(bp), &path)?;
        Ok(vec![Artifact::new(path, Class::Generated, body)])
    }
}

/// `crates/fs-<name>-core/src/lib.rs`: the seed of the program's own core.
pub struct CoreSeed;

impl Emitter for CoreSeed {
    fn emit(&self, bp: &Blueprint, _: &dyn KitCatalog) -> Result<Vec<Artifact>, Coded> {
        let path = join(&[
            "crates",
            &format!("fs-{}-core", bp.program.name),
            "src",
            "lib.rs",
        ]);
        let body = fill(CORE, &Values::of(bp), &path)?;
        Ok(vec![Artifact::new(path, Class::Seed, body)])
    }
}

/// `.gitattributes`: fixed line endings, so the output is the same bytes on every platform.
pub struct Attributes;

impl Emitter for Attributes {
    fn emit(&self, bp: &Blueprint, _: &dyn KitCatalog) -> Result<Vec<Artifact>, Coded> {
        let path = ".gitattributes".to_string();
        let body = fill(ATTRIBUTES, &Values::of(bp), &path)?;
        Ok(vec![Artifact::new(path, Class::Generated, body)])
    }
}

#[must_use]
pub fn all() -> Vec<Box<dyn Emitter>> {
    vec![Box::new(Attributes), Box::new(Entry), Box::new(CoreSeed)]
}
