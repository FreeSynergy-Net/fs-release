//! The command `fs-blueprint check`: findings as codes and sentences, exit codes, the options (S4).
mod support;
use fs_blueprint::cli::run;
use std::path::{Path, PathBuf};
use std::process::Command;

fn fixture(name: &str) -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("tests/data/repos")
        .join(name)
}

fn cli(words: &[&str]) -> (i32, String, String) {
    let words: Vec<String> = words.iter().map(|w| (*w).to_string()).collect();
    let (mut out, mut err) = (Vec::new(), Vec::new());
    let code = run(&words, &mut out, &mut err);
    (
        code,
        String::from_utf8(out).unwrap(),
        String::from_utf8(err).unwrap(),
    )
}

fn repo(name: &str) -> String {
    fixture(name).to_string_lossy().into_owned()
}

#[test]
fn a_good_repo_is_ok_with_exit_code_0_and_only_the_pending_skeleton_parts_as_warnings() {
    let (code, out, err) = cli(&["check", "--repo", &repo("ok")]);
    assert_eq!((code, err.as_str()), (0, ""), "{out}");
    assert!(out.contains("0 errors, 3 warnings"), "{out}");
    assert!(
        out.contains("warning: skeleton-part-pending{part=session}"),
        "{out}"
    );
}

#[test]
fn a_bad_repo_gives_every_finding_with_its_code_and_a_sentence_and_exit_code_1() {
    let (code, out, _) = cli(&["check", "--repo", &repo("bad")]);
    assert_eq!(code, 1);
    for want in [
        "error: module-unknown{name=fs-ghost}  The module fs-ghost is not in the catalog",
        "error: layout-outside-forms{path=layout/demo.toml}  The forms description layout/demo.toml must lie in the folder forms/",
        "error: layout-not-found{path=layout/demo.toml}",
        "error: path-not-found{path=crates/fs-demo-core/src/own.rs}",
        "error: talk-not-found{path=conventions/talks/2026-11-18-demo.md}",
    ] {
        assert!(out.contains(want), "{want}\n{out}");
    }
    assert!(out.contains("5 errors"), "{out}");
}

#[test]
fn the_sentences_come_in_german_on_request_and_the_codes_stay() {
    let (_, out, _) = cli(&["check", "--repo", &repo("bad"), "--lang", "de"]);
    assert!(
        out.contains(
            "error: module-unknown{name=fs-ghost}  Das Modul fs-ghost steht nicht im Katalog"
        ),
        "{out}"
    );
    assert!(
        out.contains("muss im Ordner forms/ im Wurzelordner des Repos liegen"),
        "{out}"
    );
}

#[test]
fn a_documentation_checkout_with_the_talk_record_removes_talk_not_found() {
    let docs =
        Path::new(env!("CARGO_TARGET_TMPDIR")).join(format!("cli-docs-{}", std::process::id()));
    let talk = docs.join("conventions").join("talks");
    std::fs::create_dir_all(&talk).unwrap();
    std::fs::write(talk.join("2026-11-18-demo.md"), "x").unwrap();
    let (_, out, _) = cli(&[
        "check",
        "--repo",
        &repo("bad"),
        "--talks",
        &docs.to_string_lossy(),
    ]);
    assert!(
        !out.contains("talk-not-found") && out.contains("4 errors"),
        "{out}"
    );
}

#[test]
fn a_catalog_file_changes_what_exists() {
    let c =
        Path::new(env!("CARGO_TARGET_TMPDIR")).join(format!("cli-cat-{}.toml", std::process::id()));
    std::fs::write(
        &c,
        "[[module]]\nname = \"fs-ghost\"\nkind = \"standard\"\ntags = [\"0.0.1\"]\n",
    )
    .unwrap();
    let (_, out, _) = cli(&[
        "check",
        "--repo",
        &repo("bad"),
        "--catalog",
        &c.to_string_lossy(),
    ]);
    assert!(!out.contains("module-unknown"), "{out}");
    assert!(
        out.contains("skeleton-unknown"),
        "the custom catalog knows no skeleton: {out}"
    );
}

#[test]
fn the_drift_option_adds_the_generated_files_that_are_missing_in_the_repo() {
    let (code, out, _) = cli(&["check", "--repo", &repo("ok"), "--drift"]);
    assert_eq!(code, 1);
    assert!(
        out.contains("error: generated-missing{path=Cargo.toml}"),
        "{out}"
    );
    assert!(
        out.contains("error: seed-missing{path=crates/fs-demo-core/Cargo.toml}"),
        "{out}"
    );
}

