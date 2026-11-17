//! Pattern: none (test). The 0.0.7 rules of the workflow files: the [EFFIZIENZ] step rules, the place of `rules.md`, the decision flow
//! (ADR search first), texts before tokens in `frontend-umbau`, jumps back as `switch` with `then`. Each rule has a counter-test.

mod support;
use support::workflows::{assert_red, read, step_body, step_names};

#[path = "workflows_pflege6/part1.rs"]
mod part1;
use part1::*;

#[path = "workflows_pflege6/part2.rs"]
mod part2;
