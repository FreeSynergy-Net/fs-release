//! Pattern: Strategy (one emitter that writes the files of the empty program from a table: CI callers, package).
//!
//! A program is data (ADR 0086, rule `code-in-program`): no Rust source and no crate manifest is generated (no entry point, no `lib.rs`, no start test, no
//! workspace or crate `Cargo.toml`); what `new` writes is a data package plus the check CI.

use super::artifact::{Artifact, Class, Emitter};
use super::slot::{fill, Values};
use crate::error::Coded;
use crate::kit::KitCatalog;
use crate::model::Blueprint;

/// One file of the output: its path (with slots), its template and its class.
struct Entry {
    path: &'static str,
    template: &'static str,
    class: Class,
}

macro_rules! tmpl {
    ($name:literal) => {
        include_str!(concat!("../../templates/program/", $name))
    };
}

const FILES: &[Entry] = &[
    Entry {
        path: ".gitattributes",
        template: tmpl!("gitattributes.tmpl"),
        class: Class::Generated,
    },
    Entry {
        path: ".github/workflows/check.yml",
        template: tmpl!("check.yml.tmpl"),
        class: Class::Generated,
    },
    Entry {
        path: ".github/workflows/windows.yml",
        template: tmpl!("windows.yml.tmpl"),
        class: Class::Generated,
    },
    Entry {
        path: "packaging/net.freesynergy.{{pascal}}.desktop",
        template: tmpl!("desktop.tmpl"),
        class: Class::Generated,
    },
];

/// The empty program: every file of the table, filled from the blueprint.
pub struct Program;

impl Emitter for Program {
    fn emit(&self, bp: &Blueprint, _: &dyn KitCatalog) -> Result<Vec<Artifact>, Coded> {
        let values = Values::of(bp)?;
        FILES
            .iter()
            .map(|e| {
                let path = fill(e.path, &values, e.path)?;
                let body = fill(e.template, &values, &path)?;
                Ok(Artifact::new(path, e.class, body))
            })
            .collect()
    }
}

/// The `.desktop` entry of the program, the ONE source of both the file in `packaging/` and the file of the RPM (the RPM has no head line): `Exec=fs-starter <name>` starts the program (ADR 0094).
///
/// # Errors
/// The errors of `Values::of` and `fill`.
pub fn desktop_entry(bp: &Blueprint) -> Result<String, Coded> {
    fill(tmpl!("desktop.tmpl"), &Values::of(bp)?, "desktop")
}

#[must_use]
pub fn all() -> Vec<Box<dyn Emitter>> {
    vec![
        Box::new(Program),
        Box::new(super::frontend::Frontend),
        Box::new(super::layout::Layout),
        Box::new(super::settings::Settings),
        Box::new(super::help::HelpTexts),
    ]
}
