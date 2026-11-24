//! Pattern: none (test). The flow `form-neu` (new or extended form in `forms.txt`): order of the steps, existing first, the reader check with the mutant, the version
//! only NAMED (no tag), the Opus review per bundle, the notation `name@x.y.z`, and the copy block rule. Each rule has a counter-test on a changed copy.

mod support;
use support::workflows::{in_order, read, step_body};

fn form_flow_is_sound(text: &str) -> bool {
    let order = [
        "vorhanden-pruefen",
        "vorhanden-pruefen-check",
        "vorhanden-pruefen-again",
        "vorhanden-weiche",
        "entwurf-lesen",
        "forms-txt-aendern",
        "forms-txt-check",
        "forms-txt-again",
        "version-und-changelog",
        "gegenstueck",
        "opus-buendel-review",
        "opus-buendel-review-again",
        "bericht",
        "abbruch",
        "eskalation",
    ];
    in_order(text, &order)
        && step_body(text, "vorhanden-pruefen").contains("origin/main")
        && step_body(text, "vorhanden-pruefen-again").contains("then: eskalation")
        && step_body(text, "vorhanden-pruefen").contains("erweitern (bool")
        && step_body(text, "vorhanden-weiche").contains("$context.befund.erweitern")
        && step_body(text, "vorhanden-weiche")
            .contains("ohne-ergebnis:\n            then: eskalation")
        && step_body(text, "forms-txt-aendern").contains("THREE-part")
        && step_body(text, "forms-txt-check").contains("`@1.0`")
        && step_body(text, "forms-txt-check").contains("MUTANT")
        && step_body(text, "forms-txt-again").contains(">= 3")
        && step_body(text, "version-und-changelog").contains("does NOT tag")
        && step_body(text, "version-und-changelog").contains("angehoben: false")
        && step_body(text, "gegenstueck").contains("forms =")
        && step_body(text, "opus-buendel-review").contains("gate: opus-ja")
        && step_body(text, "opus-buendel-review-again").contains("then: eskalation")
        && text.contains("[KOPIERBLOCK-OFFENE-SHELL]")
}

#[test]
fn form_neu_checks_existing_first_the_reader_and_names_the_version_without_a_tag() {
    assert!(form_flow_is_sound(&read("form-neu")));
}

#[test]
fn form_neu_is_red_without_each_rule() {
    for (needle, what, edit) in [
        (
            "eskalation",
            "a missing result may go on",
            ("then: eskalation", "then: continue"),
        ),
        (
            "MUTANT",
            "no mutant in the reader check",
            ("MUTANT", "NOTE"),
        ),
        ("does NOT tag", "the flow may tag", ("does NOT tag", "tags")),
        (
            "gate: opus-ja",
            "no Opus review",
            ("gate: opus-ja", "gate: none"),
        ),
        (
            "erweitern",
            "the result of vorhanden-pruefen lacks the field the switch reads",
            ("erweitern (bool", "erweitert (bool"),
        ),
        (
            "ohne-ergebnis",
            "a missing result of the switch may go on",
            (
                "ohne-ergebnis:\n            then: eskalation\n      metadata:\n        freesynergy.net:actor: machine\n        freesynergy.net:actor-type: Service\n        freesynergy.net:gate: none\n        freesynergy.net:kind: objective\n        freesynergy.net:summary: \"Control flow - `vorhanden` without",
                "ohne-ergebnis:\n            then: continue\n      metadata:\n        freesynergy.net:actor: machine\n        freesynergy.net:actor-type: Service\n        freesynergy.net:gate: none\n        freesynergy.net:kind: objective\n        freesynergy.net:summary: \"Control flow - `vorhanden` without",
            ),
        ),
        (
            "[KOPIERBLOCK-OFFENE-SHELL]",
            "no copy block rule",
            ("[KOPIERBLOCK-OFFENE-SHELL]", "[X]"),
        ),
    ] {
        let t = read("form-neu").replace(edit.0, edit.1);
        assert!(!form_flow_is_sound(&t), "{needle}: {what}");
    }
}

#[test]
fn copy_block_rule_stands_in_the_flows_that_write_copy_blocks() {
    let rule = "NO `set -e`, NO `set -u`, NO `set -o pipefail`";
    assert!(read("form-neu").contains(rule));
    assert!(read("upstream-anfrage").contains(rule));
}
