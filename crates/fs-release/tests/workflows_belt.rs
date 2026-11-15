//! Pattern: none (test). The flows `programm-grundgeruest`, `testlauf` and the `output` of `release-flow` (bundle 0.0.7): the order, the gates,
//! the fixed trait (locked BEFORE the approval, the approval bound to its hash), the red loops with escalation, and that a missing result is
//! never green. Each rule has a counter-test on a changed copy. The flow `koordinator-runde` is in `workflows_runde.rs`.

mod support;
use support::workflows::{read, step_body, step_names};

fn pos(text: &str, step: &str) -> usize {
    step_names(text)
        .iter()
        .position(|n| n == step)
        .unwrap_or_else(|| panic!("no step {step}"))
}

fn in_order(text: &str, order: &[&str]) -> bool {
    let names = step_names(text);
    order.iter().all(|o| names.iter().any(|n| n == o))
        && order.windows(2).all(|w| pos(text, w[0]) < pos(text, w[1]))
}

fn escalates(text: &str, again: &str, back: &str) -> bool {
    let b = step_body(text, again);
    b.contains("then: eskalation") && b.contains(">= 3") && b.contains(&format!("then: {back}"))
}

/// A switch is strict when its green case needs an explicit `== false` / `== green` and a missing result goes to `eskalation`.
fn strict(text: &str, step: &str, green_when: &str) -> bool {
    let b = step_body(text, step);
    b.contains(green_when) && b.contains("ohne-ergebnis:\n            then: eskalation")
}

fn belt_is_sound(t: &str) -> bool {
    let s1 = step_body(t, "stufe-1");
    let s3 = step_body(t, "stufe-3");
    let ok = step_body(t, "strategie-ok");
    let lock = step_body(t, "trait-sperre");
    let ver = step_body(t, "trait-unveraendert");
    let review = step_body(t, "review");
    in_order(
        t,
        &[
            "stufe-1",
            "stufe-1-check",
            "stufe-1-again",
            "adr-suche",
            "plan",
            "plan-check",
            "plan-again",
            "trait-sperre",
            "strategie-ok",
            "stufe-3",
            "trait-unveraendert",
            "trait-again",
            "gates",
            "gates-again",
            "speicher-check",
            "lauf-check",
            "stufe-3-again",
            "doku",
            "doku-weiter",
            "kopf",
            "review",
            "review-again",
            "bilder",
            "fertig",
        ],
    ) && s1.contains("fork:")
        && s1.contains("dossier-doku")
        && s1.contains("recherche")
        && s1.contains("never code")
        && s3.contains("fork:")
        && s3.contains("kern")
        && s3.contains("frontend")
        && s3.contains("ADR 0086")
        && s3.contains("block assignment")
        && ok.contains("actor: human")
        && ok.contains("gate: strategie-ok")
        && ok.contains("listen:")
        && ok.contains("subject: ${ $context.sperre.sha256 }")
        && step_body(t, "plan").contains("FIXED trait")
        && step_body(t, "plan").contains("gate: opus-ja")
        && lock.contains("read-only")
        && lock.contains("lock_dir")
        && lock.contains("outside every agent worktree")
        && ver.contains("expected: ${ $context.sperre.sha256 }")
        && ver.contains("lock_dir")
        && ver.contains("never green")
        && ver.contains("actor: machine")
        && strict(t, "trait-again", "$context.traitcheck.red == false")
        && step_body(t, "trait-again").contains("then: plan")
        && step_body(t, "gates").contains("name: testlauf")
        && step_body(t, "gates").contains("actor: ai")
        && strict(t, "gates-again", "$context.gates.status == \"green\"")
        && step_body(t, "gates-again").contains("then: stufe-3")
        && strict(
            t,
            "stufe-3-again",
            "$context.speicher.red == false and $context.lauf.red == false",
        )
        && step_body(t, "speicher-check").contains("except `original`")
        && step_body(t, "doku").contains("name: doku-mitziehen")
        && step_body(t, "doku").contains("actor: ai")
        && step_body(t, "doku-weiter").contains("then: eskalation")
        && review.contains("gate: opus-ja")
        && review.contains("head: ${ $context.kopf.head }")
        && review.contains("branch: ${ .input.branch }")
        && review.contains("delta: ${ .input.delta }")
        && !review.contains(".input.head")
        && step_body(t, "review-again").contains("$context.review.verdict != \"JA\"")
        && step_body(t, "review-again").contains("then: stufe-3")
        && step_body(t, "bilder").contains("name: picture-acceptance")
        && t.contains("\noutput:\n")
        && escalates(t, "stufe-1-again", "stufe-1")
        && escalates(t, "plan-again", "plan")
        && escalates(t, "stufe-3-again", "stufe-3")
        && escalates(t, "review-again", "stufe-3")
}

