//! The example blueprint of fs-tasks (addendum, field by field): it loads, and only its invented targets are refused.
mod support;
use fs_blueprint::{Blueprint, Mode, Rule, Rules};
use support::{run, run_with, FakeView, SHIFT, TASKS};

#[test]
fn the_example_loads() {
    let bp = Blueprint::from_toml(TASKS).expect("the example of the addendum loads");
    assert_eq!(bp.program.name, "tasks");
    assert_eq!(
        (
            bp.module.len(),
            bp.bundle.len(),
            bp.types.len(),
            bp.ruleset.len(),
            bp.exception.len()
        ),
        (3, 1, 1, 1, 1)
    );
    assert_eq!(
        bp.exception[0].paths,
        ["crates/fs-tasks-core/src/secret_guard.rs"]
    );
}

#[test]
fn the_example_is_refused_with_the_invented_talk_and_the_target_versions_and_nothing_else() {
    let report = run(TASKS, false);
    let got: Vec<String> = report.errors.iter().map(ToString::to_string).collect();
    assert_eq!(
        got,
        [
            "version-unknown{module=fs-todo, version=0.0.1}",
            "version-unknown{module=fs-planning, version=0.0.1}",
            "talk-not-found{path=conventions/talks/2026-11-xx-tasks-secret-guard.md}",
        ]
    );
}

#[test]
fn the_talk_code_is_there_exactly_when_the_talk_record_is_missing() {
    assert!(run(TASKS, false).has("talk-not-found"));
    assert!(!run(TASKS, true).has("talk-not-found"));
}

#[test]
fn missing_skeleton_parts_are_shown_as_warnings_never_left_out() {
    let report = run(TASKS, true);
    let parts: Vec<&str> = report
        .warnings
        .iter()
        .filter(|w| w.code == "skeleton-part-pending")
        .filter_map(|w| w.param("part"))
        .collect();
    assert_eq!(parts, ["session", "update", "mcp-switch"]);
    assert!(report
        .errors
        .iter()
        .all(|e| e.code != "skeleton-part-pending"));
}

#[test]
fn a_rule_in_mode_warn_is_a_warning_and_in_mode_off_nothing() {
    let warn = run_with(
        TASKS,
        FakeView::standard(),
        &Rules::projekt().with(Rule::TalkPerException, Mode::Warn),
    );
    assert!(
        !warn.has("talk-not-found") && warn.warnings.iter().any(|w| w.code == "talk-not-found")
    );
    let off = run_with(
        TASKS,
        FakeView::standard(),
        &Rules::projekt().with(Rule::TalkPerException, Mode::Off),
    );
    assert!(!off.has("talk-not-found") && off.warnings.iter().all(|w| w.code != "talk-not-found"));
}

#[test]
fn a_program_with_its_own_type_checks_clean() {
    let report = run(SHIFT, true);
    assert!(report.is_ok(), "{:?}", report.errors);
}

#[test]
fn the_check_is_deterministic() {
    let first = run(TASKS, false);
    for _ in 0..20 {
        assert_eq!(run(TASKS, false), first);
    }
}
