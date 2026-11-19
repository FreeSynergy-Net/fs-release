//! Pattern: none (test). The rule doku-mitziehen: the documentation moves with every change (sub-workflow `doku-mitziehen`, a mandatory step in
//! `release-flow`, `programm-neu`, `upstream-update`, `umbenennen`) and the flow `umbenennen` (dry run without AI, approval before the GitHub
//! rename, name lock). Each rule has a counter-test on a changed copy; the two scripts run their own self tests.

mod support;
use support::workflows::{in_order, pos, read, step_body, step_names};

fn calls_doku_before(text: &str, later: &str) -> bool {
    let body = step_body(text, "doku");
    body.contains("name: doku-mitziehen")
        && body.contains("actor: ai")
        && body.contains("kind: objective")
        && body.contains("doku: .")
        && pos(text, "doku") < pos(text, later)
        && step_body(text, "doku-weiter").contains("$context.doku.eskaliert == true")
        && step_body(text, "doku-weiter").contains("$context.doku.status != \"green\"")
}

/// The docs main branch is pushed by a human step of the caller, only after `after` (the review, the approval, the merge of the code).
fn publishes_after(text: &str, after: &[&str]) -> bool {
    let b = step_body(text, "doku-veroeffentlichen");
    b.contains("name: publish-doc")
        && b.contains("actor: human")
        && b.contains("gate: human")
        && after
            .iter()
            .all(|a| pos(text, a) < pos(text, "doku-veroeffentlichen"))
}

fn gate_is_sound(text: &str) -> bool {
    let gate = step_body(text, "gate");
    let again = step_body(text, "gate-again");
    in_order(
        text,
        &[
            "suchen",
            "schreiben",
            "docs-check",
            "kopf",
            "gate",
            "gate-again",
            "eskalation",
        ],
    ) && gate.contains("actor: machine")
        && gate.contains("kind: objective")
        && gate.contains("doku-mitziehen:0.0.13@fs")
        && gate.contains("docs_base")
        // R8': the gate measures the head of the agent's branch, resolved per step
        && gate.contains("head: ${ $context.kopf.head }")
        && step_body(text, "kopf").contains("resolve-head")
        && step_body(text, "kopf").contains("branch: ${ .input.branch")
        && gate.contains("proof_base: ${ .input.docs_base }")
        && gate.contains("program: ${ .input.programm")
        && gate.contains("todo/checklisten/<programm>.md")
        && step_body(text, "schreiben").contains("never delete a ticked item")
        && again.contains("then: eskalation")
        && again.contains(">= 3")
        && again.contains("then: suchen")
        && step_body(text, "schreiben").contains("actor: ai")
        && !text.contains("name: publish-doc")
        && again.contains("$context.doku.status == \"green\"")
        && again.contains("$context.doku.red == true")
        && again.contains("then: end")
        && again.contains("- skipped:")
        && step_body(text, "eskalation").contains("eskaliert: true")
        && text.contains("\noutput:\n")
}

#[test]
fn the_sub_workflow_has_the_checks_a_red_loop_and_a_human_publish() {
    assert!(gate_is_sound(&read("doku-mitziehen")));
}

#[test]
fn counter_a_gate_by_the_agent_or_without_escalation_is_caught() {
    let t = read("doku-mitziehen");
    let by_ai = t.replacen(
        "call: doku-mitziehen:0.0.13@fs",
        "call: docs-ai:0.0.13@fs",
        1,
    );
    assert!(!gate_is_sound(&by_ai));
    assert!(!gate_is_sound(
        &t.replace("then: eskalation", "then: continue")
    ));
    assert!(!gate_is_sound(&t.replace(">= 3", ">= 99")));
    // R8': no head binding (the gate would measure the checkout of the repo) is caught
    assert!(!gate_is_sound(
        &t.replace("head: ${ $context.kopf.head }", "zz: 1")
    ));
    assert!(!gate_is_sound(&t.replace(
        "call: resolve-head:0.0.13@fs",
        "call: other:0.0.13@fs"
    )));
    assert!(!gate_is_sound(
        &t.replace("proof_base: ${ .input.docs_base }", "zz: 1")
    ));
    // checklist rule: the gate without the program (no checklist check) or without the author's duty is caught
    assert!(!gate_is_sound(&t.replace(
        "        program: ${ .input.programm",
        "        zz: ${ .input.programm"
    )));
    assert!(!gate_is_sound(
        &t.replace("never delete a ticked item", "delete as you like")
    ));
    assert!(!gate_is_sound(&t.replace("then: suchen", "then: continue")));
    // B2: a skipped result (rc 2, no JSON) must not be green - the green case needs `status == "green"`, the default case escalates
    assert!(!gate_is_sound(
        &t.replace("$context.doku.status == \"green\"", "true")
    ));
    assert!(!gate_is_sound(&t.replace("- skipped:", "- weiter:")));
    // B3: the publish step is not part of the sub-workflow
    let with_publish = t.replacen(
        "  - eskalation:",
        "  - veroeffentlichen:\n      run:\n        workflow:\n          name: publish-doc\n  - eskalation:",
        1,
    );
    assert!(!gate_is_sound(&with_publish));
}

