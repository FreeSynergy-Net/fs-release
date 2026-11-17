//! Test support for the flows of the kit scaffold (run 9): the shape of a check step with its `-again` switch, and the board card as first step.

use super::workflows::{in_order, step_body, step_names};

/// A `<step>-again` switch: third red round and missing result go to the escalation, a red result goes back to `back`, only green continues.
pub fn again_ok(text: &str, step: &str, back: &str, esc: &str) -> bool {
    let b = step_body(text, step);
    let sw = b.split("metadata:").next().unwrap();
    let cases: Vec<&str> = sw.split("\n        - ").skip(1).collect();
    let open = |c: &&str| !c.contains("when:");
    b.contains(">= 3")
        && b.contains(&format!("then: {esc}"))
        && b.contains(&format!("then: {back}"))
        && cases
            .last()
            .is_some_and(|c| open(c) && c.contains(&format!("then: {esc}")))
        && !cases
            .iter()
            .any(|c| open(c) && c.contains("then: continue"))
}

pub fn with_checks(text: &str, pairs: &[(&str, &str)]) -> bool {
    pairs.iter().all(|(s, back)| {
        let names = step_names(text);
        names.contains(&format!("{s}-check"))
            && in_order(text, &[s, &format!("{s}-check"), &format!("{s}-again")])
            && again_ok(text, &format!("{s}-again"), back, "eskalation")
    })
}

pub fn starts_with_card(text: &str) -> bool {
    step_names(text).first().is_some_and(|n| n == "karte")
        && step_body(text, "karte").contains("fs-tasks")
}

/// True when `ok` calls the changed copy red; a changed copy that lost a step the rule names (the rule panics on a missing step) is red as well.
pub fn broken(ok: fn(&str) -> bool, text: &str) -> bool {
    std::panic::catch_unwind(|| !ok(text)).unwrap_or(true)
}
