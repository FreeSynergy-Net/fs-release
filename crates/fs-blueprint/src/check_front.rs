//! Pattern: Template method (rule sets and the frontend of the blueprint).

use crate::check_layout::unreadable;
use crate::error::Coded;
use crate::form::{is_covers_entry, is_kebab, is_repo_path};
use crate::kit::Kind;
use crate::model::{Blueprint, Covers};
use crate::rules::Rule;
use crate::sink::Sink;

pub fn rulesets(s: &mut Sink, bp: &Blueprint) {
    s.unique("ruleset", bp.ruleset.iter().map(|r| r.name.as_str()));
    for r in &bp.ruleset {
        let name = r.name.as_str();
        if !bp.types.iter().any(|t| t.name == r.type_name) {
            s.err(Coded::new("type-unknown").with("name", r.type_name.as_str()));
        }
        if r.presets.is_empty() {
            s.err(
                Coded::new("field-empty")
                    .with("table", "ruleset")
                    .with("field", "presets"),
            );
        }
        if !r.presets.contains(&r.default) {
            s.err(Coded::new("ruleset-default-missing").with("ruleset", name));
        }
        let known = source_presets(s, bp, &r.from);
        for p in known
            .iter()
            .flat_map(|k| r.presets.iter().filter(|p| !k.contains(p)))
        {
            s.rule(
                Rule::ExistingOnly,
                Coded::new("preset-unknown")
                    .with("ruleset", name)
                    .with("preset", p.as_str()),
            );
        }
    }
}

/// The presets the source of a rule set offers; `None` (and a finding) when the source is no declared module or bundle of the right role.
fn source_presets(s: &mut Sink, bp: &Blueprint, from: &str) -> Option<Vec<String>> {
    if let Some(m) = bp.module.iter().find(|m| m.name == from) {
        if Kind::parse(&m.kind) == Some(Kind::Component) {
            s.err(
                Coded::new("kind-wrong-role")
                    .with("module", from)
                    .with("expected", "standard|toolkit"),
            );
            return None;
        }
        return s.catalog.module(from).map(|i| i.presets);
    }
    if bp.bundle.iter().any(|b| b.name == from) {
        return s.catalog.bundle(from).map(|i| i.presets);
    }
    s.err(Coded::new("ruleset-source-unknown").with("name", from));
    None
}

/// The place of the forms description: a folder in the root of the repo.
pub const FORMS_DIR: &str = "forms/";

pub fn frontend(s: &mut Sink, bp: &Blueprint) {
    let f = &bp.frontend;
    if is_repo_path(&f.layout) {
        // the forms description lies in the folder `forms/` in the root of the repo, next to `scaffold.toml` (decision of the strategy 2026-11-18)
        if !f.layout.starts_with(FORMS_DIR) || f.layout.len() == FORMS_DIR.len() {
            s.err(Coded::new("layout-outside-forms").with("path", f.layout.as_str()));
        }
        if s.view.exists(&f.layout) {
            if let Some(cause) = s.view.read(&f.layout).as_deref().and_then(unreadable) {
                s.err(
                    Coded::new("layout-unreadable")
                        .with("path", f.layout.as_str())
                        .with("cause", cause),
                );
            }
        } else {
            s.err(Coded::new("layout-not-found").with("path", f.layout.as_str()));
        }
    } else {
        s.err(Coded::new("path-invalid").with("path", f.layout.as_str()));
    }
    if f.standard.is_empty() {
        s.err(
            Coded::new("field-empty")
                .with("table", "frontend")
                .with("field", "standard"),
        );
    } else if !is_kebab(&f.standard) {
        s.err(
            Coded::new("name-case")
                .with("key", "frontend.standard")
                .with("found", f.standard.as_str()),
        );
    }
    match &f.covers {
        Covers::Word(w) if w != "all" => {
            s.err(Coded::new("covers-invalid").with("found", w.as_str()));
        }
        Covers::List(l) if !l.is_empty() => {
            for e in l.iter().filter(|e| !is_covers_entry(e)) {
                s.err(Coded::new("covers-invalid").with("found", e.as_str()));
            }
        }
        Covers::List(l) if l.is_empty() => {
            s.err(
                Coded::new("field-empty")
                    .with("table", "frontend")
                    .with("field", "covers"),
            );
        }
        _ => {}
    }
}
