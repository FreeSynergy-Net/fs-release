//! Pattern: none (test). Workflow maintenance run 8: rule [GATE-PIPEFAIL] (`scripts/gates.sh` has `pipefail`, prints an rc per gate, never cuts
//! a gate output with `tail`/`head`; the review text and docs state the rule) and the branch CI of this repo (`branch-ci.yml` runs on `unreviewed/**`).
//! Each rule has a counter-test on a changed copy. Seven rc lines: fmt, clippy, test, rust-rules, lock, files, guard ([GATES-WAECHTER]).

mod support;
use support::workflows::{read, root};

fn file(rel: &str) -> String {
    std::fs::read_to_string(root().join(rel)).unwrap()
}

/// `pipefail` set, an rc line per gate, and no line pipes into `tail` or `head` (a pipe into them hides the exit code of the gate).
fn gates_ok(text: &str) -> bool {
    let code = |l: &&str| !l.trim_start().starts_with('#');
    let cut = |l: &str| {
        let l = l.replace(' ', "");
        l.contains("|tail") || l.contains("|head")
    };
    text.contains("set -euo pipefail")
        && text.contains("trap 'echo \"gates FAILED")
        && text.matches("rc=0\"").count() >= 7
        && !text.lines().filter(code).any(cut)
}

#[test]
fn gates_sh_has_pipefail_an_rc_per_gate_and_no_tail_or_head() {
    assert!(gates_ok(&file("scripts/gates.sh")));
}

#[test]
fn counter_gates_sh_rule_breaks_are_caught() {
    let g = file("scripts/gates.sh");
    assert!(!gates_ok(&g.replace("set -euo pipefail", "set -eu")));
    assert!(!gates_ok(
        &g.replace("cargo fmt --check", "cargo fmt --check | tail -3")
    ));
    assert!(!gates_ok(
        &g.replace("cargo fmt --check", "cargo fmt --check |head -1")
    ));
    assert!(!gates_ok(&g.replace("echo \"gate fmt: rc=0\"\n", "")));
}

#[test]
fn the_review_text_and_the_docs_state_the_gate_rule() {
    let review = read("review");
    for t in [
        &review,
        &file("docs/usage.md"),
        &file("workflows/README.md"),
    ] {
        assert!(t.contains("GATE-PIPEFAIL"), "rule missing");
        assert!(t.contains("pipefail") && t.contains("tail"));
    }
    assert!(!review
        .replace("GATE-PIPEFAIL", "")
        .contains("GATE-PIPEFAIL"));
}

fn triggers_unreviewed(text: &str) -> bool {
    text.contains("push:")
        && text.contains("'unreviewed/**'")
        && !text.contains("pull_request")
        && text.contains("scripts/gates.sh")
}

#[test]
fn branch_ci_runs_the_gates_on_push_to_unreviewed_and_the_reusable_workflows_stay() {
    let b = file(".github/workflows/branch-ci.yml");
    assert!(triggers_unreviewed(&b));
    assert!(!triggers_unreviewed(
        &b.replace("'unreviewed/**'", "'main'")
    ));
    assert!(!triggers_unreviewed(&b.replace("push:", "pull_request:")));
    for f in ["ci.yml", "check.yml"] {
        let t = file(&format!(".github/workflows/{f}"));
        assert!(t.contains("workflow_call:") && !t.contains("push:"), "{f}");
    }
}
