//! Line endings in the output and decision 45 in `generate`: no carriage return, an exception never takes the entry point or another generated file.
mod support;
use fs_blueprint::emit::slot::{fill, Values};
use fs_blueprint::emit::Emitter;
use fs_blueprint::{Artifact, Blueprint, Class, Coded, KitCatalog};
use support::synth::{blueprint, text};
use support::{edit, generate, generate_with, FakeCatalog, TASKS};

#[test]
fn a_template_with_crlf_gives_the_same_bytes_as_one_with_lf_and_a_lone_cr_is_refused() {
    let v = Values::of(&blueprint(0)).unwrap();
    let lf = "a {{kebab}}\nb\n";
    assert_eq!(
        fill(lf, &v, "p").unwrap(),
        fill("a {{kebab}}\r\nb\r\n", &v, "p").unwrap()
    );
    assert_eq!(
        fill("a\rb", &v, "p").unwrap_err().to_string(),
        "line-ending{path=p}"
    );
}

#[test]
fn no_generated_byte_is_a_carriage_return_in_any_blueprint() {
    for i in 0..support::synth::COUNT {
        for a in generate(&blueprint(i), &FakeCatalog).unwrap() {
            assert!(!a.content.contains('\r'), "{}", a.path);
        }
    }
    let bp = Blueprint::from_toml(TASKS).unwrap();
    assert!(generate(&bp, &FakeCatalog)
        .unwrap()
        .iter()
        .all(|a| !a.content.contains('\r')));
}

struct Cr;
impl Emitter for Cr {
    fn emit(&self, _: &Blueprint, _: &dyn KitCatalog) -> Result<Vec<Artifact>, Coded> {
        Ok(vec![Artifact::new("a.rs", Class::Generated, "x\r\n")])
    }
}

#[test]
fn an_emitter_that_brings_a_carriage_return_is_refused() {
    let e = generate_with(&blueprint(0), &FakeCatalog, &[&Cr]).unwrap_err();
    assert_eq!(e.to_string(), "line-ending{path=a.rs}");
}

#[test]
fn the_repo_has_an_attributes_rule_that_keeps_the_crate_files_at_lf() {
    let attr = include_str!("../../../.gitattributes");
    assert!(attr
        .lines()
        .any(|l| l.starts_with("crates/fs-blueprint/**") && l.contains("eol=lf")));
}

fn with_exception(path: &str) -> Blueprint {
    let src = edit(
        &text(0),
        "[frontend]",
        &format!("[[exception]]\nwhat = \"w\"\nwhy = \"y\"\nchecked = [\"x\"]\nalternatives = []\ntalk = \"t.md\"\npaths = [\"{path}\"]\n\n[frontend]"),
    );
    Blueprint::from_toml(&src).unwrap()
}

#[test]
fn an_exception_over_a_former_entry_point_is_no_longer_taken_by_generate_the_check_still_reserves_main_files(
) {
    // generate makes no Rust source any more: nothing generated is left to cover
    let out = generate(
        &with_exception("crates/fs-alpha-desk-bin/src/main.rs"),
        &FakeCatalog,
    )
    .unwrap();
    assert!(out.iter().all(|a| a.class != Class::Exception));
}

#[test]
fn an_exception_over_another_generated_file_is_path_generated_and_over_a_hand_written_manifest_nothing_is_made(
) {
    let e = generate(&with_exception(".gitattributes"), &FakeCatalog).unwrap_err();
    assert_eq!(e.to_string(), "path-generated{path=.gitattributes}");
    // no manifest is generated any more: the path is the program's own, generate has nothing to take or to mark
    let out = generate(
        &with_exception("crates/fs-alpha-desk-core/Cargo.toml"),
        &FakeCatalog,
    )
    .unwrap();
    assert!(out
        .iter()
        .all(|a| a.class != Class::Exception && !a.path.ends_with("Cargo.toml")));
}
