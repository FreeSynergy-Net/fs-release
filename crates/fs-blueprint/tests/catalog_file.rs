//! The catalog read from a file: what the shipped one holds, what a custom one can say, what it refuses.
use fs_blueprint::catalog_file::BUILTIN;
use fs_blueprint::{FileCatalog, Kind, KitCatalog};

#[test]
fn the_shipped_catalog_is_valid_and_knows_the_basic_types_of_fs_types_and_the_components() {
    let c = FileCatalog::builtin();
    for b in [
        "text",
        "multilineText",
        "email",
        "phone",
        "name",
        "dateTime",
        "language",
        "link",
        "reference",
        "choice",
        "keywords",
        "whole",
    ] {
        assert!(c.basic_known(b), "{b}");
    }
    assert!(!c.basic_known("timeSpan") && !c.basic_known("Text"));
    assert!(c.skeleton_known("0.1.0") && !c.skeleton_known("0.2.0"));
    assert_eq!(
        c.skeleton_pending("0.1.0"),
        ["session", "update", "mcp-switch"]
    );
    assert_eq!(c.skeleton_pending("9.9.9"), Vec::<String>::new());
    assert!(
        c.capability_known("tasks")
            && c.capability_known("people.persons")
            && !c.capability_known("nothing")
    );
    let board = c.module("board").unwrap();
    assert_eq!(
        (board.kind, board.tags.as_slice()),
        (Kind::Component, ["0.0.29".to_string()].as_slice())
    );
    assert!(
        c.module("fs-todo").is_none() && c.bundle("fs-planning").is_none(),
        "what does not exist is not listed"
    );
    assert!(BUILTIN.contains("[[skeleton]]"));
}

#[test]
fn a_custom_catalog_gives_modules_bundles_kits_words_and_presets() {
    let c = FileCatalog::parse(
        "kits = [\"timeSpan\"]\ncapabilities = [\"a.b\"]\n[[module]]\nname = \"fs-x\"\nkind = \"standard\"\ntags = [\"0.0.1\", \"0.0.2\"]\nwords = [\"state\"]\npresets = [\"p\"]\n[[bundle]]\nname = \"fs-y\"\nparts = [\"one\", \"two\"]\ntags = [\"1.0.0\"]\npresets = [\"q\"]\n[[skeleton]]\nversion = \"1.0.0\"\n",
    )
    .unwrap();
    assert!(c.kit_known("timeSpan") && c.capability_known("a.b") && c.skeleton_known("1.0.0"));
    let m = c.module("fs-x").unwrap();
    assert_eq!(
        (
            m.kind,
            m.tags.len(),
            m.words.as_slice(),
            m.presets.as_slice()
        ),
        (
            Kind::Standard,
            2,
            ["state".to_string()].as_slice(),
            ["p".to_string()].as_slice()
        )
    );
    let b = c.bundle("fs-y").unwrap();
    assert_eq!(
        (b.parts.len(), b.tags.as_slice(), b.presets.as_slice()),
        (
            2,
            ["1.0.0".to_string()].as_slice(),
            ["q".to_string()].as_slice()
        )
    );
}

#[test]
fn an_unknown_key_a_wrong_kind_and_no_toml_are_refused_with_a_reason() {
    for (text, part) in [
        ("nope = 1\n", "unknown field"),
        ("[[module]]\nname = \"x\"\nkind = \"gadget\"\n", "gadget"),
        ("[[module]]\nname = \"x\"\n", "missing field"),
        (
            "[[skeleton]]\nversion = \"1\"\nextra = 1\n",
            "unknown field",
        ),
        ("this is no toml [", "TOML"),
    ] {
        let e = FileCatalog::parse(text).unwrap_err();
        assert!(e.contains(part), "{text:?}: {e}");
    }
    assert!(FileCatalog::parse("").is_ok());
}
