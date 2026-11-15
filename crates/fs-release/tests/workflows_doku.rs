//! Pattern: none (test). The rule doku-mitziehen: the documentation moves with every change (sub-workflow `doku-mitziehen`, a mandatory step in
//! `release-flow`, `programm-neu`, `upstream-update`, `umbenennen`) and the flow `umbenennen` (dry run without AI, approval before the GitHub
//! rename, name lock). Each rule has a counter-test on a changed copy; the two scripts run their own self tests.

mod support;
use std::process::Command;
use support::workflows::{read, root, step_body, step_names};

fn pos(text: &str, step: &str) -> usize {
    step_names(text)
        .iter()
        .position(|n| n == step)
        .unwrap_or_else(|| panic!("no step {step}"))
}

fn in_order(text: &str, order: &[&str]) -> bool {
    order.windows(2).all(|w| pos(text, w[0]) < pos(text, w[1]))
}

fn calls_doku_before(text: &str, later: &str) -> bool {
    let body = step_body(text, "doku");
    body.contains("name: doku-mitziehen")
        && body.contains("actor: machine")
        && body.contains("kind: objective")
        && pos(text, "doku") < pos(text, later)
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
            "gate",
            "gate-again",
            "veroeffentlichen",
        ],
    ) && gate.contains("actor: machine")
        && gate.contains("kind: objective")
        && gate.contains("doku-mitziehen:0.0.6@fs")
        && gate.contains("docs_base")
        && again.contains("then: eskalation")
        && again.contains(">= 3")
        && again.contains("then: suchen")
        && step_body(text, "schreiben").contains("actor: ai")
        && step_body(text, "veroeffentlichen").contains("gate: human")
}

fn rename_is_sound(text: &str) -> bool {
    let order = [
        "trockenlauf",
        "plan-red",
        "plan-gate",
        "umbau",
        "sperre",
        "doku",
        "nachher",
        "gates",
        "review",
        "freigabe",
        "github-umbenennen",
        "weiterleitung",
        "release",
    ];
    let github = step_body(text, "github-umbenennen");
    in_order(text, &order)
        && step_body(text, "trockenlauf").contains("actor: machine")
        && step_body(text, "trockenlauf").contains("scripts/umbenennen dry-run")
        && step_body(text, "nachher").contains("scripts/umbenennen after")
        && step_body(text, "sperre").contains("retired-names.txt")
        && step_body(text, "doku").contains("name: doku-mitziehen")
        && step_body(text, "freigabe").contains("gate: strategie-ok")
        && step_body(text, "freigabe").contains("net.freesynergy.approval")
        && github.contains("actor: human")
        && github.contains("gate: strategie-ok")
        && step_body(text, "review").contains("gate: opus-ja")
        && step_body(text, "nachher-again").contains("then: eskalation")
        && step_body(text, "review").contains(".input.head")
}

#[test]
fn the_sub_workflow_has_the_checks_a_red_loop_and_a_human_publish() {
    assert!(gate_is_sound(&read("doku-mitziehen")));
}

#[test]
fn counter_a_gate_by_the_agent_or_without_escalation_is_caught() {
    let t = read("doku-mitziehen");
    let by_ai = t.replacen("call: doku-mitziehen:0.0.6@fs", "call: docs-ai:0.0.6@fs", 1);
    assert!(!gate_is_sound(&by_ai));
    assert!(!gate_is_sound(
        &t.replace("then: eskalation", "then: continue")
    ));
    assert!(!gate_is_sound(&t.replace(">= 3", ">= 99")));
    assert!(!gate_is_sound(&t.replace("then: suchen", "then: continue")));
}

#[test]
fn the_mandatory_step_is_in_every_release_workflow_before_the_review() {
    assert!(calls_doku_before(&read("release-flow"), "gates"));
    assert!(calls_doku_before(&read("release-flow"), "review-request"));
    assert!(calls_doku_before(&read("programm-neu"), "release"));
    assert!(calls_doku_before(&read("upstream-update"), "head"));
    assert!(calls_doku_before(&read("upstream-update"), "review"));
    assert!(read("lift").contains("doku-mitziehen"));
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

#[test]
fn umbenennen_runs_the_dry_run_first_and_renames_only_after_the_approval() {
    assert!(rename_is_sound(&read("umbenennen")));
}

#[test]
fn counter_a_rename_without_the_gate_or_by_an_agent_is_caught() {
    let t = read("umbenennen");
    assert!(!rename_is_sound(&t.replacen(
        "gate: strategie-ok",
        "gate: none",
        2
    )));
    let github = step_body(&t, "github-umbenennen");
    let by_ai = github.replace("actor: human", "actor: ai");
    assert!(!rename_is_sound(&t.replacen(&github, &by_ai, 1)));
    assert!(!rename_is_sound(
        &t.replace("scripts/umbenennen dry-run", "an agent looks")
    ));
    assert!(!rename_is_sound(&t.replace("retired-names.txt", "nothing")));
    let swapped = t
        .replacen("  - freigabe:", "  - zz:", 1)
        .replacen("  - github-umbenennen:", "  - freigabe:", 1)
        .replacen("  - zz:", "  - github-umbenennen:", 1);
    assert!(!rename_is_sound(&swapped));
}

fn run_self_test(name: &str) {
    if Command::new("python3").arg("--version").output().is_err() {
        return;
    }
    let out = Command::new("bash")
        .arg(root().join("scripts").join(name))
        .output()
        .expect("bash");
    let text = String::from_utf8_lossy(&out.stdout);
    assert!(
        out.status.success() && text.contains("all ok"),
        "{text}\n{}",
        String::from_utf8_lossy(&out.stderr)
    );
}

#[test]
fn the_doku_gate_script_passes_its_self_test() {
    run_self_test("test_doku_mitziehen.sh");
}

#[test]
fn the_rename_script_passes_its_self_test() {
    run_self_test("test_umbenennen.sh");
}
