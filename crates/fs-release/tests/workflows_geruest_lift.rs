//! Pattern: none (test). `geruest-lift` is created INACTIVE (until `fs-blueprint` exists): it calls the existing `lift` unchanged, and `lift` / `lift-v1plus`
//! keep their names and are still called by `release-flow` and `lift-v1plus`. Counter-tests on changed copies.

mod support;
use support::workflows::{in_order, read, step_body};

fn lift_ok(t: &str) -> bool {
    let call = step_body(t, "lift");
    in_order(
        t,
        &[
            "bauplan-anheben",
            "erzeugen",
            "lift",
            "check",
            "check-again",
            "eskalation",
        ],
    ) && t.contains("INACTIVE")
        && step_body(t, "bauplan-anheben").contains("INACTIVE")
        && step_body(t, "erzeugen").contains("fs-blueprint generate")
        && step_body(t, "check").contains("fs-blueprint check")
        && call.contains("name: lift\n")
        && call.contains("UNCHANGED")
        && call.contains("repo: ${ .input.repo }")
}

#[test]
fn geruest_lift_is_inactive_and_calls_lift_unchanged() {
    assert!(lift_ok(&read("geruest-lift")));
}

#[test]
fn counter_geruest_lift_active_wording_another_sub_flow_or_a_renamed_lift_are_caught() {
    let t = read("geruest-lift");
    let bad = |x: String| x != t && !std::panic::catch_unwind(|| lift_ok(&x)).unwrap_or(false);
    assert!(bad(t.replace("INACTIVE", "active")));
    assert!(bad(t.replace("name: lift\n", "name: lift-v2\n")));
    assert!(bad(t.replace("`fs-blueprint generate`", "a script")));
    assert!(bad(t.replace("UNCHANGED", "changed")));
}

#[test]
fn lift_and_lift_v1plus_keep_their_names_and_callers() {
    assert!(read("release-flow").contains("name: lift\n"));
    assert!(read("lift-v1plus").contains("name: lift\n"));
    assert!(
        support::workflows::FILES.contains(&"lift")
            && support::workflows::FILES.contains(&"lift-v1plus")
    );
}
