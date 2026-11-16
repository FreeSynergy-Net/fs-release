//! Pattern: none (test). The flow `stil-neu` (new style in fs-components-styles): order, gates, the dossier gate first, the Rust tests as stand-in for the
//! missing scripts, the review and the picture OK bound to the head, the release only through the strategy. Each rule has a counter-test on a changed copy.

mod support;
use support::workflows::{in_order, read, step_body};

/// A missing result must never reach `continue`: the last (unconditional) case of the switch goes to `eskalation`, and no unconditional case continues.
fn ends_in_escalation(body: &str) -> bool {
    let switch = body.split("metadata:").next().unwrap();
    let cases: Vec<&str> = switch.split("\n        - ").skip(1).collect();
    let unconditional = |c: &&str| !c.contains("when:");
    cases
        .last()
        .is_some_and(|c| unconditional(c) && c.contains("then: eskalation"))
        && !cases
            .iter()
            .any(|c| unconditional(c) && c.contains("then: continue"))
}

fn style_flow_is_sound(text: &str) -> bool {
    let order = [
        "dossier",
        "dossier-gate",
        "dossier-again",
        "vertrag",
        "tokens",
        "bausteine",
        "vollstaendigkeit",
        "kontrast",
        "abstand",
        "waechter",
        "checks-again",
        "bilder",
        "bilder-again",
        "head",
        "review",
        "review-again",
        "bild-ok",
        "bild-ok-again",
        "freigabe",
        "release",
        "eskalation",
    ];
    in_order(text, &order)
        && step_body(text, "dossier-gate").contains("call: dossier-gate:")
        && step_body(text, "dossier-gate").contains("ohne Vorbild")
        && step_body(text, "dossier-again").contains("then: eskalation")
        && step_body(text, "vertrag").contains("THEME-CONTRACT.md")
        && step_body(text, "vertrag").contains("16 block_tokens")
        && step_body(text, "vollstaendigkeit").contains("--test block_tokens")
        && step_body(text, "vollstaendigkeit").contains("TODO")
        && step_body(text, "abstand").contains("--test blocks_styles")
        && step_body(text, "abstand").contains("r: 0.25")
        && step_body(text, "kontrast").contains("4.5:1")
        && step_body(text, "kontrast").contains("3:1")
        && step_body(text, "checks-again").contains(">= 3")
        && step_body(text, "checks-again").contains("then: eskalation")
        && step_body(text, "checks-again").contains("then: tokens")
        && step_body(text, "bilder").contains("byte-identical")
        && step_body(text, "bilder").contains("call: picture-gate:")
        && step_body(text, "bilder-again").contains("then: tokens")
        && step_body(text, "bilder-again").contains("then: eskalation")
        && ends_in_escalation(&step_body(text, "bild-ok-again"))
        && step_body(text, "bild-ok-again").contains("then: tokens")
        && step_body(text, "bild-ok-again").contains("| length) == 0")
        && ends_in_escalation(&step_body(text, "review-again"))
        && step_body(text, "review-again").contains("then: tokens")
        && step_body(text, "review").contains("gate: opus-ja")
        && step_body(text, "review").contains("head: ${ $context.kopf.head }")
        && step_body(text, "bild-ok").contains("actor: human")
        && step_body(text, "bild-ok").contains("gate: human")
        && step_body(text, "freigabe").contains("gate: strategie-ok")
        && step_body(text, "freigabe").contains("subject: ${ $context.kopf.head }")
        && step_body(text, "release").contains("actor: human")
        && step_body(text, "release").contains("gate: strategie-ok")
}

#[test]
fn stil_neu_runs_the_gates_before_review_picture_ok_and_release() {
    assert!(style_flow_is_sound(&read("stil-neu")));
}

#[test]
fn counter_a_flow_without_dossier_gate_or_with_swapped_order_is_caught() {
    let t = read("stil-neu");
    assert!(!style_flow_is_sound(
        &t.replace("call: dossier-gate:", "call: other:")
    ));
    let swapped = t
        .replacen("  - review:", "  - zz:", 1)
        .replacen("  - bild-ok:", "  - review:", 1)
        .replacen("  - zz:", "  - bild-ok:", 1);
    assert!(!style_flow_is_sound(&swapped));
}

#[test]
fn counter_a_flow_with_missing_stand_in_tests_or_distance_or_rounds_is_caught() {
    let t = read("stil-neu");
    assert!(!style_flow_is_sound(
        &t.replace("--test block_tokens", "--test none")
    ));
    assert!(!style_flow_is_sound(
        &t.replace("--test blocks_styles", "--test none")
    ));
    assert!(!style_flow_is_sound(&t.replace("r: 0.25", "r: 0.0")));
    assert!(!style_flow_is_sound(&t.replace("4.5:1", "3:1 only")));
    assert!(!style_flow_is_sound(
        &t.replace("16 block_tokens", "block tokens")
    ));
    let checks = step_body(&t, "checks-again");
    assert!(!style_flow_is_sound(&t.replacen(
        &checks,
        &checks.replace(">= 3", ">= 9"),
        1
    )));
}

#[test]
fn counter_a_release_without_the_strategy_gate_or_by_an_agent_is_caught() {
    let t = read("stil-neu");
    assert!(!style_flow_is_sound(
        &t.replace("gate: strategie-ok", "gate: none")
    ));
    let release = step_body(&t, "release");
    let by_ai = release.replace("actor: human", "actor: ai");
    assert!(!style_flow_is_sound(&t.replacen(&release, &by_ai, 1)));
    assert!(!style_flow_is_sound(
        &t.replace("subject: ${ $context.kopf.head }", "")
    ));
    assert!(!style_flow_is_sound(
        &t.replace("gate: opus-ja", "gate: none")
    ));
    let ok = step_body(&t, "bild-ok");
    assert!(!style_flow_is_sound(&t.replacen(
        &ok,
        &ok.replace("gate: human", "gate: none"),
        1
    )));
}

#[test]
fn counter_the_loops_back_and_the_picture_gate_are_guarded() {
    let t = read("stil-neu");
    assert!(ends_in_escalation(&step_body(&t, "bild-ok-again")));
    for step in ["bild-ok-again", "bilder-again"] {
        let body = step_body(&t, step);
        let broken = body.replace("then: tokens", "then: continue");
        assert!(
            !style_flow_is_sound(&t.replacen(&body, &broken, 1)),
            "{step}"
        );
    }
    assert!(!style_flow_is_sound(
        &t.replace("call: picture-gate:", "call: other-gate:")
    ));
}

#[test]
fn counter_a_missing_picture_ok_or_verdict_never_reaches_the_release() {
    let t = read("stil-neu");
    let ok = step_body(&t, "bild-ok-again");
    // the old form: the default `angenommen` continues without a result
    let old = ok.replace(
        "        - ohne-ergebnis:\n            then: eskalation\n",
        "",
    );
    let old = old.replace(
        "            when: '${ $context.abnahme.befunde != null and ($context.abnahme.befunde | length) == 0 }'\n",
        "",
    );
    assert_ne!(old, ok);
    assert!(!ends_in_escalation(&old));
    assert!(!style_flow_is_sound(&t.replacen(&ok, &old, 1)));
    let rv = step_body(&t, "review-again");
    assert!(!ends_in_escalation(&rv.replace(
        "        - ohne-urteil:\n            then: eskalation\n",
        ""
    )));
}
