//! Pattern: Strategy (one emitter that writes the files of the empty program from a table: workspace, crate manifests, CI callers, package).
//!
//! A program is data (ADR 0086, rule `code-in-program`): no Rust source is generated any more (no entry point, no `lib.rs`, no start test); the manifests of the
//! crates stay until the data package replaces them (open follow-up of E1-2).

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
        path: "Cargo.toml",
        template: tmpl!("workspace.toml.tmpl"),
        class: Class::Generated,
    },
    Entry {
        path: "crates/fs-{{program}}-bin/Cargo.toml",
        template: tmpl!("bin.toml.tmpl"),
        class: Class::Generated,
    },
    Entry {
        path: "crates/fs-{{program}}-client/Cargo.toml",
        template: tmpl!("client.toml.tmpl"),
        class: Class::Generated,
    },
    Entry {
        path: "crates/fs-{{program}}-core/Cargo.toml",
        template: tmpl!("core.toml.tmpl"),
        class: Class::Seed,
    },
    Entry {
        path: "crates/fs-{{program}}-server/Cargo.toml",
        template: tmpl!("server.toml.tmpl"),
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

#[must_use]
pub fn all() -> Vec<Box<dyn Emitter>> {
    vec![Box::new(Program)]
}
