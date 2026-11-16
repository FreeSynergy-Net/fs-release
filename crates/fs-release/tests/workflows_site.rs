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

/// The build job has no AI step and names no secret, whatever the spelling (`${{ secrets.x }}`, `${ $secrets.x }`, `secrets:`, `GPG_...`).
fn build_has_no_ai_and_no_secret(text: &str) -> bool {
    let low = text.to_lowercase();
    !text.contains("actor: ai")
        && !low.contains("secret")
        && !low.contains("gpg_")
        && !low.contains("passphrase")
        && !low.contains("private_key")
}

/// The text of one job of `.github/workflows/site.yml` (from `  <job>:` to the next job or the end).
fn job(text: &str, name: &str) -> String {
    let head = format!("  {name}:");
    let (mut inside, mut out) = (false, Vec::new());
    for line in text.lines() {
        let top =
            line.starts_with("  ") && !line.starts_with("   ") && line.trim_end().ends_with(':');
        if inside && top {
            break;
        }
        if line == head {
            inside = true;
        }
        if inside {
            out.push(line);
        }
    }
    assert!(inside, "no job {name}");
    out.join("\n")
}

/// The CI side of ADR 0078: `build` has no key, no environment and no write right and runs on main only; `sign` waits for `build`, sits behind the
/// protected environment, runs on main only, and holds the key only in the step that calls site-signieren.sh.
fn ci_jobs_are_separated(text: &str) -> bool {
    let b = job(text, "build");
    let s = job(text, "sign");
    let low = b.to_lowercase();
    !low.contains("secret")
        && !low.contains("gpg_")
        && !b.contains("environment:")
        && !b.contains("write")
        && b.contains("refs/heads/main")
        && s.contains("needs: build")
        && s.contains("environment: site-signing")
        && s.contains("refs/heads/main")
        && s.contains("secrets.GPG_PRIVATE_KEY")
        && s.contains("scripts/site-signieren.sh")
        && !text.contains("pull_request")
        && !text.contains("push:")
}

/// The approval is the person's (`human` gate, `subjective`), a script checks it (`machine`, gate `freigabe-beleg`) before the signing, and
/// the signing is a machine step behind the same gate.
fn approval_then_check_then_sign(text: &str) -> bool {
    let a = step_body(text, "approval");
    let c = step_body(text, "approval-check");
    let s = step_body(text, "sign");
    a.contains("listen:")
        && !a.contains("call:")
        && a.contains("actor: human")
        && a.contains("gate: human")
        && a.contains("kind: subjective")
        && c.contains("actor: machine")
        && c.contains("gate: freigabe-beleg")
        && s.contains("actor: machine")
        && s.contains("gate: freigabe-beleg")
        && position(text, "build") < position(text, "request")
        && position(text, "request") < position(text, "approval")
        && position(text, "approval") < position(text, "approval-check")
        && step_body(text, "rebuild").contains("actor: machine")
        && step_body(text, "rebuild").contains("gate: freigabe-beleg")
        && position(text, "approval-check") < position(text, "rebuild")
        && position(text, "rebuild") < position(text, "sign")
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
    assert!(position(&text, "build") < position(&text, "hash-out"));
    // a step that only says "done elsewhere" can never turn red: there is none
    assert!(!step_names(&text).iter().any(|n| n == "same-bytes"));
}

#[test]
fn counter_site_bauen_an_ai_step_or_a_secret_is_caught() {
    let text = read("site-bauen");
    assert!(!build_has_no_ai_and_no_secret(&text.replacen(
        "actor: machine",
        "actor: ai",
        1
    )));
    for leak in [
        "# GPG_PRIVATE_KEY\n",
        "# key: ${{ secrets.release_key }}\n",
        "# key: ${ $secrets.release_key }\n",
        "# secrets: inherit\n",
        "# GPG_PASSPHRASE\n",
    ] {
        assert!(
            !build_has_no_ai_and_no_secret(&format!("{text}{leak}")),
            "{leak}"
        );
    }
}

#[test]
fn the_ci_jobs_build_and_sign_are_separated() {
    let text =
        std::fs::read_to_string(root().join(".github/workflows/site.yml")).expect("site.yml");
    assert!(ci_jobs_are_separated(&text));
}

#[test]
fn counter_ci_jobs_a_key_in_build_or_a_missing_protection_is_caught() {
    let text =
        std::fs::read_to_string(root().join(".github/workflows/site.yml")).expect("site.yml");
    // a secret in the build job (the first `run:` of the file is in `build`)
    let leak = text.replacen(
        "          MODE: ${{ inputs.mode }}\n",
        "          MODE: ${{ inputs.mode }}\n          K: ${{ secrets.GPG_PRIVATE_KEY }}\n",
        1,
    );
    assert_ne!(leak, text);
    assert!(!ci_jobs_are_separated(&leak));
    // sign without the protected environment, without `needs`, on any branch; build with an environment
    assert!(!ci_jobs_are_separated(
        &text.replace("    environment: site-signing\n", "")
    ));
    assert!(!ci_jobs_are_separated(
        &text.replace("    needs: build\n", "")
    ));
    assert!(!ci_jobs_are_separated(
        &text.replace("github.ref == 'refs/heads/main' && ", "")
    ));
    assert!(!ci_jobs_are_separated(&text.replacen(
        "    runs-on: ubuntu-24.04\n",
        "    runs-on: ubuntu-24.04\n    environment: site-signing\n",
        1
    )));
    // a trigger from a pull request would run the scripts of the pull request
    assert!(!ci_jobs_are_separated(&text.replace(
        "workflow_dispatch:",
        "pull_request:\n  workflow_dispatch:"
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
    // the person's approval replaced by an automatic call with the same type and subject (a text-only check would not see it)
    let auto = text.replace(
        "      listen:\n        to:\n          one:\n            with:\n              type: net.freesynergy.approval\n              subject: ${ $context.tree }",
        "      call: auto-approve\n      with:\n        type: net.freesynergy.approval\n        subject: ${ $context.tree }",
    );
    assert_ne!(auto, text, "the mutation did not apply");
    assert!(!approval_then_check_then_sign(&auto));
    // the rebuild comparison after the key was used, or its gate weakened
    let late = text
        .replacen("  - rebuild:", "  - zz:", 1)
        .replacen("  - sign:", "  - rebuild:", 1)
        .replacen("  - zz:", "  - sign:", 1);
    assert_ne!(late, text);
    assert!(!approval_then_check_then_sign(&late));
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
        text.lines().filter(|l| l.starts_with("ok")).count() >= 120,
        "{text}"
    );
}
