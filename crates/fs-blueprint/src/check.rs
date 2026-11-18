//! Pattern: Template method (the check runs the parts in a fixed order; program, contract, modules, bundles here, types and the rest in the siblings).

use crate::check_exc::exceptions;
use crate::check_front::{frontend, rulesets};
use crate::check_types::types;
use crate::error::{Coded, Report};
use crate::form::{is_file_id, is_kebab, is_major_minor, is_semver};
use crate::kit::{Kind, KitCatalog, RepoView};
use crate::model::{Blueprint, Bundle, Module, Program};
use crate::rules::{Rule, Rules};
use crate::sink::Sink;

/// Desktop main categories (freedesktop Desktop Menu Specification): at least one must be there.
const MAIN_CATEGORIES: &[&str] = &[
    "AudioVideo",
    "Audio",
    "Video",
    "Development",
    "Education",
    "Game",
    "Graphics",
    "Network",
    "Office",
    "Science",
    "Settings",
    "System",
    "Utility",
];

/// Checks a parsed blueprint against the catalog and the repo. Findings come in a fixed order (the order of the tables).
#[must_use]
pub fn check(
    bp: &Blueprint,
    catalog: &dyn KitCatalog,
    view: &dyn RepoView,
    rules: &Rules,
) -> Report {
    let mut s = Sink::new(rules, catalog, view);
    program(&mut s, &bp.program);
    contract(&mut s, bp);
    modules(&mut s, bp);
    bundles(&mut s, bp);
    types(&mut s, bp);
    rulesets(&mut s, bp);
    frontend(&mut s, bp);
    exceptions(&mut s, bp);
    s.report
}

/// Parses and checks in one go: a text that does not load is a report with the one code of the loading error.
#[must_use]
pub fn check_source(
    src: &str,
    catalog: &dyn KitCatalog,
    view: &dyn RepoView,
    rules: &Rules,
) -> Report {
    match Blueprint::from_toml(src) {
        Ok(bp) => check(&bp, catalog, view, rules),
        Err(code) => Report {
            errors: vec![code],
            warnings: Vec::new(),
        },
    }
}

fn program(s: &mut Sink, p: &Program) {
    if !is_kebab(&p.name) || p.name.starts_with("fs-") {
        s.rule(
            Rule::NameRule,
            Coded::new("name-rule").with("name", p.name.as_str()),
        );
    }
    if !is_semver(&p.skeleton) {
        s.err(
            Coded::new("semver-invalid")
                .with("key", "program.skeleton")
                .with("found", p.skeleton.as_str()),
        );
    } else if s.catalog.skeleton_known(&p.skeleton) {
        for part in s.catalog.skeleton_pending(&p.skeleton) {
            s.warn(Coded::new("skeleton-part-pending").with("part", part));
        }
    } else {
        s.rule(
            Rule::ExistingOnly,
            Coded::new("skeleton-unknown").with("found", p.skeleton.as_str()),
        );
    }
    if !s.catalog.capability_known(&p.capability) {
        s.rule(
            Rule::ExistingOnly,
            Coded::new("capability-unknown").with("key", p.capability.as_str()),
        );
    }
    s.text("program.summary", &p.summary);
    s.text("program.description", &p.description);
    if p.categories.is_empty() {
        s.err(
            Coded::new("field-empty")
                .with("table", "program")
                .with("field", "categories"),
        );
    } else if !p
        .categories
        .iter()
        .any(|c| MAIN_CATEGORIES.contains(&c.as_str()))
    {
        s.err(Coded::new("category-main-missing"));
    }
}

fn contract(s: &mut Sink, bp: &Blueprint) {
    let c = &bp.contract;
    if !is_major_minor(&c.version) {
        s.err(Coded::new("contract-version-invalid").with("found", c.version.as_str()));
    }
    if !is_file_id(&c.id) {
        s.err(Coded::new("contract-id-invalid").with("found", c.id.as_str()));
    }
    for key in &c.provides {
        if !s.catalog.capability_known(key) {
            s.rule(
                Rule::ExistingOnly,
                Coded::new("capability-unknown").with("key", key.as_str()),
            );
        }
    }
}

fn version_ok(s: &mut Sink, name: &str, version: &str, tags: &[String]) {
    if !is_semver(version) {
        s.err(
            Coded::new("semver-invalid")
                .with("key", format!("{name}.version"))
                .with("found", version),
        );
    } else if !tags.iter().any(|t| t == version) {
        s.rule(
            Rule::ExistingOnly,
            Coded::new("version-unknown")
                .with("module", name)
                .with("version", version),
        );
    }
}

fn modules(s: &mut Sink, bp: &Blueprint) {
    s.unique("module", bp.module.iter().map(|m| m.name.as_str()));
    for m in &bp.module {
        module(s, bp, m);
    }
}

fn module(s: &mut Sink, bp: &Blueprint, m: &Module) {
    let declared = Kind::parse(&m.kind);
    if declared.is_none() {
        s.err(
            Coded::new("kind-unknown")
                .with("module", m.name.as_str())
                .with("found", m.kind.as_str()),
        );
    }
    for b in bp.bundle.iter().filter(|b| b.parts.contains(&m.name)) {
        s.err(
            Coded::new("module-in-bundle")
                .with("module", m.name.as_str())
                .with("bundle", b.name.as_str()),
        );
    }
    let Some(info) = s.catalog.module(&m.name) else {
        s.rule(
            Rule::ExistingOnly,
            Coded::new("module-unknown").with("name", m.name.as_str()),
        );
        return;
    };
    if let Some(d) = declared {
        if d != info.kind {
            s.err(
                Coded::new("kind-mismatch")
                    .with("module", m.name.as_str())
                    .with("declared", d.as_str())
                    .with("catalog", info.kind.as_str()),
            );
        }
    }
    version_ok(s, &m.name, &m.version, &info.tags);
}

fn bundles(s: &mut Sink, bp: &Blueprint) {
    s.unique("bundle", bp.bundle.iter().map(|b| b.name.as_str()));
    for b in &bp.bundle {
        bundle(s, bp, b);
    }
}

fn bundle(s: &mut Sink, bp: &Blueprint, b: &Bundle) {
    if b.parts.is_empty() {
        s.err(
            Coded::new("field-empty")
                .with("table", "bundle")
                .with("field", "parts"),
        );
    }
    if !bp.types.iter().any(|t| t.name == b.item) {
        s.err(Coded::new("type-unknown").with("name", b.item.as_str()));
    }
    let Some(info) = s.catalog.bundle(&b.name) else {
        s.rule(
            Rule::ExistingOnly,
            Coded::new("bundle-unknown").with("name", b.name.as_str()),
        );
        return;
    };
    for part in b.parts.iter().filter(|p| !info.parts.contains(p)) {
        s.rule(
            Rule::ExistingOnly,
            Coded::new("bundle-part-unknown")
                .with("bundle", b.name.as_str())
                .with("part", part.as_str()),
        );
    }
    version_ok(s, &b.name, &b.version, &info.tags);
}
