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
        let main = dir.join(".github").join("workflows").join("check.yml");
        let text = std::fs::read_to_string(&main).unwrap();
        std::fs::write(&main, format!("{text}{change}")).unwrap();
        let (code, out) = check(&dir);
        assert_eq!(code, 1, "{out}");
        assert!(
            out.contains("error: generated-changed{path=.github/workflows/check.yml}"),
            "{out}"
        );
    }
}

#[test]
fn a_generated_file_that_was_deleted_is_red() {
    let dir = made("deleted");
    std::fs::remove_file(dir.join(".gitattributes")).unwrap();
    let (code, out) = check(&dir);
    assert_eq!(code, 1, "{out}");
    assert!(
        out.contains("generated-missing{path=.gitattributes}") && !out.contains("seed-missing"),
        "{out}"
    );
}

#[test]
fn a_file_without_a_head_under_crates_is_red_a_hand_written_manifest_too_but_a_tests_file_is_not() {
    let dir = made("unowned");
    let core_dir = dir.join("crates").join("fs-demo-core");
    std::fs::create_dir_all(core_dir.join("src")).unwrap();
    std::fs::write(core_dir.join("src").join("extra.rs"), "pub fn x() {}\n").unwrap();
    let (code, out) = check(&dir);
    assert_eq!(code, 1, "{out}");
    assert!(
        out.contains("error: file-unowned{path=crates/fs-demo-core/src/extra.rs}")
            && out.contains("error: code-in-program{path=crates/fs-demo-core/src/extra.rs}"),
        "{out}"
    );
    // a hand-written manifest is not generated any more and has no head either: it is red like any file under crates/
    std::fs::remove_file(core_dir.join("src").join("extra.rs")).unwrap();
    std::fs::write(core_dir.join("Cargo.toml"), "# mine\n").unwrap();
    let (code, out) = check(&dir);
    assert_eq!(code, 1, "{out}");
    assert!(
        out.contains("error: file-unowned{path=crates/fs-demo-core/Cargo.toml}"),
        "{out}"
    );
    // only a file in a tests folder: clean again
    std::fs::remove_file(core_dir.join("Cargo.toml")).unwrap();
    std::fs::create_dir_all(core_dir.join("tests")).unwrap();
    std::fs::write(core_dir.join("tests").join("t.rs"), "#[test]\nfn t() {}\n").unwrap();
    let (code, out) = check(&dir);
    assert_eq!(code, 0, "{out}");
}

