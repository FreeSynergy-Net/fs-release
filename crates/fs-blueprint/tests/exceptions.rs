//! `[[exception]]` with `paths` (addendum point 7): every hand-written file belongs to exactly one exception; a talk record is mandatory.
mod support;
use fs_blueprint::Rules;
use support::{edit, run, run_with, FakeView, TASKS};

const PATHS: &str = "paths = [\"crates/fs-tasks-core/src/secret_guard.rs\"]";

fn has(src: &str, view: FakeView, text: &str) -> bool {
    run_with(src, view, &Rules::projekt())
        .errors
        .iter()
        .any(|e| e.to_string() == text)
}

fn talk() -> FakeView {
    FakeView::standard().with_talk()
}

#[test]
fn an_exception_without_paths_is_refused_and_empty_paths_too() {
    assert!(fs_blueprint::Blueprint::from_toml(&edit(TASKS, PATHS, "")).is_err());
    assert!(has(
        &edit(TASKS, PATHS, "paths = []"),
        talk(),
        "field-empty{table=exception, field=paths}"
    ));
}

#[test]
fn paths_with_dots_wildcards_backslashes_or_a_leading_slash_are_invalid() {
    for bad in [
        "../x.rs",
        "a/../x.rs",
        "crates/*.rs",
        "crates/a?.rs",
        "crates\\\\a.rs",
        "/etc/x",
        "./a.rs",
        "a//b.rs",
        "C:/a.rs",
        "",
    ] {
        let src = edit(TASKS, PATHS, &format!("paths = [\"{bad}\"]"));
        assert!(
            run(&src, true)
                .errors
                .iter()
                .any(|e| e.code == "path-invalid"),
            "{bad:?}"
        );
    }
}

#[test]
fn a_path_that_does_not_exist_is_refused() {
    let src = edit(TASKS, PATHS, "paths = [\"crates/none.rs\"]");
    assert!(has(&src, talk(), "path-not-found{path=crates/none.rs}"));
}

#[test]
fn a_path_over_a_generated_file_is_refused_also_through_a_folder() {
    let file = edit(
        TASKS,
        PATHS,
        "paths = [\"crates/fs-tasks-core/src/lib.rs\"]",
    );
    assert!(has(
        &file,
        talk(),
        "path-generated{path=crates/fs-tasks-core/src/lib.rs}"
    ));
    let folder = edit(TASKS, PATHS, "paths = [\"crates/fs-tasks-core/src/\"]");
    assert!(has(
        &folder,
        talk(),
        "path-generated{path=crates/fs-tasks-core/src/}"
    ));
}

fn two(first: &str, second: &str) -> String {
    let more = format!("\n[[exception]]\nwhat = \"a\"\nwhy = \"b\"\nchecked = [\"c\"]\nalternatives = []\ntalk = \"conventions/talks/2026-11-xx-tasks-secret-guard.md\"\npaths = [\"{second}\"]\n");
    edit(TASKS, PATHS, &format!("paths = [\"{first}\"]")) + &more
}

#[test]
fn two_exceptions_over_the_same_file_overlap() {
    let src = two(
        "crates/fs-tasks-core/src/secret_guard.rs",
        "crates/fs-tasks-core/src/secret_guard.rs",
    );
    assert!(has(
        &src,
        talk(),
        "path-overlap{path=crates/fs-tasks-core/src/secret_guard.rs, first=1, second=2}"
    ));
}

#[test]
fn a_folder_and_a_file_inside_it_overlap_in_both_orders() {
    let mut view = talk();
    view.generated.clear();
    let a = two(
        "crates/fs-tasks-core/src/",
        "crates/fs-tasks-core/src/secret_guard.rs",
    );
    assert!(has(
        &a,
        view,
        "path-overlap{path=crates/fs-tasks-core/src/secret_guard.rs, first=1, second=2}"
    ));
    let mut view = talk();
    view.generated.clear();
    let b = two(
        "crates/fs-tasks-core/src/secret_guard.rs",
        "crates/fs-tasks-core/src/",
    );
    assert!(has(
        &b,
        view,
        "path-overlap{path=crates/fs-tasks-core/src/secret_guard.rs, first=1, second=2}"
    ));
}

