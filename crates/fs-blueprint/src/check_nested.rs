//! Pattern: Template method (the checks of nested types and references: targets exist, no cycle, a nested type has one parent only).

use crate::error::Coded;
use crate::form::is_iri;
use crate::model::{Blueprint, Field};
use crate::sink::Sink;
use std::collections::BTreeSet;

/// The target of a `nested` or `reference` field: an own type (`reference` may also be an IRI).
pub fn target(s: &mut Sink, bp: &Blueprint, f: &Field) {
    let known = |name: &str| bp.own_type(name).is_some();
    if let Some(n) = f.nested.as_deref().filter(|n| !known(n)) {
        s.err(Coded::new("type-unknown").with("name", n));
    }
    if let Some(r) = f.reference.as_deref().filter(|r| !is_iri(r) && !known(r)) {
        s.err(Coded::new("type-unknown").with("name", r));
    }
    // a nested type has no table of its own name (it is `<parent>_<field>`): a key to it would point nowhere
    if let Some(r) = f
        .reference
        .as_deref()
        .filter(|r| !is_iri(r) && bp.is_nested(r))
    {
        s.err(Coded::new("reference-to-nested").with("name", r));
    }
}

/// The nesting as a whole: a type is nested by one field only (`nested-twice`) and never inside itself, directly or through others (`nested-cycle`).
pub fn graph(s: &mut Sink, bp: &Blueprint) {
    let mut seen = BTreeSet::new();
    let mut twice = BTreeSet::new();
    for n in bp
        .own_types()
        .flat_map(|t| &t.field)
        .filter_map(|f| f.nested.as_deref())
    {
        if !seen.insert(n) {
            twice.insert(n);
        }
    }
    for n in twice {
        s.err(Coded::new("nested-twice").with("type", n));
    }
    for t in bp.own_types() {
        if reaches(bp, &t.name, &t.name, &mut BTreeSet::new()) {
            s.err(Coded::new("nested-cycle").with("type", t.name.as_str()));
        }
    }
}

/// True when `from` nests `goal`, directly or through other nested types.
fn reaches<'a>(bp: &'a Blueprint, from: &str, goal: &str, seen: &mut BTreeSet<&'a str>) -> bool {
    let Some(t) = bp.own_type(from) else {
        return false;
    };
    t.field
        .iter()
        .filter_map(|f| f.nested.as_deref())
        .any(|n| n == goal || (seen.insert(n) && reaches(bp, n, goal, seen)))
}
