//! Pattern: none (test). The flows `site-bauen` and `site-signieren` (stage 0 of the website): the build has no secrets and no AI step, the
//! signing sits behind a human approval that a script checks, nobody pushes or logs in to a server. Each rule has a counter-test on a
//! changed copy. The scripts have their own selftest, `scripts/test-site.sh`, which this file runs.

mod support;
use std::process::Command;
use support::workflows::{read, root, step_body, step_names};

fn position(text: &str, step: &str) -> usize {
    step_names(text)
        .iter()
        .position(|n| n == step)
        .unwrap_or_else(|| panic!("no step {step}"))
}

/// The build job has no AI step and names no secret.
fn build_has_no_ai_and_no_secret(text: &str) -> bool {
    !text.contains("actor: ai")
        && !text.contains("GPG_PRIVATE_KEY")
        && !text.contains("GPG_PASSPHRASE")
        && !text.contains("${{ secrets")
}

/// The approval is the person's (`human` gate, `subjective`), a script checks it (`machine`, gate `freigabe-beleg`) before the signing, and
/// the signing is a machine step behind the same gate.
fn approval_then_check_then_sign(text: &str) -> bool {
    let a = step_body(text, "approval");
    let c = step_body(text, "approval-check");
    let s = step_body(text, "sign");
    a.contains("actor: human")
        && a.contains("gate: human")
        && a.contains("kind: subjective")
        && c.contains("actor: machine")
        && c.contains("gate: freigabe-beleg")
        && s.contains("actor: machine")
        && s.contains("gate: freigabe-beleg")
        && position(text, "build") < position(text, "request")
        && position(text, "request") < position(text, "approval")
        && position(text, "approval") < position(text, "approval-check")
        && position(text, "approval-check") < position(text, "sign")
        && position(text, "sign") < position(text, "signatures-check")
        && position(text, "signatures-check") < position(text, "deliver")
}

/// The approval waits for an event of exactly this hash; a listen without a subject would take any approval.
fn approval_is_bound_to_the_hash(text: &str) -> bool {
    let a = step_body(text, "approval");
    a.contains("type: net.freesynergy.approval") && a.contains("subject: ${ $context.tree }")
}

/// The person delivers and logs; no AI step after the signing.
fn delivery_is_human_and_last(text: &str) -> bool {
    let after = &step_names(text)[position(text, "deliver")..];
    after
        .iter()
        .all(|s| step_body(text, s).contains("actor: human"))
}

#[test]
fn site_bauen_has_no_ai_step_and_no_secret() {
    let text = read("site-bauen");
    assert!(build_has_no_ai_and_no_secret(&text));
    for s in step_names(&text) {
        assert!(step_body(&text, &s).contains("actor: machine"), "{s}");
    }
    assert!(position(&text, "source-check") < position(&text, "build"));
    assert!(position(&text, "build") < position(&text, "same-bytes"));
    assert!(position(&text, "same-bytes") < position(&text, "hash-out"));
}

#[test]
fn counter_site_bauen_an_ai_step_or_a_secret_is_caught() {
    let text = read("site-bauen");
    assert!(!build_has_no_ai_and_no_secret(&text.replacen(
        "actor: machine",
        "actor: ai",
        1
    )));
    assert!(!build_has_no_ai_and_no_secret(&format!(
        "{text}# GPG_PRIVATE_KEY\n"
    )));
}

#[test]
fn site_signieren_signs_only_after_the_human_approval_and_the_script_check() {
    let text = read("site-signieren");
    assert!(approval_then_check_then_sign(&text));
    assert!(approval_is_bound_to_the_hash(&text));
    assert!(delivery_is_human_and_last(&text));
    assert!(step_body(&text, "build").contains("name: site-bauen"));
    assert!(step_body(&text, "approval-check").contains("pinned main"));
    assert!(step_body(&text, "sign").contains("missing = red"));
}

#[test]
fn counter_site_signieren_a_weaker_approval_or_another_order_is_caught() {
    let text = read("site-signieren");
    // the approval by an AI, a check as a human step, a sign step before the check: all caught
    let ai = text.replacen("actor: human", "actor: ai", 1);
    assert!(!approval_then_check_then_sign(&ai));
    let unchecked = text.replace("gate: freigabe-beleg", "gate: none");
    assert!(!approval_then_check_then_sign(&unchecked));
    let swapped = text
        .replacen("  - approval-check:", "  - zz:", 1)
        .replacen("  - sign:", "  - approval-check:", 1)
        .replacen("  - zz:", "  - sign:", 1);
    assert!(!approval_then_check_then_sign(&swapped));
    // an approval that is not bound to the hash
    assert!(!approval_is_bound_to_the_hash(
        &text.replace("subject: ${ $context.tree }", "subject: any")
    ));
    assert!(!approval_is_bound_to_the_hash(
        &text.replace("type: net.freesynergy.approval", "type: other")
    ));
    // an AI step after the delivery
    let late = format!(
        "{text}  - post:\n      call: x:0.0.6@fs\n      metadata:\n        freesynergy.net:actor: ai\n"
    );
    assert!(!delivery_is_human_and_last(&late));
}

#[test]
fn the_selftest_of_the_site_scripts_is_green() {
    let out = Command::new("bash")
        .arg(root().join("scripts/test-site.sh"))
        .output()
        .expect("bash");
    let text = String::from_utf8_lossy(&out.stdout);
    assert!(
        out.status.success(),
        "{text}\n{}",
        String::from_utf8_lossy(&out.stderr)
    );
    assert!(!text.contains("FAIL"), "{text}");
    assert!(
        text.lines().filter(|l| l.starts_with("ok")).count() >= 40,
        "{text}"
    );
}
