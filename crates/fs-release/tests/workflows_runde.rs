//! Pattern: none (test). The flow `koordinator-runde` (bundle 0.0.7): who does which step, the order, the gates, the red loops with escalation and
//! the rules. Agents run in the BACKGROUND on purpose: the coordinator is its own session and the messages go to it (instruction of the strategy,
//! 2026-11-15 evening; the earlier foreground rule applied only to coordinators inside the strategy session). The coordinator still ends only when
//! all agents have reported (`einsammeln` waits for all). Every result is green only when it says so; a missing result is never green.
//! Each rule has a counter-test on a changed copy. The flows `programm-grundgeruest` and `testlauf` are in `workflows_belt.rs`.

mod support;
use support::workflows::{read, step_body, step_names};

#[path = "workflows_runde/part1.rs"]
mod part1;
pub use part1::*;

#[path = "workflows_runde/part2.rs"]
mod part2;
pub use part2::*;