#[test]
fn the_mandatory_step_is_in_every_release_workflow_before_the_review() {
    assert!(calls_doku_before(&read("release-flow"), "gates"));
    assert!(calls_doku_before(&read("release-flow"), "review-request"));
    // programm-neu has no own `doku` (it ran twice); the one `release` step runs release-flow with the docs inputs, and that flow has `doku`
    let pn = read("programm-neu");
    assert!(!step_names(&pn).contains(&"doku".to_string()));
    let rel = step_body(&pn, "release");
    assert!(
        rel.contains("name: release-flow") && rel.contains("docs_base:") && rel.contains("allow:")
    );
    assert!(calls_doku_before(&read("upstream-update"), "head"));
    assert!(calls_doku_before(&read("upstream-update"), "review"));
    assert!(read("lift").contains("doku-mitziehen"));
}

#[test]
fn the_docs_are_published_only_after_the_review_and_the_approval() {
    let flow = read("release-flow");
    assert!(publishes_after(
        &flow,
        &["review", "tag", "push", "main-ff"]
    ));
    let up = read("upstream-update");
    assert!(publishes_after(
        &up,
        &["review", "review-again", "tag", "push"]
    ));
    let rn = read("umbenennen");
    assert!(publishes_after(
        &rn,
        &["review-again", "freigabe", "merge", "github-umbenennen"]
    ));
    // counter-tests: the publish step moved before the review is caught
    let moved = flow
        .replacen("  - doku-veroeffentlichen:", "  - zz:", 1)
        .replacen("  - gates:", "  - doku-veroeffentlichen:", 1)
        .replacen("  - zz:", "  - gates:", 1);
    assert!(!publishes_after(
        &moved,
        &["review", "tag", "push", "main-ff"]
    ));
    let by_agent = flow.replacen("name: publish-doc", "name: other", 1);
    assert!(!publishes_after(&by_agent, &["review"]));
}

#[test]
fn upstream_update_has_a_resolved_head_a_caught_timeout_and_a_review_with_a_verdict() {
    let up = read("upstream-update");
    // K1: no `head` as input from step 1
    assert!(!up.contains(".input.head"));
    assert!(step_body(&up, "head").contains("In a run from step 1 there is no input `head`"));
    // K2: the timeout is caught and sends the flow to the escalation
    let wait = step_body(&up, "wait-build");
    assert!(wait.contains("try:") && wait.contains("catch:") && wait.contains("errors/timeout"));
    assert!(step_body(&up, "build-weiter").contains("then: upstream-eskalation"));
    assert!(
        pos(&up, "wait-build") < pos(&up, "build-weiter")
            && pos(&up, "build-weiter") < pos(&up, "publish")
    );
    // K3: review.yaml returns verdict and runde, which review-again reads
    let rv = read("review");
    assert!(
        rv.contains("\noutput:\n")
            && rv.contains("verdict: $context.review.verdict")
            && rv.contains("runde:")
    );
    assert!(step_body(&rv, "verdict").contains("export:"));
    assert!(step_body(&up, "review-again").contains("$context.review.verdict"));
}

#[test]
fn counter_a_caller_without_the_step_or_after_the_review_is_caught() {
    let flow = read("release-flow");
    let without = flow.replacen("name: doku-mitziehen", "name: other", 1);
    assert!(!calls_doku_before(&without, "gates"));
    let up = read("upstream-update");
    let renamed = up.replacen("name: doku-mitziehen", "name: other", 1);
    assert!(!calls_doku_before(&renamed, "head"));
}