#[test]
fn a_folder_without_an_end_slash_is_a_file_and_covers_nothing_below() {
    let mut view = talk();
    view.generated.clear();
    let src = two(
        "crates/fs-tasks-core/src",
        "crates/fs-tasks-core/src/secret_guard.rs",
    );
    assert!(!run_with(&src, view, &Rules::projekt()).has("path-overlap"));
}

#[test]
fn a_hand_written_file_in_no_exception_is_unowned_and_one_in_a_folder_exception_is_not() {
    let mut view = talk();
    view.unowned = vec!["crates/fs-tasks-core/src/extra.rs".to_string()];
    assert!(has(
        TASKS,
        view,
        "file-unowned{path=crates/fs-tasks-core/src/extra.rs}"
    ));
    let mut view = talk();
    view.generated.clear();
    view.unowned = vec![
        "crates/fs-tasks-core/src/extra.rs".to_string(),
        "crates/fs-tasks-core/src/secret_guard.rs".to_string(),
    ];
    let src = edit(TASKS, PATHS, "paths = [\"crates/fs-tasks-core/src/\"]");
    assert!(!run_with(&src, view, &Rules::projekt()).has("file-unowned"));
}

#[test]
fn a_file_next_to_a_folder_name_is_not_covered_by_its_prefix() {
    let mut view = talk();
    view.generated.clear();
    view.files
        .insert("crates/fs-tasks-core/src_old/x.rs".to_string());
    view.unowned = vec!["crates/fs-tasks-core/src_old/x.rs".to_string()];
    let src = edit(TASKS, PATHS, "paths = [\"crates/fs-tasks-core/src/\"]");
    assert!(has(
        &src,
        view,
        "file-unowned{path=crates/fs-tasks-core/src_old/x.rs}"
    ));
}

#[test]
fn an_exception_without_a_talk_record_is_refused_and_one_with_it_is_not() {
    assert!(has(
        TASKS,
        FakeView::standard(),
        "talk-not-found{path=conventions/talks/2026-11-xx-tasks-secret-guard.md}"
    ));
    assert!(!run(TASKS, true).has("talk-not-found"));
}

#[test]
fn an_exception_with_nothing_checked_is_refused() {
    let src = edit(
        TASKS,
        "checked = [\"fs-todo\", \"fs-planning\"]",
        "checked = []",
    );
    assert!(has(
        &src,
        talk(),
        "field-empty{table=exception, field=checked}"
    ));
}

// User decision 45: the generated `main` only loads the blueprint and starts; nothing program-specific may move into it.
#[test]
fn no_exception_may_cover_a_main_file_neither_as_file_nor_through_a_folder() {
    let mut view = talk();
    view.generated.clear();
    for path in [
        "crates/fs-tasks-bin/src/main.rs",
        "crates/fs-tasks-bin/src/",
        "crates/fs-tasks-bin/",
    ] {
        let src = edit(TASKS, PATHS, &format!("paths = [\"{path}\"]"));
        let mut v = FakeView {
            files: view.files.clone(),
            ..FakeView::default()
        };
        v.talks = view.talks.clone();
        assert!(
            has(&src, v, &format!("main-reserved{{path={path}}}")),
            "{path}"
        );
    }
}

#[test]
fn an_exception_over_other_files_leaves_main_alone() {
    assert!(!run(TASKS, true).has("main-reserved"));
}

#[test]
fn unowned_files_are_listed_in_sorted_order_whatever_the_view_returns() {
    let mut view = talk();
    view.unowned = vec![
        "crates/z.rs".to_string(),
        "crates/a.rs".to_string(),
        "crates/m.rs".to_string(),
    ];
    let r = run_with(TASKS, view, &Rules::projekt());
    let got: Vec<&str> = r
        .errors
        .iter()
        .filter(|e| e.code == "file-unowned")
        .filter_map(|e| e.param("path"))
        .collect();
    assert_eq!(got, ["crates/a.rs", "crates/m.rs", "crates/z.rs"]);
}
