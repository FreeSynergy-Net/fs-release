//! Pattern: none (test). The flows `programm-grundgeruest`, `testlauf` and the `output` of `release-flow` (bundle 0.0.7): the order, the gates,
//! the fixed trait (locked BEFORE the approval, the approval bound to its hash), the red loops with escalation, and that a missing result is
//! never green. Each rule has a counter-test on a changed copy. The flow `koordinator-runde` is in `workflows_runde.rs`.

mod support;
use support::workflows::{read, step_body, step_names};

#[path = "workflows_belt/part1.rs"]
mod part1;
pub use part1::*;

#[path = "workflows_belt/part2.rs"]
mod part2;
pub use part2::*;
