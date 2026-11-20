//! Pattern: Template method (the exceptions: what they say, where their talk is, and which files they cover; `paths`, ADR 0099 addendum point 7).

use crate::error::Coded;
use crate::form::is_repo_path;
use crate::model::Blueprint;
use crate::rules::Rule;
use crate::sink::Sink;

/// True when path `a` covers path `b`: the same path, or `a` is a folder (end slash) that contains `b`.
#[must_use]
pub fn covers(a: &str, b: &str) -> bool {
    a == b || (a.ends_with('/') && b.starts_with(a))
}

fn is_rust_source(path: &str) -> bool {
    std::path::Path::new(path)
        .extension()
        .is_some_and(|x| x == "rs")
}

pub fn exceptions(s: &mut Sink, bp: &Blueprint) {
    let mains = s.view.main_files();
    let mut valid: Vec<(usize, &str)> = Vec::new();
    for (i, e) in bp.exception.iter().enumerate() {
        for (key, text) in [("what", &e.what), ("why", &e.why)] {
            if text.trim().is_empty() {
                s.err(
                    Coded::new("field-empty")
                        .with("table", "exception")
                        .with("field", key),
                );
            }
        }
        if e.checked.is_empty() {
            s.err(
                Coded::new("field-empty")
                    .with("table", "exception")
                    .with("field", "checked"),
            );
        }
        if e.paths.is_empty() {
            s.err(
                Coded::new("field-empty")
                    .with("table", "exception")
                    .with("field", "paths"),
            );
        }
        for p in &e.paths {
            if !is_repo_path(p) {
                s.err(Coded::new("path-invalid").with("path", p.as_str()));
                continue;
            }
            valid.push((i + 1, p.as_str()));
            if !s.view.exists(p) {
                s.err(Coded::new("path-not-found").with("path", p.as_str()));
            }
            if mains.iter().any(|m| covers(p, m)) {
                s.err(Coded::new("main-reserved").with("path", p.as_str()));
            }
            if s.view.covers_generated(p) {
                s.err(Coded::new("path-generated").with("path", p.as_str()));
            }
        }
        if !s.view.talk_exists(&e.talk) {
            s.rule(
                Rule::TalkPerException,
                Coded::new("talk-not-found").with("path", e.talk.as_str()),
            );
        }
    }
    overlaps(s, &valid);
    let mut files = s.view.unowned_files();
    files.sort();
    for f in files {
        if !valid.iter().any(|(_, p)| covers(p, &f)) {
            // a program is data (ADR 0086): hand-written Rust without an exception is code in the program, besides being unowned
            if is_rust_source(&f) {
                s.err(Coded::new("code-in-program").with("path", f.as_str()));
            }
            s.err(Coded::new("file-unowned").with("path", f));
        }
    }
}

/// Two different exceptions must not cover a common file: the more specific of the two paths is named.
fn overlaps(s: &mut Sink, valid: &[(usize, &str)]) {
    for (k, (i, a)) in valid.iter().enumerate() {
        for (j, b) in &valid[k + 1..] {
            if i == j || !(covers(a, b) || covers(b, a)) {
                continue;
            }
            let deeper = if covers(a, b) { b } else { a };
            s.err(
                Coded::new("path-overlap")
                    .with("path", *deeper)
                    .with("first", i.to_string())
                    .with("second", j.to_string()),
            );
        }
    }
}
