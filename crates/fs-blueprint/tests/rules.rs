//! The set `projekt` and what each rule does (plan scaffold-Kern 3.5): the default blocks everything; a rule in another mode changes the report.
mod support;
use fs_blueprint::{Mode, Rule, Rules};
use support::{edit, run_with, FakeView, SHIFT, TASKS};

#[test]
fn the_set_projekt_blocks_every_rule_and_is_the_only_set() {
    let r = Rules::projekt();
    assert_eq!(r.name(), "projekt");
    for rule in Rule::ALL {
        assert_eq!(r.mode(rule), Mode::Block, "{rule:?}");
    }
    assert_eq!(Rules::by_name("projekt"), Some(Rules::projekt()));
    assert_eq!(Rules::by_name("fremd"), None);
}

#[test]
fn with_changes_exactly_one_rule() {
    let r = Rules::projekt().with(Rule::NameRule, Mode::Off);
    assert_eq!(r.mode(Rule::NameRule), Mode::Off);
    assert_eq!(r.mode(Rule::TalkPerException), Mode::Block);
}

fn report(src: &str, rules: &Rules) -> fs_blueprint::Report {
    run_with(src, FakeView::standard().with_talk(), rules)
}

#[test]
fn the_naming_rule_blocks_in_projekt_and_is_silent_when_off() {
    let src = edit(TASKS, "name = \"tasks\"", "name = \"fs-tasks\"");
    assert!(report(&src, &Rules::projekt()).has("name-rule"));
    assert!(!report(&src, &Rules::projekt().with(Rule::NameRule, Mode::Off)).has("name-rule"));
    let warn = report(&src, &Rules::projekt().with(Rule::NameRule, Mode::Warn));
    assert!(!warn.has("name-rule") && warn.warnings.iter().any(|w| w.code == "name-rule"));
}

#[test]
fn only_existing_blocks_every_unknown_building_block() {
    let src = edit(
        TASKS,
        "name = \"board\"\nkind = \"component\"",
        "name = \"ghost\"\nkind = \"component\"",
    );
    assert!(report(&src, &Rules::projekt()).has("module-unknown"));
    assert!(
        !report(&src, &Rules::projekt().with(Rule::ExistingOnly, Mode::Off)).has("module-unknown")
    );
    let versions = report(TASKS, &Rules::projekt().with(Rule::ExistingOnly, Mode::Off));
    assert!(versions.is_ok(), "{:?}", versions.errors);
}

#[test]
fn the_standard_evidence_rule_covers_every_field() {
    let src = edit(
        SHIFT,
        "standard = \"https://www.rfc-editor.org/rfc/rfc8984#section-5.1\"\n",
        "",
    );
    assert!(report(&src, &Rules::projekt()).has("standard-missing"));
    assert!(!report(
        &src,
        &Rules::projekt().with(Rule::StandardEvidence, Mode::Off)
    )
    .has("standard-missing"));
}

#[test]
fn the_talk_rule_blocks_and_off_it_does_not() {
    let blocked = run_with(TASKS, FakeView::standard(), &Rules::projekt());
    assert!(blocked.has("talk-not-found"));
    assert!(!run_with(
        TASKS,
        FakeView::standard(),
        &Rules::projekt().with(Rule::TalkPerException, Mode::Off)
    )
    .has("talk-not-found"));
}
