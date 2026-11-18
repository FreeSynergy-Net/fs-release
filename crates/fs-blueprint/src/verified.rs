//! Pattern: Value object by construction (a blueprint that passed the check: only `verify` makes one, and the generator takes nothing else).

use crate::check::check;
use crate::error::Report;
use crate::kit::{KitCatalog, RepoView};
use crate::model::Blueprint;
use crate::rules::Rules;

/// A blueprint that passed the check with no error. The field is private: the only way to get one is [`verify`].
#[derive(Debug, Clone, Copy)]
pub struct Verified<'a> {
    bp: &'a Blueprint,
}

impl<'a> Verified<'a> {
    /// The checked blueprint.
    #[must_use]
    pub fn blueprint(&self) -> &'a Blueprint {
        self.bp
    }
}

/// Checks a blueprint and gives it back as [`Verified`] when there is no error.
///
/// # Errors
/// The whole report (errors and warnings) when the check found an error.
pub fn verify<'a>(
    bp: &'a Blueprint,
    catalog: &dyn KitCatalog,
    view: &dyn RepoView,
    rules: &Rules,
) -> Result<Verified<'a>, Report> {
    let report = check(bp, catalog, view, rules);
    if report.is_ok() {
        Ok(Verified { bp })
    } else {
        Err(report)
    }
}
