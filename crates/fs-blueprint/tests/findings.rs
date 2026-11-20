//! Condition B3 (mutant O17): every finding has the parameters of its code and renders in both languages.
mod support;
use fs_blueprint::{messages::render, Coded, Report};
use support::{edit, run, SHIFT, TASKS};

const FIELD_STD: &str = "standard = \"https://www.rfc-editor.org/rfc/rfc8984#section-5.1\"";

/// Broken variants that between them produce many codes; the findings of all of them are checked for form and for both languages.
#[test]
fn b3_every_finding_has_the_parameters_of_its_code_and_renders_in_both_languages() {
    let variants: Vec<String> = vec![
        edit(TASKS, "name = \"task-card\"\nkind = \"component\"", "name = \"task-card\"\nkind = \"toolkit\""),
        edit(TASKS, "name = \"fs-planning\"\nversion", "name = \"fs-ghost\"\nversion"),
        edit(TASKS, "parts = [\"readiness\"", "parts = [\"zzz\", \"readiness\""),
        edit(TASKS, "item = \"Task\"", "item = \"Nope\""),
        edit(TASKS, "default = \"freesynergy\"", "default = \"zzz\""),
        edit(TASKS, "from = \"fs-planning\"", "from = \"board\""),
        edit(TASKS, "layout = \"forms/board.toml\"", "layout = \"nope/x\""),
        edit(TASKS, "covers = [\"Items.list\"", "covers = [\"zz\", \"Items.list\""),
        edit(TASKS, "paths = [\"crates/fs-tasks-core/src/secret_guard.rs\"]", "paths = [\"../x\", \"crates/none.rs\", \"crates/fs-tasks-bin/src/main.rs\"]"),
        edit(TASKS, "skeleton = \"0.1.0\"", "skeleton = \"9.9.9\""),
        edit(TASKS, "capability = \"tasks\"", "capability = \"nope\""),
        edit(TASKS, "summary = { en = \"Tasks, dependencies and boards\", de = \"Aufgaben, Abhängigkeiten und Boards\" }", "summary = { en = \"{$a}\", de = \"{$b}\" }"),
        edit(TASKS, "schema = 2", "schema = 1"),
        edit(TASKS, "schema = 2", "schema = 3"),
        edit(TASKS, "schema = 2", "schema = 2\nzzz = 1"),
        edit(TASKS, "categories = [\"Office\", \"ProjectManagement\"]", "categories = []"),
        edit(SHIFT, "kit = \"timeSpan\"", "kit = \"ghost\"\nwords = \"x\""),
        edit(SHIFT, FIELD_STD, "standard = \"foo\""),
        edit(SHIFT, "envelope = \"Event\"", "envelope = \"\""),
        edit(SHIFT, "name = \"Shift\"", "name = \"shift\""),
        edit(SHIFT, "name = \"schedules\"", "name = \"fs-Sched\""),
        "not toml at all [".to_string(),
        edit(TASKS, "[[module]]\nname = \"board\"", "[[module]]\nname = \"ghost\"\nkind = \"standard\"\nversion = \"0.0.1\"\n\n[[module]]\nname = \"board\""),
    ];
    let mut seen = std::collections::BTreeSet::new();
    for v in &variants {
        let r: Report = run(v, false);
        for f in r.errors.iter().chain(&r.warnings) {
            assert!(f.conforms(), "{f}");
            assert_renders(f);
            seen.insert(f.code);
        }
    }
    for code in [
        "bundle-part-unknown",
        "kind-mismatch",
        "module-unknown",
        "bundle-unknown",
        "type-unknown",
        "ruleset-default-missing",
        "path-invalid",
        "path-not-found",
        "main-reserved",
        "covers-invalid",
        "layout-not-found",
        "skeleton-unknown",
        "capability-unknown",
        "text-vars",
        "schema-unsupported",
        "schema-newer",
        "unknown-key",
        "field-empty",
        "standard-invalid",
        "envelope-invalid",
        "name-case",
        "name-rule",
        "parse-error",
        "talk-not-found",
        "version-unknown",
        "words-unknown",
        "kit-unknown",
    ] {
        assert!(seen.contains(code), "no variant produced {code}: {seen:?}");
    }
}

/// Rule `code-in-program`: a hand-written `.rs` file without an exception is code in the program; other hand-written files and covered Rust are not.
#[test]
fn a_hand_written_rust_file_without_an_exception_is_code_in_the_program() {
    use support::{run_with, FakeView};
    let mut view = FakeView::standard().with_talk();
    view.unowned = vec![
        "crates/fs-tasks-core/src/extra.rs".to_string(),
        "crates/fs-tasks-core/notes.txt".to_string(),
        "crates/fs-tasks-core/src/secret_guard.rs".to_string(),
    ];
    let r = run_with(TASKS, view, &fs_blueprint::Rules::projekt());
    let got: Vec<String> = r
        .errors
        .iter()
        .filter(|e| e.code == "code-in-program")
        .map(ToString::to_string)
        .collect();
    assert_eq!(
        got,
        ["code-in-program{path=crates/fs-tasks-core/src/extra.rs}"]
    );
    for f in r.errors.iter().filter(|e| e.code == "code-in-program") {
        assert!(f.conforms());
        assert_renders(f);
    }
    assert!(!run_with(
        TASKS,
        FakeView::standard().with_talk(),
        &fs_blueprint::Rules::projekt()
    )
    .has("code-in-program"));
}

fn assert_renders(c: &Coded) {
    for lang in ["en", "de"] {
        let text = render(c, lang).unwrap_or_else(|| panic!("no message for {c}"));
        assert!(!text.contains("{$"), "{lang}: {text}");
    }
}
