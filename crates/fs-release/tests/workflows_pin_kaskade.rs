//! Pattern: none (test). `pin-kaskade` (specification): ONE cascade after a new fs-types tag, Glied by Glied in DAG order. Per Glied: the existing `lift`,
//! strict lock check, gates (`CARGO_BUILD_JOBS=4`, `BUILD_PRIORITY=0`), branch CI, Opus review, `ff-und-tag` as the only way to main and tag, THEN the next Glied.
//! A red Glied aborts (no skipping, no repeat), resume with `ab`; the end checks one fs-types version in every lock. Counter-tests on changed copies.

mod support;
use support::workflows::{in_order, read, step_body};

fn ok(t: &str) -> bool {
    let to = |s: &str, target: &str| {
        let b = step_body(t, s);
        b.contains(&format!("then: {target}"))
    };
    in_order(
        t,
        &[
            "vorhanden-pruefen",
            "karte",
            "lage",
            "plan",
            "glied-weiche",
            "glied-start",
            "pin-lift",
            "besonderes",
            "lock-strikt",
            "gates",
            "gates-weiter",
            "push-zweig",
            "zweig-ci",
            "ci-weiter",
            "review",
            "review-weiter",
            "ff-tag",
            "ff-weiter",
            "naechstes",
            "lock-ende",
            "lock-ende-weiter",
            "bericht",
            "eskalation",
        ],
    ) && step_body(t, "pin-lift").contains("name: lift\n")
        && step_body(t, "ff-tag").contains("name: ff-und-tag\n")
        && !t.contains("name: release-flow")
        && !t.contains("tag-annotated")
        && step_body(t, "gates").contains("BUILD_PRIORITY=0")
        && step_body(t, "gates").contains("CARGO_BUILD_JOBS=4")
        && step_body(t, "gates").contains("[GATE-PIPEFAIL]")
        && t.contains("[KEINE-KALTEN-FLACHPROBEN-BIS-AUFRAEUMEN]")
        && step_body(t, "lock-strikt").contains("strict = true")
        && step_body(t, "gates-weiter").contains("ABORT")
        && step_body(t, "gates-weiter").contains("then: eskalation")
        && !step_body(t, "gates-weiter").contains("then: pin-lift")
        && to("naechstes", "glied-weiche")
        && step_body(t, "review").contains("gate: opus-ja")
        && step_body(t, "eskalation").contains("actor: human")
        && step_body(t, "eskalation").contains("`ab`")
        && step_body(t, "lock-ende").contains("SAME fs-types version")
        && t.contains(".input[\"ausloeser-tag\"]")
        && t.contains(".input.glieder")
        && t.contains(".input.ab")
}

#[test]
fn pin_kaskade_follows_the_order_and_calls_the_existing_flows() {
    assert!(ok(&read("pin-kaskade")));
}

#[test]
fn counter_own_tag_way_skipped_abort_or_changed_order_are_caught() {
    let t = read("pin-kaskade");
    let bad = |x: String| x != t && !std::panic::catch_unwind(|| ok(&x)).unwrap_or(false);
    assert!(bad(t.replace("name: ff-und-tag\n", "name: release-flow\n")));
    assert!(bad(t.replace("name: lift\n", "name: lift-v2\n")));
    assert!(bad(t.replace("BUILD_PRIORITY=0", "BUILD_PRIORITY=5")));
    assert!(bad(t.replace("strict = true", "strict = false")));
    assert!(bad(t.replace("then: glied-weiche", "then: lock-ende")));
    assert!(bad(t.replace("  - lock-strikt:", "  - lock-x:")));
    assert!(bad(t.replacen("gate: opus-ja", "gate: none", 1)));
    assert!(bad(format!("{t}\n# tag-annotated\n")));
}

#[test]
fn the_cascade_calls_each_other_flow_only_by_run_and_passes_the_inputs() {
    let t = read("pin-kaskade");
    let ff = step_body(&t, "ff-tag");
    for k in ["repo", "branch", "head", "tag", "allow", "workdir"] {
        assert!(
            ff.contains(&format!("          {k}: ")),
            "ff-und-tag input {k}"
        );
    }
    let lift = step_body(&t, "pin-lift");
    for k in ["repo", "commit", "libs"] {
        assert!(lift.contains(&format!("          {k}: ")), "lift input {k}");
    }
}
