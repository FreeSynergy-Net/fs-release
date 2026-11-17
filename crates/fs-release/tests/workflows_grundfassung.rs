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

const SPERRE_WHEN: &str =
    "when: ${ .input.art != \"grundfassung\" and $context.vertrag.sperre != true }\n            then: end";
const WEICHE_WHEN: &str = "when: ${ $context.vor_tag.red != false }\n            then: end";
const PFLICHT: &str = "is MANDATORY before the first FEATURE release";

/// The release lock, the pre-tag switch, the mandatory sentence in the head and the green exit of `gates-again`, word for word.
fn lock_lines(text: &str) -> bool {
    step_body(text, "release-sperre").contains(SPERRE_WHEN)
        && step_body(text, "vor-tag-weiche").contains(WEICHE_WHEN)
        && text.contains(PFLICHT)
        && step_body(text, "gates-again").contains("- green:\n            then: continue")
}

#[test]
fn lock_and_pre_tag_lines_are_word_for_word() {
    let t = read("grundfassung");
    assert!(lock_lines(&t));
    for (from, to) in [
        (
            "when: ${ .input.art != \"grundfassung\" and $context.vertrag.sperre != true }",
            "when: ${ $context.vertrag.sperre != true }",
        ),
        (SPERRE_WHEN, "when: ${ false }\n            then: end"),
        (PFLICHT, "is optional"),
        (WEICHE_WHEN, "when: ${ false }\n            then: end"),
        (
            "- green:\n            then: continue",
            "- green:\n            then: release",
        ),
    ] {
        assert!(!lock_lines(&t.replacen(from, to, 1)), "{to}");
    }
}

/// A loop with a counter: from the third round it goes to `grundfassung-eskalation`, which ends.
fn loops_count(text: &str) -> bool {
    let ends = step_body(text, "grundfassung-eskalation").contains("then: end");
    let gates = step_body(text, "gates-again");
    let bild = step_body(text, "bild-weiche");
    ends && gates.contains("- runden:")
        && gates.contains("(($context.gates.runde // 0) >= 3")
        && gates.contains("then: grundfassung-eskalation")
        && bild.contains("$context.verdict.accepted == false and ($context.bild_runde.n // 0) >= 3")
        && bild.contains("then: grundfassung-eskalation")
}

#[test]
fn loops_have_a_round_counter_and_escalate() {
    let t = read("grundfassung");
    assert!(loops_count(&t));
    assert!(!loops_count(
        &t.replace("($context.bild_runde.n // 0) >= 3", "false")
    ));
    assert!(!loops_count(
        &t.replace("(($context.gates.runde // 0) >= 3", "((false")
    ));
    let (head, tail) = t.split_once("  - grundfassung-eskalation:\n").unwrap();
    let open = format!(
        "{head}  - grundfassung-eskalation:\n{}",
        tail.replace("then: end", "then: gates")
    );
    assert!(!loops_count(&open));
}

#[test]
fn no_stale_texts() {
    let t = read("grundfassung");
    assert!(!t.contains("not yet in main"));
    assert!(!t.contains("v0.0.1 WITHOUT"));
    assert!(!t.contains("Same release package"));
    assert!(has(
        &t,
        "frontend-crate",
        &["Own package `fs-<programm>-<frontend>` in the same release (deb/rpm)"]
    ));
    assert!(step_body(&t, "release-sperre").contains("v0.0.x"));
}

/// Every `runde` read in a `runden` branch is set or carried by a step.
fn counters_are_carried(text: &str) -> bool {
    let carried = |step: &str| step_body(text, step).contains("Carries red and runde");
    let gates = ["gates", "binary-test", "lehren-pruefen"]
        .iter()
        .all(|s| carried(s));
    let again = step_body(text, "gates-again");
    let reads = again.contains("$context.gates.runde")
        && again.contains("$context.binary.runde")
        && again.contains("$context.lehren.runde");
    let counts = text.contains("  - bild-zaehlen:\n")
        && step_body(text, "bild-zaehlen").contains("n: ${ ($context.bild_runde.n // 0) + 1 }")
        && step_body(text, "bild-zaehlen").contains("export:")
        && in_order(
            text,
            &[
                "  - bild-start:\n",
                "  - fenster:\n",
                "  - picture-ok:\n",
                "  - bild-zaehlen:\n",
                "  - bild-weiche:\n",
            ],
        );
    gates && reads && counts && !text.contains("$context.verdict.runde")
}

#[test]
fn round_counters_are_really_carried() {
    let t = read("grundfassung");
    assert!(counters_are_carried(&t));
    for step in ["gates", "binary-test", "lehren-pruefen"] {
        let (head, tail) = t.split_once(&format!("  - {step}:\n")).unwrap();
        let weak = format!(
            "{head}  - {step}:\n{}",
            tail.replacen("Carries red and runde", "Carries red", 1)
        );
        assert!(!counters_are_carried(&weak), "{step}");
    }
    assert!(!counters_are_carried(
        &t.replace("($context.bild_runde.n // 0) + 1", "0")
    ));
    assert!(!counters_are_carried(
        &t.replace("  - bild-zaehlen:", "  - bild-zaehlen-aus:")
    ));
}

/// In both loops the branch `runden` stands BEFORE the branch it would otherwise be shadowed by.
fn runden_first(text: &str) -> bool {
    let before = |step: &str, other: &str| {
        let b = step_body(text, step);
        matches!((b.find("- runden:"), b.find(other)), (Some(a), Some(c)) if a < c)
    };
    before("gates-again", "- red:") && before("bild-weiche", "- not-accepted:")
}

#[test]
fn runden_branch_comes_before_red_and_not_accepted() {
    let t = read("grundfassung");
    assert!(runden_first(&t));
    for (step, a, b) in [
        ("gates-again", "- runden:", "- red:"),
        ("bild-weiche", "- runden:", "- not-accepted:"),
    ] {
        let body = step_body(&t, step);
        let i = body.find(a).unwrap();
        let j = body.find(b).unwrap();
        let (x, y) = (body[i..j].to_string(), body[j..].to_string());
        let y_end = y.find("      metadata:").unwrap();
        let swapped = format!("{}{}{}{}{}", &body[..i], &y[..y_end], x, &y[y_end..], "");
        let mutated = t.replacen(&body, &swapped, 1);
        assert!(!runden_first(&mutated), "{step}");
    }
}
