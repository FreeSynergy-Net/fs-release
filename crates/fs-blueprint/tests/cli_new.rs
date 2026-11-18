//! The command `fs-blueprint new`: check, generate, write; nothing is written on an error; nothing is overwritten (S4).
mod support;
use fs_blueprint::cli::run;
use std::path::{Path, PathBuf};

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

fn scratch(name: &str) -> PathBuf {
    let d =
        Path::new(env!("CARGO_TARGET_TMPDIR")).join(format!("cli-{name}-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&d);
    d
}

fn files(root: &Path) -> Vec<String> {
    let mut out = Vec::new();
    let mut todo = vec![root.to_path_buf()];
    while let Some(d) = todo.pop() {
        for e in std::fs::read_dir(&d).unwrap() {
            let p = e.unwrap().path();
            if p.is_dir() {
                todo.push(p);
            } else {
                out.push(
                    p.strip_prefix(root)
                        .unwrap()
                        .to_string_lossy()
                        .replace('\\', "/"),
                );
            }
        }
    }
    out.sort();
    out
}

#[test]
fn new_checks_generates_and_writes_the_program_with_the_blueprint_and_its_forms_folder() {
    let out_dir = scratch("new-ok");
    let bp = fixture("ok").join("scaffold.toml");
    let (code, out, err) = cli(&[
        "new",
        &bp.to_string_lossy(),
        "--out",
        &out_dir.to_string_lossy(),
    ]);
    assert_eq!((code, err.as_str()), (0, ""), "{out}");
    assert!(
        out.contains("made demo (written 14, kept 0)") && out.contains("cargo build"),
        "{out}"
    );
    let got = files(&out_dir);
    assert!(
        got.contains(&"scaffold.toml".to_string())
            && got.contains(&"forms/demo.toml".to_string())
            && got.contains(&"crates/fs-demo-bin/src/main.rs".to_string()),
        "{got:?}"
    );
    assert_eq!(got.len(), 16);
    // the second run changes nothing and says so; the repo is the output of the blueprint
    let (code, out, _) = cli(&[
        "new",
        &bp.to_string_lossy(),
        "--out",
        &out_dir.to_string_lossy(),
    ]);
    assert_eq!(code, 0, "{out}");
    assert!(out.contains("written 12, kept 2"), "{out}");
    let (code, out, _) = cli(&["check", "--repo", &out_dir.to_string_lossy(), "--drift"]);
    assert_eq!(code, 0, "{out}");
}

#[test]
fn new_writes_nothing_when_the_check_finds_an_error_and_says_why_in_both_languages() {
    for (lang, sentence) in [
        ("en", "The module fs-ghost is not in the catalog"),
        ("de", "Das Modul fs-ghost steht nicht im Katalog"),
    ] {
        let out_dir = scratch(&format!("new-bad-{lang}"));
        let bp = fixture("bad").join("scaffold.toml");
        let (code, out, err) = cli(&[
            "new",
            &bp.to_string_lossy(),
            "--out",
            &out_dir.to_string_lossy(),
            "--lang",
            lang,
        ]);
        assert_eq!((code, err.as_str()), (1, ""));
        assert!(
            out.contains("error: module-unknown{name=fs-ghost}") && out.contains(sentence),
            "{out}"
        );
        assert!(!out_dir.exists(), "nothing was written");
    }
}

#[test]
fn new_never_overwrites_a_different_blueprint_in_the_target_and_the_default_target_is_fs_name() {
    let out_dir = scratch("new-exists");
    std::fs::create_dir_all(&out_dir).unwrap();
    std::fs::write(out_dir.join("scaffold.toml"), "schema = 2\n# other\n").unwrap();
    let bp = fixture("ok").join("scaffold.toml");
    let (code, _, err) = cli(&[
        "new",
        &bp.to_string_lossy(),
        "--out",
        &out_dir.to_string_lossy(),
    ]);
    assert_eq!(code, 2);
    assert!(err.contains("exists and is not the same"), "{err}");
    assert_eq!(
        std::fs::read_to_string(out_dir.join("scaffold.toml")).unwrap(),
        "schema = 2\n# other\n"
    );
}

#[test]
fn new_in_the_folder_of_the_blueprint_adds_only_the_generated_files() {
    let work = scratch("new-inplace");
    std::fs::create_dir_all(work.join("forms")).unwrap();
    for f in ["scaffold.toml", "forms/demo.toml"] {
        std::fs::copy(fixture("ok").join(f), work.join(f)).unwrap();
    }
    let bp = work.join("scaffold.toml");
    let (code, out, _) = cli(&[
        "new",
        &bp.to_string_lossy(),
        "--out",
        &work.to_string_lossy(),
    ]);
    assert_eq!(code, 0, "{out}");
    assert_eq!(files(&work).len(), 16);
}

#[test]
fn new_needs_exactly_one_blueprint_and_a_readable_one() {
    for words in [
        vec!["new"],
        vec!["new", "a", "b"],
        vec!["new", "/nonexistent/scaffold.toml"],
        vec!["check", "stray"],
    ] {
        let (code, out, err) = cli(&words);
        assert_eq!(code, 2, "{words:?}");
        assert!(
            err.contains("fs-blueprint:") && out.is_empty(),
            "{words:?}: {err}"
        );
    }
}

#[test]
fn new_without_out_makes_the_folder_fs_name_in_the_current_folder() {
    let work = scratch("new-default");
    std::fs::create_dir_all(&work).unwrap();
    let out = std::process::Command::new(env!("CARGO_BIN_EXE_fs-blueprint"))
        .arg("new")
        .arg(fixture("ok").join("scaffold.toml"))
        .current_dir(&work)
        .output()
        .unwrap();
    assert!(
        out.status.success(),
        "{}",
        String::from_utf8_lossy(&out.stdout)
    );
    assert!(
        work.join("fs-demo").join("Cargo.toml").is_file()
            && work.join("fs-demo").join("scaffold.toml").is_file()
    );
}

#[test]
fn a_stray_word_after_check_is_refused_also_with_a_good_repo() {
    let (code, out, err) = cli(&["check", "stray", "--repo", &fixture("ok").to_string_lossy()]);
    assert_eq!(code, 2, "{out}");
    assert!(err.contains("unknown word stray") && out.is_empty());
}
