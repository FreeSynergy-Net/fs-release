//! Pattern: none (test). Workflow maintenance run 5: the fixed step `upstream-watch` (rule: always answer at the upstream). The sub-workflow
//! `upstream-watch` reads (GET only) the answers to our open Blinc pull requests and issues, makes a card and a short draft for every new
//! comment of another person and sends NOTHING itself (the helper of b7 sends after the YES of the user). The step stands in `koordinator-runde`
//! and `bahnen` with the repeat condition "older than 2 hours". No `gh`/network = YELLOW, never silently green. Counter-tests on the real files.

mod support;
use support::workflows::{in_order as order_of, read, step_body, step_names};

/// Steps in this order (a missing step is no order: false, never a panic).
fn in_order(text: &str, steps: &[&str]) -> bool {
    steps.iter().all(|s| has_step(text, s)) && order_of(text, steps)
}

#[path = "workflows_pflege10/part1.rs"]
mod part1;
use part1::*;

#[path = "workflows_pflege10/part2.rs"]
mod part2;
