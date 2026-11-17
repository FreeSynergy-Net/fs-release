//! Pattern: none (test). The workflows of the maintenance run 1: `ff-und-tag`, `branch-neu-schreiben`, `katalog`, the step
//! `version-abgleich` in `release-flow`, the step `ci-lesen` and the delta mode in `review`. Each rule has a counter-test on a changed copy.

mod support;
use support::workflows::{in_order, read, step_body, step_names, FILES};

/// Swaps the names of two steps (the bodies stay): the same flow with the two steps in the other order.
fn swapped(text: &str, a: &str, b: &str) -> String {
    text.replacen(&format!("  - {a}:"), "  - zz-tmp:", 1)
        .replacen(&format!("  - {b}:"), &format!("  - {a}:"), 1)
        .replacen("  - zz-tmp:", &format!("  - {b}:"), 1)
}

#[path = "workflows_pflege7/part1.rs"]
mod part1;
use part1::*;

#[path = "workflows_pflege7/part2.rs"]
mod part2;
use part2::*;

#[path = "workflows_pflege7/part3.rs"]
mod part3;
