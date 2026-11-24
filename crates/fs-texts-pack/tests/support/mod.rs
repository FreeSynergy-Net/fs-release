//! Pattern: none (test helpers). The two small sources of `tests/data`.
#![allow(dead_code, clippy::missing_panics_doc, clippy::must_use_candidate)]

use fs_texts_pack::{read_dir, Source};
use std::path::Path;

pub fn data() -> Vec<Source> {
    read_dir(&Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/data")).expect("tests/data")
}

pub fn with(language: &str, text: &str) -> Vec<Source> {
    let mut s: Vec<Source> = data()
        .into_iter()
        .filter(|s| s.language != language)
        .collect();
    s.push(Source {
        language: language.to_owned(),
        text: text.to_owned(),
    });
    s
}

pub fn problems(sources: &[Source]) -> Vec<String> {
    match fs_texts_pack::pack(sources, &[]) {
        Err(fs_texts_pack::PackError::Check(list)) => list,
        other => panic!("expected a check error, got {other:?}"),
    }
}