#[test]
fn a_blueprint_that_does_not_load_is_one_error_with_its_code() {
    let root =
        Path::new(env!("CARGO_TARGET_TMPDIR")).join(format!("cli-broken-{}", std::process::id()));
    std::fs::create_dir_all(&root).unwrap();
    std::fs::write(root.join("scaffold.toml"), "schema = 1\n").unwrap();
    let (code, out, _) = cli(&["check", "--repo", &root.to_string_lossy()]);
    assert_eq!(code, 1);
    assert!(
        out.starts_with("error: schema-unsupported{found=1, known=2}"),
        "{out}"
    );
}

#[test]
fn wrong_words_and_unreadable_files_end_with_2_and_a_message_on_stderr() {
    for words in [
        vec![],
        vec!["nonsense"],
        vec!["check", "--nonsense"],
        vec!["check", "--lang", "fr"],
        vec!["check", "--rules", "none"],
        vec!["check", "--repo"],
        vec!["check", "--repo", "/nonexistent/for/sure"],
        vec![
            "check",
            "--catalog",
            "/nonexistent/catalog.toml",
            "--repo",
            "x",
        ],
        vec!["--version", "x"],
    ] {
        let (code, out, err) = cli(&words);
        assert_eq!(code, 2, "{words:?}: {out}{err}");
        assert!(err.contains("fs-blueprint:") && out.is_empty(), "{words:?}");
    }
}

#[test]
fn version_and_help_say_themselves() {
    assert_eq!(
        cli(&["--version"]).1.trim(),
        format!("fs-blueprint {}", env!("CARGO_PKG_VERSION"))
    );
    let (code, out, _) = cli(&["--help"]);
    assert!(code == 0 && out.contains("fs-blueprint check") && out.contains("--talks"));
}

#[test]
fn the_real_binary_gives_the_same_answers_and_exit_codes() {
    let exe = env!("CARGO_BIN_EXE_fs-blueprint");
    let ok = Command::new(exe)
        .args(["check", "--repo"])
        .arg(fixture("ok"))
        .output()
        .unwrap();
    assert_eq!(ok.status.code(), Some(0));
    let bad = Command::new(exe)
        .args(["check", "--repo"])
        .arg(fixture("bad"))
        .output()
        .unwrap();
    assert_eq!(bad.status.code(), Some(1));
    assert!(String::from_utf8_lossy(&bad.stdout).contains("layout-outside-forms"));
    let wrong = Command::new(exe).arg("nonsense").output().unwrap();
    assert_eq!(wrong.status.code(), Some(2));
}

#[test]
fn an_exception_over_a_file_the_generator_makes_is_found_on_disk_and_so_is_an_entry_point() {
    let root =
        Path::new(env!("CARGO_TARGET_TMPDIR")).join(format!("cli-gen-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&root);
    std::fs::create_dir_all(root.join("forms")).unwrap();
    std::fs::create_dir_all(root.join("crates").join("fs-demo-bin").join("src")).unwrap();
    std::fs::write(root.join("forms").join("demo.toml"), "#\n").unwrap();
    std::fs::write(root.join(".gitattributes"), "* text=auto\n").unwrap();
    std::fs::write(
        root.join("crates")
            .join("fs-demo-bin")
            .join("src")
            .join("main.rs"),
        "fn main() {}\n",
    )
    .unwrap();
    let ok = std::fs::read_to_string(fixture("ok").join("scaffold.toml")).unwrap();
    let exception = "\n[[exception]]\nwhat = \"w\"\nwhy = \"y\"\nchecked = [\"x\"]\nalternatives = []\ntalk = \"t.md\"\n";
    for (paths, want) in [
        (
            ".gitattributes",
            "error: path-generated{path=.gitattributes}",
        ),
        (
            "crates/fs-demo-bin/src/main.rs",
            "error: main-reserved{path=crates/fs-demo-bin/src/main.rs}",
        ),
    ] {
        std::fs::write(
            root.join("scaffold.toml"),
            format!("{ok}{exception}paths = [\"{paths}\"]\n"),
        )
        .unwrap();
        let (code, out, _) = cli(&["check", "--repo", &root.to_string_lossy()]);
        assert_eq!(code, 1);
        assert!(out.contains(want), "{want}\n{out}");
    }
}
