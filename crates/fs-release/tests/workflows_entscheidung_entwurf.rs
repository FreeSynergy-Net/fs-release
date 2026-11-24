//! Pattern: none (test). The step `gegenprobe` of `entscheidung` (a reader checks the draft `entwurf` against the real code): its place
//! (after `entwurf`, before `ask` and `record`), its gate and kind, its `if` and its model hint, checked on the real text and on mutants.
//! The real file is green; every mutant (a removed step, a changed `if`, a changed gate, a moved step) is red.

mod support;
use support::workflows::{in_order, read, step_body, step_names};

fn has(t: &str, step: &str) -> bool {
    step_names(t).iter().any(|n| n == step)
}

fn ok(t: &str) -> bool {
    has(t, "gegenprobe")
        && has(t, "entwurf")
        && has(t, "ask")
        && has(t, "record")
        && in_order(
            t,
            &["recherche", "cut", "entwurf", "gegenprobe", "ask", "record"],
        )
        && step_body(t, "gegenprobe").contains("call: gegenprobe-ai:0.0.18@fs")
        && step_body(t, "gegenprobe").contains("if: ${ $context.entwurf != null }")
        && step_body(t, "gegenprobe").contains("actor: ai")
        && step_body(t, "gegenprobe").contains("kind: objective")
        && step_body(t, "gegenprobe").contains("gate: none")
        && step_body(t, "gegenprobe").contains("Opus")
        && step_body(t, "entwurf").contains("gate: strategie-ok")
        && !step_body(t, "ask").contains("gegenprobe")
}

/// The text without the `gegenprobe` block, and the block itself.
fn cut_block(t: &str) -> (String, String) {
    let start = t.find("  - gegenprobe:\n").expect("no gegenprobe block");
    let end = t.find("  - ask:\n").expect("no ask step");
    (
        format!("{}{}", &t[..start], &t[end..]),
        t[start..end].to_string(),
    )
}

#[test]
fn the_real_gegenprobe_is_connected_and_green() {
    assert!(ok(&read("entscheidung")));
}

#[test]
fn a_removed_gegenprobe_is_red() {
    let (rest, _) = cut_block(&read("entscheidung"));
    assert!(!ok(&rest));
}

#[test]
fn a_gegenprobe_without_the_entwurf_condition_is_red() {
    let t = read("entscheidung").replace("${ $context.entwurf != null }", "${ true }");
    assert!(!ok(&t));
}

#[test]
fn a_gegenprobe_with_a_human_gate_is_red() {
    let t = read("entscheidung").replace(
        "entscheidung.gegenprobe\n        freesynergy.net:gate: none",
        "entscheidung.gegenprobe\n        freesynergy.net:gate: human",
    );
    assert!(!ok(&t));
}

#[test]
fn a_gegenprobe_before_the_draft_is_red() {
    let (rest, block) = cut_block(&read("entscheidung"));
    let t = rest.replacen("  - entwurf:\n", &format!("{block}  - entwurf:\n"), 1);
    assert!(!ok(&t));
}
