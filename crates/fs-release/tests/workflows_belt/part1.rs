use super::*;

pub(super) fn pos(text: &str, step: &str) -> usize {
    step_names(text)
        .iter()
        .position(|n| n == step)
        .unwrap_or_else(|| panic!("no step {step}"))
}

pub(super) fn in_order(text: &str, order: &[&str]) -> bool {
    let names = step_names(text);
    order.iter().all(|o| names.iter().any(|n| n == o))
        && order.windows(2).all(|w| pos(text, w[0]) < pos(text, w[1]))
}

pub(super) fn escalates(text: &str, again: &str, back: &str) -> bool {
    let b = step_body(text, again);
    b.contains("then: eskalation") && b.contains(">= 3") && b.contains(&format!("then: {back}"))
}

/// A switch is strict when its green case needs an explicit `== false` / `== green` and a missing result goes to `eskalation`.
pub(super) fn strict(text: &str, step: &str, green_when: &str) -> bool {
    let b = step_body(text, step);
    b.contains(green_when) && b.contains("ohne-ergebnis:\n            then: eskalation")
}

/// A3: the hash alone is readable by agents - the sender of the approval is checked at the event.
pub(super) fn sender_is_checked(t: &str) -> bool {
    let ok = step_body(t, "strategie-ok");
    ok.contains("freigabe: .")
        && step_body(t, "absender").contains("$context.freigabe.source")
        && step_body(t, "absender").contains("net.freesynergy.strategie")
        && step_body(t, "absender").contains("net.freesynergy.nutzer")
        && step_body(t, "absender").contains("$context.freigabe.data.actor")
        && step_body(t, "absender").contains("== \"human\"")
        && step_body(t, "absender").contains("$context.freigabe.subject == $context.sperre.sha256")
        && step_body(t, "absender").contains("then: stufe-3")
        && step_body(t, "absender").contains("then: eskalation")
}

/// A missing verdict of the review or of the pictures escalates; only the explicit answer goes on (G6).
pub(super) fn verdicts_are_read(t: &str) -> bool {
    step_body(t, "review-again").contains("$context.review.verdict == \"NEIN\"")
        && step_body(t, "review-again").contains("$context.review.verdict == \"JA\"")
        && step_body(t, "review-again").contains("ohne-ergebnis:\n            then: eskalation")
        && !step_body(t, "review-again").contains("!= \"JA\"")
        && step_body(t, "bilder").contains("ours: ${ $context.images }")
        && step_body(t, "bilder").contains("bild: .")
        && step_body(t, "bilder-erzeugen").contains("render-images")
        && step_body(t, "bilder-again").contains("$context.bild.accepted == true")
        && step_body(t, "bilder-again").contains("then: fertig")
        && step_body(t, "bilder-again").contains("$context.bild.accepted == false")
        && step_body(t, "bilder-again").contains("then: stufe-3")
        && step_body(t, "bilder-again").contains("ohne-urteil:\n            then: eskalation")
        && step_body(t, "review-again").contains("then: stufe-3")
}

pub(super) fn belt_is_sound(t: &str) -> bool {
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
            "absender",
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
            "bilder-erzeugen",
            "bilder",
            "bilder-again",
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
        && sender_is_checked(t)
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
        && verdicts_are_read(t)
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
    // A3: an approval of any sender with the right hash, or without `actor: human`
    bad("$context.freigabe.data.actor", "$context.freigabe.zz");
    bad("net.freesynergy.strategie", "net.freesynergy.agent");
    bad("net.freesynergy.nutzer", "net.freesynergy.agent");
    bad(
        "$context.freigabe.subject == $context.sperre.sha256",
        "true",
    );
    // G6: a picture verdict that is not read
    bad("$context.bild.accepted == true", "true");
    bad("ours: ${ $context.images }", "model: x");
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
    assert!(!belt_is_sound(&by_ai(
        "bilder-again",
        "ohne-urteil:\n            then: eskalation",
        "ohne-urteil:\n            then: fertig"
    )));
    assert!(!belt_is_sound(&by_ai(
        "bilder-again",
        "then: stufe-3",
        "then: fertig"
    )));
    assert!(!belt_is_sound(&by_ai(
        "review-again",
        "ohne-ergebnis:\n            then: eskalation",
        "ohne-ergebnis:\n            then: stufe-3"
    )));
    assert!(!belt_is_sound(&by_ai(
        "absender",
        "        - fremd:\n            then: eskalation",
        "        - fremd:\n            then: stufe-3"
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
    assert!(!belt_is_sound(&swap("bilder", "bilder-again")));
}
