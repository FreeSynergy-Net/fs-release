//! Pattern: none (test). Structure of `grundfassung.yaml` (rules of 17 Nov.): placeholder rule, version rule, picture verdict by the
//! strategy with the `accepted` gate before the catalog proposal, `was_kommt` from `feature-liste`, release lock by `art`, lessons
//! and pre-tag check. Each rule has a counter-test.

mod support;
use support::workflows::{in_order, read, step_body, step_names};

fn has(text: &str, step: &str, phrases: &[&str]) -> bool {
    let body = step_body(text, step);
    phrases.iter().all(|p| body.contains(p))
}

/// `picture-ok` is done by the strategy (an AI actor, no human gate) and gets `dossier` and `waechter`.
fn picture_by_strategy(text: &str) -> bool {
    let body = step_body(text, "picture-ok");
    body.contains("freesynergy.net:actor: ai")
        && !body.contains("freesynergy.net:actor: human")
        && body.contains("dossier: ${ .input.dossier }")
        && body.contains("waechter: ${ $context.waechter }")
}

/// Only `accepted == true` goes on, and the gate sits between `picture-ok` and `katalog-vorschlag`.
fn accepted_gate(text: &str) -> bool {
    let body = step_body(text, "bild-weiche");
    body.contains("$context.verdict.accepted == true")
        && body.contains("then: bild-eskalation")
        && in_order(text, &["picture-ok", "bild-weiche", "katalog-vorschlag"])
}

#[test]
fn placeholder_rule_is_in_window_and_summary() {
    let t = read("grundfassung");
    for step in ["fenster", "zusammenfassung"] {
        assert!(has(&t, step, &["vorläufig", "Lücken"]), "{step}");
    }
    assert!(step_body(&t, "fenster").contains("nearest EXISTING component"));
    assert!(!t.contains("or placeholder"), "no PNG placeholder");
    let weak = t.replace("nearest EXISTING component", "grey box");
    assert!(!step_body(&weak, "fenster").contains("nearest EXISTING component"));
}

#[test]
fn versions_may_jump_and_version_is_tag() {
    let t = read("grundfassung");
    assert!(t.contains("may currently JUMP") && t.contains("version = tag"));
    assert!(has(
        &t,
        "zusammenfassung",
        &["may currently jump", "version = tag"]
    ));
}

#[test]
fn picture_verdict_is_the_strategys_and_gated_on_accepted() {
    let t = read("grundfassung");
    assert!(picture_by_strategy(&t) && accepted_gate(&t));
    let human = t.replacen(
        "freesynergy.net:actor: ai\n        freesynergy.net:actor-type: Application\n        freesynergy.net:gate: none\n        freesynergy.net:kind: subjective",
        "freesynergy.net:actor: human\n        freesynergy.net:actor-type: Person\n        freesynergy.net:gate: bild-ok\n        freesynergy.net:kind: subjective",
        1,
    );
    assert!(!picture_by_strategy(&human));
    let no_gate = t.replace("then: bild-eskalation", "then: continue");
    assert!(!accepted_gate(&no_gate));
    assert!(step_names(&t).contains(&"bild-eskalation".to_string()));
}

#[test]
fn was_kommt_comes_from_the_feature_list_only() {
    let t = read("grundfassung");
    assert!(!t.contains(".input.was_kommt"), "no own input");
    assert!(t.contains("was_kommt: ${ $context.features.was_kommt }"));
    assert!(in_order(
        &t,
        &["feature-liste", "fenster", "zusammenfassung"]
    ));
}

#[test]
fn release_lock_by_art_and_pre_tag() {
    let t = read("grundfassung");
    assert!(t.contains("release_frei") && t.contains("draft contract"));
    assert!(has(&t, "release-sperre", &["art", "grundfassung"]));
    assert!(in_order(
        &t,
        &["release-sperre", "vor-tag", "vor-tag-weiche", "release"]
    ));
    assert!(has(
        &t,
        "vor-tag",
        &["ALL workflows", "NO tag with an open condition"]
    ));
    assert!(has(&t, "release", &["ff-und-tag", "release_frei"]));
    let weak = t.replace("NO tag with an open condition", "a tag");
    assert!(!has(&weak, "vor-tag", &["NO tag with an open condition"]));
}

#[test]
fn lessons_are_a_mandatory_step_before_the_gates() {
    let t = read("grundfassung");
    assert!(in_order(&t, &["binary-test", "lehren-pruefen", "gates"]));
    let phrases = [
        "CONTRACT_VERSION",
        "CapabilityList",
        "@v0.0.10",
        "env -i",
        "$auto",
        "compare=",
        "ETXTBSY",
        "Windows/macOS follow",
        "message_key()",
        "create_private",
    ];
    assert!(has(&t, "lehren-pruefen", &phrases));
    assert!(step_body(&t, "gates-again").contains("$context.lehren.red"));
    let weak = t.replace("ETXTBSY", "x");
    assert!(!has(&weak, "lehren-pruefen", &phrases));
}