#[test]
fn the_belt_locks_the_trait_before_the_approval_and_binds_the_approval_to_its_hash() {
    assert!(belt_is_sound(&read("programm-grundgeruest")));
}

#[test]
fn counter_a_belt_with_a_late_lock_a_loose_gate_or_a_review_before_the_docs_is_caught() {
    let t = read("programm-grundgeruest");
    let bad = |from: &str, to: &str| {
        assert!(t.contains(from), "{from}");
        assert!(!belt_is_sound(&t.replace(from, to)), "{from}");
    };
    bad("gate: strategie-ok", "gate: none");
    bad("subject: ${ $context.sperre.sha256 }", "subject: plan");
    bad("expected: ${ $context.sperre.sha256 }", "trait: x");
    bad("outside every agent worktree", "inside the worktree");
    bad("head: ${ $context.kopf.head }", "head: ${ .input.head }");
    bad("$context.gates.status == \"green\"", "true");
    bad("$context.traitcheck.red == false", "true");
    bad(
        "$context.speicher.red == false and $context.lauf.red == false",
        "true",
    );
    bad("except `original`", "");
    bad(">= 3", ">= 99");
    bad("name: doku-mitziehen", "name: x");
    let by_ai = |step: &str, from: &str, to: &str| {
        let b = step_body(&t, step);
        t.replacen(&b, &b.replace(from, to), 1)
    };
    assert!(!belt_is_sound(&by_ai(
        "strategie-ok",
        "actor: human",
        "actor: ai"
    )));
    assert!(!belt_is_sound(&by_ai(
        "gates-again",
        "ohne-ergebnis:\n            then: eskalation",
        "ohne-ergebnis:\n            then: speicher-check"
    )));
    assert!(!belt_is_sound(&by_ai(
        "doku-weiter",
        "then: eskalation",
        "then: kopf"
    )));
    assert!(!belt_is_sound(&t.replacen("fork:", "set:", 1)));
    let swap = |a: &str, b: &str| {
        t.replacen(&format!("  - {a}:"), "  - zz:", 1)
            .replacen(&format!("  - {b}:"), &format!("  - {a}:"), 1)
            .replacen("  - zz:", &format!("  - {b}:"), 1)
    };
    assert!(!belt_is_sound(&swap("trait-sperre", "strategie-ok")));
    assert!(!belt_is_sound(&swap("doku", "review")));
    assert!(!belt_is_sound(&swap("strategie-ok", "stufe-3")));
}

