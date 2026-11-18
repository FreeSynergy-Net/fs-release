//! Pattern: Template method (types and their fields, rule sets and the frontend of the blueprint).

use crate::error::Coded;
use crate::form::{is_camel, is_media_type, is_pascal, is_repo_path, is_standard_uri};
use crate::kit::Kind;
use crate::model::{Blueprint, Covers, Field, TypeDef};
use crate::rules::Rule;
use crate::sink::Sink;

pub fn types(s: &mut Sink, bp: &Blueprint) {
    s.unique("type", bp.types.iter().map(|t| t.name.as_str()));
    for t in &bp.types {
        if !is_pascal(&t.name) {
            s.err(
                Coded::new("name-case")
                    .with("key", "type.name")
                    .with("found", t.name.as_str()),
            );
        }
        match &t.module {
            Some(m) => of_module(s, bp, t, m),
            None => own(s, t),
        }
    }
}

/// A type of a module carries nothing of its own (point 6).
fn of_module(s: &mut Sink, bp: &Blueprint, t: &TypeDef, module: &str) {
    match bp.module.iter().find(|m| m.name == module) {
        None => s.err(Coded::new("module-unknown").with("name", module)),
        Some(m) if Kind::parse(&m.kind) != Some(Kind::Standard) => {
            s.err(
                Coded::new("kind-wrong-role")
                    .with("module", module)
                    .with("expected", "standard"),
            );
        }
        Some(_) => {}
    }
    let own = [
        ("standard", t.standard.is_some()),
        ("envelope", t.envelope.is_some()),
        ("media_type", t.media_type.is_some()),
        ("label", t.label.is_some()),
    ];
    for (key, present) in own {
        if present {
            s.err(
                Coded::new("owned-by-module")
                    .with("type", t.name.as_str())
                    .with("key", key),
            );
        }
    }
    if !t.field.is_empty() {
        s.err(
            Coded::new("owned-by-module")
                .with("type", t.name.as_str())
                .with("key", "field"),
        );
    }
}

fn missing(s: &mut Sink, field: &'static str) {
    s.err(
        Coded::new("field-missing")
            .with("table", "type")
            .with("field", field),
    );
}

/// A program's own type: standard, envelope, media type and label are mandatory, and so is the standard reference of every field.
fn own(s: &mut Sink, t: &TypeDef) {
    match &t.standard {
        None => missing(s, "standard"),
        Some(u) if !is_standard_uri(u) => s.rule(
            Rule::StandardEvidence,
            Coded::new("standard-invalid")
                .with("type", t.name.as_str())
                .with("found", u.as_str()),
        ),
        Some(_) => {}
    }
    if t.envelope.is_none() {
        missing(s, "envelope");
    }
    match &t.media_type {
        None => missing(s, "media_type"),
        Some(m) if !is_media_type(m) => s.err(
            Coded::new("media-type-invalid")
                .with("type", t.name.as_str())
                .with("found", m.as_str()),
        ),
        Some(_) => {}
    }
    match &t.label {
        None => missing(s, "label"),
        Some(l) => s.text(&format!("type.{}.label", t.name), l),
    }
    s.unique("type.field", t.field.iter().map(|f| f.name.as_str()));
    for f in &t.field {
        field(s, t, f);
    }
}

fn field(s: &mut Sink, t: &TypeDef, f: &Field) {
    let ty = t.name.as_str();
    if !is_camel(&f.name) {
        s.err(
            Coded::new("name-case")
                .with("key", "type.field.name")
                .with("found", f.name.as_str()),
        );
    }
    match (&f.kit, &f.basic) {
        (Some(_), Some(_)) => s.err(Coded::new("field-kind-both").with("field", f.name.as_str())),
        (None, None) => s.err(Coded::new("field-kind-missing").with("field", f.name.as_str())),
        _ => {}
    }
    if let Some(k) = f.kit.as_deref().filter(|k| !s.catalog.kit_known(k)) {
        s.rule(
            Rule::ExistingOnly,
            Coded::new("kit-unknown").with("name", k),
        );
    }
    if let Some(b) = f.basic.as_deref().filter(|b| !s.catalog.basic_known(b)) {
        s.rule(
            Rule::ExistingOnly,
            Coded::new("basic-unknown").with("name", b),
        );
    }
    if f.standard.as_deref().is_none_or(str::is_empty) {
        s.rule(
            Rule::StandardEvidence,
            Coded::new("standard-missing")
                .with("type", ty)
                .with("field", f.name.as_str()),
        );
    }
    if let Some(w) = &f.words {
        words(s, f, w);
    }
    s.text(&format!("type.{ty}.{}.label", f.name), &f.label);
}

/// `words = "<module>:<list>"` only for `choice` / `keywords`, and the module must offer the list.
fn words(s: &mut Sink, f: &Field, reference: &str) {
    let bad = || Coded::new("words-unknown").with("ref", reference);
    let Some((module, list)) = reference.split_once(':') else {
        return s.err(bad());
    };
    if !matches!(f.basic.as_deref(), Some("choice" | "keywords")) {
        return s.err(bad());
    }
    if !s
        .catalog
        .module(module)
        .is_some_and(|m| m.words.iter().any(|w| w == list))
    {
        s.rule(Rule::ExistingOnly, bad());
    }
}

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

pub fn frontend(s: &mut Sink, bp: &Blueprint) {
    let f = &bp.frontend;
    if !is_repo_path(&f.layout) {
        s.err(Coded::new("path-invalid").with("path", f.layout.as_str()));
    } else if !s.view.exists(&f.layout) {
        s.err(Coded::new("layout-not-found").with("path", f.layout.as_str()));
    }
    match &f.covers {
        Covers::Word(w) if w != "all" => {
            s.err(Coded::new("covers-invalid").with("found", w.as_str()))
        }
        Covers::List(l) if l.is_empty() => s.err(
            Coded::new("field-empty")
                .with("table", "frontend")
                .with("field", "covers"),
        ),
        _ => {}
    }
}
