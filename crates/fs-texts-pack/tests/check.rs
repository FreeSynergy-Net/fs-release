//! Pattern: none (test). The pair check: a gap is an error with a clear message, never silent.

mod support;
use support::{problems, with};

#[test]
fn a_message_missing_in_de_is_an_error() {
    let p = problems(&with("de", "hello = Hallo, {$name}!\n"));
    assert!(
        p.iter().any(|m| m == "files: in en, missing in de"),
        "{p:?}"
    );
    assert!(p.iter().any(|m| m == "when: in en, missing in de"), "{p:?}");
}

#[test]
fn a_message_missing_in_en_is_an_error() {
    let en =
        std::fs::read_to_string(concat!(env!("CARGO_MANIFEST_DIR"), "/tests/data/en.mf2")).unwrap();
    let p = problems(&with("en", &en.replace("hello = Hello, {$name}!\n", "")));
    assert!(
        p.iter().any(|m| m == "hello: in de, missing in en"),
        "{p:?}"
    );
}

#[test]
fn different_variables_are_an_error() {
    let p = problems(&with("de", "hello = Hallo, {$who}!\nfiles = x\nwhen = y\n"));
    assert!(
        p.iter().any(|m| m.starts_with("hello: variables differ")),
        "{p:?}"
    );
}

#[test]
fn a_broken_message_a_bad_line_and_a_double_id_are_errors() {
    let p = problems(&with(
        "de",
        "hello = Hallo, {$name\nfiles = x\nfiles = y\nwhen = z\nno equals\n",
    ));
    assert!(
        p.iter()
            .any(|m| m.contains("hello") && m.contains("does not parse")),
        "{p:?}"
    );
    assert!(
        p.iter().any(|m| m.contains("files is given twice")),
        "{p:?}"
    );
    assert!(p.iter().any(|m| m.contains("no `=`")), "{p:?}");
}

#[test]
fn en_and_de_are_both_required() {
    let only_en: Vec<_> = support::data()
        .into_iter()
        .filter(|s| s.language == "en")
        .collect();
    assert!(problems(&only_en).iter().any(|m| m == "no source for de"));
    let only_de: Vec<_> = support::data()
        .into_iter()
        .filter(|s| s.language == "de")
        .collect();
    assert!(problems(&only_de).iter().any(|m| m == "no source for en"));
}

#[test]
fn a_language_asked_for_without_a_source_is_an_error() {
    let e = fs_texts_pack::pack(&support::data(), &["fr".to_owned()]).unwrap_err();
    assert!(matches!(e, fs_texts_pack::PackError::NoSuchLanguage(l) if l == "fr"));
}
