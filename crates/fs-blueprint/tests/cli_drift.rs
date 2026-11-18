//! The check on a program that `new` made (S5, plan: a hand change to a generated file is red, a file without a head under `crates/` is red).
mod support;
use fs_blueprint::cli::run;
use std::path::{Path, PathBuf};

fn fixture(name: &str) -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("tests/data/repos")
        .join(name)
}

fn cli(words: &[&str]) -> (i32, String) {
    let words: Vec<String> = words.iter().map(|w| (*w).to_string()).collect();
    let (mut out, mut err) = (Vec::new(), Vec::new());
    let code = run(&words, &mut out, &mut err);
    (
        code,
        String::from_utf8(out).unwrap() + &String::from_utf8(err).unwrap(),
    )
}

fn made(name: &str) -> PathBuf {
    let dir =
        Path::new(env!("CARGO_TARGET_TMPDIR")).join(format!("drift-{name}-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    let bp = fixture("ok").join("scaffold.toml");
    let (code, out) = cli(&[
        "new",
        &bp.to_string_lossy(),
        "--out",
        &dir.to_string_lossy(),
    ]);
    assert_eq!(code, 0, "{out}");
    dir
}

fn check(dir: &Path) -> (i32, String) {
    cli(&["check", "--repo", &dir.to_string_lossy(), "--drift"])
}

#[test]
fn a_program_made_by_new_is_clean() {
    let (code, out) = check(&made("clean"));
    assert_eq!(code, 0, "{out}");
}

#[test]
fn a_hand_change_to_a_generated_file_is_red_with_its_path_whatever_the_size_of_the_change() {
    for (n, change) in ["// hand\n", "\n", " "].into_iter().enumerate() {
        let dir = made(&format!("hand{n}"));
        let main = dir
            .join("crates")
            .join("fs-demo-bin")
            .join("src")
            .join("main.rs");
        let text = std::fs::read_to_string(&main).unwrap();
        std::fs::write(&main, format!("{text}{change}")).unwrap();
        let (code, out) = check(&dir);
        assert_eq!(code, 1, "{out}");
        assert!(
            out.contains("error: generated-changed{path=crates/fs-demo-bin/src/main.rs}"),
            "{out}"
        );
    }
}

#[test]
fn a_generated_file_that_was_deleted_and_a_seed_that_was_deleted_are_red() {
    let dir = made("deleted");
    std::fs::remove_file(dir.join("Cargo.toml")).unwrap();
    std::fs::remove_file(
        dir.join("crates")
            .join("fs-demo-core")
            .join("src")
            .join("lib.rs"),
    )
    .unwrap();
    let (code, out) = check(&dir);
    assert_eq!(code, 1, "{out}");
    assert!(
        out.contains("generated-missing{path=Cargo.toml}")
            && out.contains("seed-missing{path=crates/fs-demo-core/src/lib.rs}"),
        "{out}"
    );
}

#[test]
fn a_file_without_a_head_under_crates_is_red_but_a_seed_the_program_changed_and_a_tests_file_are_not(
) {
    let dir = made("unowned");
    let core_dir = dir.join("crates").join("fs-demo-core");
    std::fs::write(core_dir.join("src").join("extra.rs"), "pub fn x() {}\n").unwrap();
    let (code, out) = check(&dir);
    assert_eq!(code, 1, "{out}");
    assert!(
        out.contains("error: file-unowned{path=crates/fs-demo-core/src/extra.rs}"),
        "{out}"
    );
    assert!(
        !out.contains("fs-demo-core/src/lib.rs"),
        "the seed is the program's: {out}"
    );
    // the seed changed by the program and a file in a tests folder: clean again
    std::fs::remove_file(core_dir.join("src").join("extra.rs")).unwrap();
    let lib = core_dir.join("src").join("lib.rs");
    let text = std::fs::read_to_string(&lib).unwrap();
    std::fs::write(&lib, format!("{text}pub fn mine() {{}}\n")).unwrap();
    std::fs::create_dir_all(core_dir.join("tests")).unwrap();
    std::fs::write(core_dir.join("tests").join("t.rs"), "#[test]\nfn t() {}\n").unwrap();
    let (code, out) = check(&dir);
    assert_eq!(code, 0, "{out}");
}

#[test]
fn the_drift_is_shown_together_with_a_hand_written_file_and_with_a_missing_talk_record() {
    let dir = made("together");
    let main = dir
        .join("crates")
        .join("fs-demo-bin")
        .join("src")
        .join("main.rs");
    let text = std::fs::read_to_string(&main).unwrap();
    std::fs::write(&main, format!("{text}// hand\n")).unwrap();
    std::fs::write(
        dir.join("crates")
            .join("fs-demo-core")
            .join("src")
            .join("extra.rs"),
        "pub fn x() {}\n",
    )
    .unwrap();
    let bp = dir.join("scaffold.toml");
    let exception = "\n[[exception]]\nwhat = \"w\"\nwhy = \"y\"\nchecked = [\"x\"]\nalternatives = []\ntalk = \"conventions/talks/none.md\"\npaths = [\"crates/fs-demo-core/src/extra.rs\"]\n";
    let with = format!("{}{exception}", std::fs::read_to_string(&bp).unwrap());
    std::fs::write(&bp, with).unwrap();
    let (code, out) = check(&dir);
    assert_eq!(code, 1, "{out}");
    // the drift of the generated file shows although the talk record is missing; the scaffold.toml of the repo is no longer the one the files were made from
    assert!(
        out.contains("generated-changed{path=crates/fs-demo-bin/src/main.rs}"),
        "{out}"
    );
    assert!(
        out.contains("talk-not-found{path=conventions/talks/none.md}"),
        "{out}"
    );
    assert!(
        !out.contains("file-unowned"),
        "the hand-written file is covered by the exception: {out}"
    );
}