fn testlauf_is_sound(t: &str) -> bool {
    if !in_order(t, &["voll", "voll-ende", "zeit"]) {
        return false;
    }
    let full = step_body(t, "voll");
    let end = step_body(t, "voll-ende");
    in_order(
        t,
        &[
            "geaendert",
            "slot",
            "gezielt",
            "gezielt-again",
            "fix",
            "end-weiche",
            "voll",
            "voll-ende",
            "zeit",
            "voll-again",
            "gruen",
            "eskalation",
        ],
    ) && ["geaendert", "slot", "gezielt", "voll", "voll-ende", "zeit"]
        .iter()
        .all(|s| step_body(t, s).contains("actor: machine"))
        && step_body(t, "gezielt").contains("build-slot.sh")
        && step_body(t, "geaendert").contains("-p")
        && step_body(t, "geaendert").contains("--test")
        && full.contains("build-slot.sh")
        && full.contains("BACKGROUND")
        && full.contains("never the whole log")
        && end.contains("listen:")
        && end.contains("type: net.freesynergy.test.done")
        && end.contains("subject: ${ .input.head }")
        && end.contains("export:")
        && step_body(t, "end-weiche").contains("== \"end\"")
        && step_body(t, "slot").contains("at most 3")
        && step_body(t, "fix").contains("then: slot")
        && strict(t, "gezielt-again", "$context.gezielt.red == false")
        && strict(t, "voll-again", "$context.voll.red == false")
        && step_body(t, "gezielt-again").contains("then: end-weiche")
        && step_body(t, "voll-again").contains("then: gruen")
        && step_body(t, "gruen").contains("status: green")
        && step_body(t, "eskalation").contains("eskaliert: true")
        && t.contains("\noutput:\n")
        && t.contains("// \"skipped\"")
        && !t.contains("then: continue")
        && escalates(t, "gezielt-again", "fix")
        && escalates(t, "voll-again", "fix")
}

#[test]
fn the_test_run_is_targeted_waits_for_the_end_event_and_reports_to_the_caller() {
    assert!(testlauf_is_sound(&read("testlauf")));
}

#[test]
fn counter_a_full_run_while_working_a_read_without_the_event_or_a_green_default_is_caught() {
    let t = read("testlauf");
    let bad = |from: &str, to: &str| {
        assert!(t.contains(from), "{from}");
        assert!(!testlauf_is_sound(&t.replace(from, to)), "{from}");
    };
    bad("== \"end\"", "== \"immer\"");
    bad("BACKGROUND", "FOREGROUND");
    bad("never the whole log", "the whole log");
    bad("build-slot.sh", "cargo");
    bad(">= 3", ">= 99");
    bad("--test", "--all");
    bad("net.freesynergy.test.done", "net.freesynergy.other");
    bad("subject: ${ .input.head }", "subject: any");
    bad("$context.voll.red == false", "true");
    bad("$context.gezielt.red == false", "true");
    bad("then: end-weiche", "then: continue");
    bad("\noutput:\n", "\nzz:\n");
    // the wait for the event is gone: `zeit` and `voll-again` would read an empty result
    assert!(!testlauf_is_sound(&t.replacen(
        "  - voll-ende:",
        "  - voll-weg:",
        1
    )));
    let voll = step_body(&t, "voll");
    assert!(!testlauf_is_sound(&t.replacen(
        &voll,
        &voll.replace("actor: machine", "actor: ai"),
        1
    )));
    let again = step_body(&t, "voll-again");
    let no_default = again.replace(
        "        - ohne-ergebnis:\n            then: eskalation\n",
        "        - ohne-ergebnis:\n            then: gruen\n",
    );
    assert!(!testlauf_is_sound(&t.replacen(&again, &no_default, 1)));
    let swapped = t
        .replacen("  - voll-ende:", "  - zz:", 1)
        .replacen("  - zeit:", "  - voll-ende:", 1)
        .replacen("  - zz:", "  - zeit:", 1);
    assert!(!testlauf_is_sound(&swapped));
}

#[test]
fn release_flow_returns_a_result_and_a_docs_escalation_ends_it_with_eskaliert() {
    let t = read("release-flow");
    let sound = |t: &str| {
        t.contains("\noutput:\n")
            && t.contains("$context.ergebnis.eskaliert // false")
            && step_body(t, "doku-weiter").contains("then: eskalation")
            && step_body(t, "eskalation").contains("eskaliert: true")
            && step_body(t, "fertig").contains("status: released")
            && pos(t, "sign") < pos(t, "fertig")
            && pos(t, "fertig") < pos(t, "eskalation")
    };
    assert!(sound(&t));
    assert!(!sound(&t.replace("\noutput:\n", "\nzz:\n")));
    assert!(!sound(&t.replace("then: eskalation", "then: end")));
    assert!(!sound(&t.replace("eskaliert: true", "eskaliert: false")));
}
