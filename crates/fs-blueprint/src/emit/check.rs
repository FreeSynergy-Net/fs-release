//! Pattern: Template method (is the repo the product of the blueprint? every generated file byte-equal, every seed there, exceptions left alone).

use super::{generate, Class};
use crate::error::{Coded, Report};
use crate::kit::{KitCatalog, RepoView};
use crate::model::Blueprint;

/// Compares the repo with what `generate` gives, in memory. A seed is only looked for, never compared (it belongs to the program); an exception is not looked at.
/// Drift is an error with the path: `generated-missing`, `generated-changed`, `seed-missing`. The structure of the blueprint itself is `crate::check`.
#[must_use]
pub fn check(bp: &Blueprint, catalog: &dyn KitCatalog, view: &dyn RepoView) -> Report {
    let artifacts = match generate(bp, catalog) {
        Ok(a) => a,
        Err(code) => {
            return Report {
                errors: vec![code],
                warnings: Vec::new(),
            }
        }
    };
    let mut report = Report::default();
    for a in &artifacts {
        let found = view.read(&a.path);
        let finding = match (a.class, found) {
            (Class::Seed, None) => Some("seed-missing"),
            (Class::Generated, None) => Some("generated-missing"),
            (Class::Generated, Some(c)) if c != a.content => Some("generated-changed"),
            // an exception is not looked at; a seed that is there is never compared
            _ => None,
        };
        if let Some(code) = finding {
            report
                .errors
                .push(Coded::new(code).with("path", a.path.as_str()));
        }
    }
    report
}