#[test]
fn the_drift_is_shown_together_with_a_hand_written_file_and_with_a_missing_talk_record() {
    let dir = made("together");
    let main = dir.join(".github").join("workflows").join("check.yml");
    let text = std::fs::read_to_string(&main).unwrap();
    std::fs::write(&main, format!("{text}// hand\n")).unwrap();
    std::fs::create_dir_all(dir.join("crates").join("fs-demo-core").join("src")).unwrap();
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
        out.contains("generated-changed{path=.github/workflows/check.yml}"),
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

#[test]
fn the_drift_is_shown_together_with_a_hand_written_file_that_no_exception_covers() {
    let dir = made("with-unowned");
    let main = dir.join(".github").join("workflows").join("check.yml");
    let text = std::fs::read_to_string(&main).unwrap();
    std::fs::write(&main, format!("{text}// hand\n")).unwrap();
    std::fs::create_dir_all(dir.join("crates").join("fs-demo-core").join("src")).unwrap();
    std::fs::write(
        dir.join("crates")
            .join("fs-demo-core")
            .join("src")
            .join("extra.rs"),
        "pub fn x() {}\n",
    )
    .unwrap();
    let (code, out) = check(&dir);
    assert_eq!(code, 1, "{out}");
    assert!(
        out.contains("generated-changed{path=.github/workflows/check.yml}"),
        "the drift is hidden by the hand-written file: {out}"
    );
    assert!(
        out.contains("file-unowned{path=crates/fs-demo-core/src/extra.rs}"),
        "{out}"
    );
}

#[test]
fn the_drift_is_shown_for_an_own_type_without_a_standard_that_an_exception_with_a_talk_record_excuses(
) {
    // a type without `standard` is allowed only when an exception names it and its talk record exists (decision 6e): the structure gate of the drift needs that record
    let work = Path::new(env!("CARGO_TARGET_TMPDIR"))
        .join(format!("drift-no-standard-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&work);
    let docs = work.join("docs-checkout");
    std::fs::create_dir_all(docs.join("conventions").join("talks")).unwrap();
    std::fs::write(
        docs.join("conventions").join("talks").join("extra.md"),
        "# talk\n",
    )
    .unwrap();
    let repo = work.join("repo");
    std::fs::create_dir_all(repo.join("forms")).unwrap();
    std::fs::create_dir_all(repo.join("crates").join("fs-demo-core").join("src")).unwrap();
    std::fs::copy(
        fixture("ok").join("forms").join("demo.toml"),
        repo.join("forms").join("demo.toml"),
    )
    .unwrap();
    std::fs::write(
        repo.join("crates")
            .join("fs-demo-core")
            .join("src")
            .join("own.rs"),
        "pub fn own() {}\n",
    )
    .unwrap();
    let ok = std::fs::read_to_string(fixture("ok").join("scaffold.toml")).unwrap();
    let extra = "\n[[type]]\nname = \"Extra\"\nenvelope = \"Note\"\nmedia_type = \"application/x-extra\"\nlabel = { en = \"Extra\", de = \"Extra\" }\n\n[[exception]]\nwhat = \"The type Extra has no standard\"\nwhy = \"for the test\"\nchecked = [\"x\"]\nalternatives = []\ntalk = \"conventions/talks/extra.md\"\npaths = [\"crates/fs-demo-core/src/own.rs\"]\n";
    // the exception tables come last in the file: put the type before the frontend table
    let text = ok.replacen(
        "[frontend]",
        &format!(
            "{}\n[frontend]",
            extra.split("[[exception]]").next().unwrap()
        ),
        1,
    ) + "\n[[exception]]"
        + extra.split("[[exception]]").nth(1).unwrap();
    std::fs::write(repo.join("scaffold.toml"), text).unwrap();
    let (repo_s, docs_s) = (
        repo.to_string_lossy().into_owned(),
        docs.to_string_lossy().into_owned(),
    );
    let (code, out) = cli(&[
        "new",
        &repo.join("scaffold.toml").to_string_lossy(),
        "--out",
        &repo_s,
        "--talks",
        &docs_s,
    ]);
    assert_eq!(code, 0, "{out}");
    let (code, out) = cli(&["check", "--repo", &repo_s, "--talks", &docs_s, "--drift"]);
    assert_eq!(code, 0, "clean with the talk record: {out}");
    let main = repo.join(".github").join("workflows").join("check.yml");
    let text = std::fs::read_to_string(&main).unwrap();
    std::fs::write(&main, format!("{text}// hand\n")).unwrap();
    // a hand-written file that no exception names: the structure gate must not count it (it must not hide the drift)
    std::fs::write(
        repo.join("crates")
            .join("fs-demo-core")
            .join("src")
            .join("loose.rs"),
        "pub fn loose() {}\n",
    )
    .unwrap();
    let (code, out) = cli(&["check", "--repo", &repo_s, "--talks", &docs_s, "--drift"]);
    assert_eq!(code, 1, "{out}");
    assert!(
        out.contains("generated-changed{path=.github/workflows/check.yml}"),
        "the drift is hidden by the type without a standard: {out}"
    );
    assert!(
        out.contains("file-unowned{path=crates/fs-demo-core/src/loose.rs}"),
        "the hand-written file is shown next to the drift: {out}"
    );
    assert!(
        !out.contains("talk-not-found") && !out.contains("field-missing"),
        "{out}"
    );
}
