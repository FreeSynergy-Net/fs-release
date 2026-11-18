//! Paths that are the same file on every platform: what Windows refuses or changes is refused everywhere (reserved device names, a name that ends in a dot or a
//! space, `\\?\`, a drive, alternate data streams), and two paths that differ only in case are one file.
mod support;
use fs_blueprint::emit::write::{write, DirStore};
use fs_blueprint::emit::Emitter;
use fs_blueprint::{Artifact, Blueprint, Class, Coded, KitCatalog};
use support::synth::blueprint;
use support::{generate_with, FakeCatalog};

fn valid(p: &str) -> bool {
    fs_blueprint::form::is_repo_path(p)
}

#[test]
fn good_paths_pass_also_folders_with_an_end_slash() {
    for p in [
        "a.rs",
        "crates/fs-x-bin/src/main.rs",
        "crates/fs-x-core/",
        ".github/workflows/check.yml",
        ".gitattributes",
        "a/b.c.d",
        "conx.rs",
        "nullable.rs",
        "com10.rs",
        "com0.rs",
        "lpt.txt",
    ] {
        assert!(valid(p), "{p}");
    }
}

#[test]
fn windows_reserved_device_names_are_refused_with_or_without_extension_in_any_case() {
    for p in [
        "con",
        "CON",
        "a/nul.txt",
        "a/NUL.TXT",
        "prn.rs",
        "aux",
        "Com1",
        "x/com9.rs",
        "lpt1",
        "LPT9.md",
        "dir/con/x.rs",
        "con.tar.gz",
    ] {
        assert!(!valid(p), "{p}");
    }
}

#[test]
fn a_name_that_ends_in_a_dot_or_a_space_is_refused() {
    for p in [
        "a.",
        "a /b.rs",
        "a/b.rs.",
        "a/b.rs ",
        "dir./x.rs",
        "x/.. /y",
    ] {
        assert!(!valid(p), "{p:?}");
    }
}

#[test]
fn drives_unc_extended_prefixes_streams_and_odd_characters_are_refused() {
    for p in [
        "C:/x.rs",
        "c:x.rs",
        "a/b:stream",
        "\\\\?\\C:\\x.rs",
        "\\\\server\\share\\x",
        "//server/share/x",
        "a\\b.rs",
        "a/b|c",
        "a/b<c",
        "a/b>c",
        "a/\"b\"",
        "a/b*c",
        "a/b?c",
        "a\0b",
        "a/b\nc",
        "a/\tb",
    ] {
        assert!(!valid(p), "{p:?}");
    }
}

#[test]
fn dots_empty_parts_and_absolute_paths_are_refused() {
    for p in [
        "", "/", "/a", "./a", "a/./b", "../a", "a/../b", "a//b", "a/..", "//",
    ] {
        assert!(!valid(p), "{p:?}");
    }
}

struct Two;
impl Emitter for Two {
    fn emit(&self, _: &Blueprint, _: &dyn KitCatalog) -> Result<Vec<Artifact>, Coded> {
        Ok(vec![
            Artifact::new("Readme.md", Class::Generated, "x\n"),
            Artifact::new("README.md", Class::Generated, "y\n"),
        ])
    }
}

#[test]
fn two_paths_that_differ_only_in_case_are_one_file_and_refused() {
    let e = generate_with(&blueprint(0), &FakeCatalog, &[&Two]).unwrap_err();
    assert_eq!(e.to_string(), "artifact-duplicate{path=README.md}");
}

#[test]
fn write_refuses_every_path_the_check_refuses() {
    let root = std::path::Path::new(env!("CARGO_TARGET_TMPDIR"))
        .join(format!("winpaths-{}", std::process::id()));
    for bad in ["con.txt", "a./b.rs", "x:y.rs", "a/b ", "NUL"] {
        let mut store = DirStore::new(&root);
        let list = vec![Artifact::new(bad, Class::Generated, "x\n")];
        let err = write(&list, &mut store).unwrap_err();
        assert_eq!(
            err[0].to_string(),
            format!("path-invalid{{path={bad}}}"),
            "{bad}"
        );
    }
    assert!(!root.exists());
}
