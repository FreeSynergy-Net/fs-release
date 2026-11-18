//! Pattern: Template method (types and their fields, rule sets and the frontend of the blueprint).

use crate::error::Coded;
use crate::form::{is_camel, is_iri, is_media_type, is_pascal, is_standard_uri};
use crate::kit::Kind;
use crate::model::{Blueprint, Field, TypeDef};
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
            None => own(s, bp, t),
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

/// True when `text` contains `word` as a whole word (letters, digits and `-` and `_` belong to a word).
fn mentions(text: &str, word: &str) -> bool {
    let is_word = |c: char| c.is_alphanumeric() || c == '-' || c == '_';
    text.split(|c: char| !is_word(c)).any(|w| w == word)
}

/// A type without its own standard is a documented exception: an `[[exception]]` names the type in `what` and its talk record exists (user decision 6e, B6).
fn has_exception(s: &Sink, bp: &Blueprint, ty: &str) -> bool {
    bp.exception
        .iter()
        .any(|e| mentions(&e.what, ty) && s.view.talk_exists(&e.talk))
}

/// A program's own type: standard, envelope, media type and label are mandatory, and so is the standard reference of every field.
fn own(s: &mut Sink, bp: &Blueprint, t: &TypeDef) {
    match &t.standard {
        None if has_exception(s, bp, &t.name) => {}
        None => missing(s, "standard"),
        Some(u) if !is_standard_uri(u) => s.rule(
            Rule::StandardEvidence,
            Coded::new("standard-invalid")
                .with("type", t.name.as_str())
                .with("found", u.as_str()),
        ),
        Some(_) => {}
    }
    match &t.envelope {
        None => missing(s, "envelope"),
        Some(e) if !is_pascal(e) && !is_iri(e) => s.err(
            Coded::new("envelope-invalid")
                .with("type", t.name.as_str())
                .with("found", e.as_str()),
        ),
        Some(_) => {}
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
        field(s, bp, t, f);
    }
}

fn field(s: &mut Sink, bp: &Blueprint, t: &TypeDef, f: &Field) {
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
    } else if let Some(u) = f.standard.as_deref().filter(|u| !is_standard_uri(u)) {
        s.rule(
            Rule::StandardEvidence,
            Coded::new("standard-invalid")
                .with("type", ty)
                .with("found", u),
        );
    }
    if let Some(w) = &f.words {
        words(s, bp, f, w);
    }
    s.text(&format!("type.{ty}.{}.label", f.name), &f.label);
}

/// `words = "<module>:<list>"` only for `choice` / `keywords`, and the module must offer the list.
fn words(s: &mut Sink, bp: &Blueprint, f: &Field, reference: &str) {
    let bad = || Coded::new("words-unknown").with("ref", reference);
    let Some((module, list)) = reference.split_once(':') else {
        return s.err(bad());
    };
    if !matches!(f.basic.as_deref(), Some("choice" | "keywords")) {
        return s.err(bad());
    }
    // only modules that stand in the blueprint (a part of a bundle is no module here)
    if !bp.module.iter().any(|m| m.name == module) {
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
