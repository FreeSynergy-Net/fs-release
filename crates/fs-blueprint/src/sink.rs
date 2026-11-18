//! Pattern: Collecting parameter (every check writes its findings here; a rule decides whether a finding is an error, a warning or nothing).

use crate::error::{Coded, Report};
use crate::form::{text_faults, TextFault};
use crate::kit::{KitCatalog, RepoView};
use crate::model::Text2;
use crate::rules::{Mode, Rule, Rules};

pub struct Sink<'a> {
    pub rules: &'a Rules,
    pub catalog: &'a dyn KitCatalog,
    pub view: &'a dyn RepoView,
    pub report: Report,
}

impl<'a> Sink<'a> {
    #[must_use]
    pub fn new(rules: &'a Rules, catalog: &'a dyn KitCatalog, view: &'a dyn RepoView) -> Self {
        Self {
            rules,
            catalog,
            view,
            report: Report::default(),
        }
    }

    /// A finding of the structure: always an error.
    pub fn err(&mut self, c: Coded) {
        self.report.errors.push(c);
    }

    /// A finding of an adjustable rule: error, warning or nothing, as the rule's mode says.
    pub fn rule(&mut self, rule: Rule, c: Coded) {
        match self.rules.mode(rule) {
            Mode::Block => self.report.errors.push(c),
            Mode::Warn => self.report.warnings.push(c),
            Mode::Off => {}
        }
    }

    pub fn warn(&mut self, c: Coded) {
        self.report.warnings.push(c);
    }

    /// A pair of texts: both languages, the same variables.
    pub fn text(&mut self, key: &str, t: &Text2) {
        for fault in text_faults(t) {
            self.err(match fault {
                TextFault::Missing(lang) => Coded::new("text-pair")
                    .with("key", key)
                    .with("missing", lang),
                TextFault::Vars => Coded::new("text-vars").with("key", key),
            });
        }
    }

    /// `duplicate{table, name}` for a name that came before.
    pub fn unique<'n>(&mut self, table: &str, names: impl Iterator<Item = &'n str>) {
        let mut seen = std::collections::BTreeSet::new();
        for n in names {
            if !seen.insert(n) {
                self.err(Coded::new("duplicate").with("table", table).with("name", n));
            }
        }
    }
}
