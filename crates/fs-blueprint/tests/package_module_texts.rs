#![cfg(target_os = "linux")]
//! Pattern: none (test). `fs-blueprint package module` and the optional `texts/` of a module: packed with the library of `pack-texts` into `catalog/texts/`,
//! without `dates/`; no `texts/` changes nothing; a gap or a broken source is an error.
mod common;
use common::{cli, have_rpm, rpm, scratch};
use std::path::{Path, PathBuf};

const EN: &str = "person.emails = Emails\nperson.titles = Titles\n";
const DE: &str = "person.emails = E-Mails\nperson.titles = Titel\n";

fn module(name: &str, texts: &[(&str, &str)]) -> (PathBuf, PathBuf) {
    let d = scratch(name);
    std::fs::create_dir_all(d.join("catalog")).unwrap();
    for (f, t) in [
        ("catalog/commands.json", "[]"),
        ("catalog/forms.json", "{}"),
        ("metainfo.xml", "<component/>"),
        ("LICENSE-MIT", "MIT"),
        ("LICENSE-APACHE", "Apache"),
        ("service", "\u{7f}ELF"),
    ] {
        std::fs::write(d.join(f), t).unwrap();
    }
    if !texts.is_empty() {
        std::fs::create_dir_all(d.join("texts")).unwrap();
        for (f, t) in texts {
            std::fs::write(d.join("texts").join(f), t).unwrap();
        }
    }
    let bin = d.join("service");
    (d, bin)
}

fn has_ext(p: &str, ext: &str) -> bool {
    Path::new(p).extension().is_some_and(|e| e == ext)
}

fn make(out: &Path, d: &Path, bin: &Path) -> (i32, String) {
    let (code, o, e) = cli(&[
        "package",
        "module",
        d.to_str().unwrap(),
        "--name",
        "person",
        "--pkg-version",
        "0.0.4",
        "--binary",
        bin.to_str().unwrap(),
        "--arch",
        "x86_64",
        "--out",
        out.to_str().unwrap(),
    ]);
    (code, format!("{o}{e}"))
}

fn list_of(texts: &[(&str, &str)], tag: &str) -> String {
    let (d, bin) = module(tag, texts);
    let out = scratch(&format!("{tag}-out"));
    let (code, msg) = make(&out, &d, &bin);
    assert_eq!(code, 0, "{msg}");
    rpm(&["-qpl"], &out.join("fs-module-person-0.0.4-1.x86_64.rpm"))
}

#[test]
fn texts_are_packed_into_catalog_texts_without_dates() {
    if !have_rpm() {
        return;
    }
    let list = list_of(&[("en.mf2", EN), ("de.mf2", DE)], "tx-ok");
    let base = "/usr/libexec/freesynergy/modules/person/catalog/texts";
    assert!(
        list.lines().any(|l| l == format!("{base}/manifest.mf2m")),
        "{list}"
    );
    for tag in ["en", "de"] {
        assert!(
            list.lines()
                .any(|l| l.starts_with(&format!("{base}/{tag}.")) && has_ext(l, "mf2b")),
            "{tag}: {list}"
        );
    }
    assert!(!list.contains("dates"), "{list}");
    assert!(
        !list.contains("icu.table") && !list.contains("names.blob"),
        "{list}"
    );
    assert!(
        !list.lines().any(|l| has_ext(l, "mf2")),
        "sources are not packaged: {list}"
    );
}

#[test]
fn a_module_without_texts_is_unchanged() {
    if !have_rpm() {
        return;
    }
    let list = list_of(&[], "tx-none");
    assert!(!list.contains("catalog/texts"), "{list}");
    assert_eq!(
        list.lines().filter(|l| l.contains("/catalog/")).count(),
        2,
        "{list}"
    );
}

#[test]
fn a_gap_or_a_broken_source_is_an_error() {
    for (tag, texts) in [
        ("tx-only-en", vec![("en.mf2", EN)]),
        (
            "tx-pair",
            vec![("en.mf2", EN), ("de.mf2", "person.emails = E-Mails\n")],
        ),
        (
            "tx-broken",
            vec![
                ("en.mf2", EN),
                ("de.mf2", "person.emails = {oops\nperson.titles = Titel\n"),
            ],
        ),
    ] {
        let (d, bin) = module(tag, &texts);
        let out = scratch(&format!("{tag}-out"));
        let (code, msg) = make(&out, &d, &bin);
        assert_ne!(code, 0, "{tag}: {msg}");
        assert!(msg.contains("texts"), "{tag}: {msg}");
        assert!(
            !out.join("fs-module-person-0.0.4-1.x86_64.rpm").exists(),
            "{tag}"
        );
    }
}
