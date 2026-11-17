//! Pattern: none (test). Workflow maintenance run 3: `grundfassung` (step `hauptansicht`, picture pair gate, test `server-unresponsive`,
//! bundle switch before `release`, head rules), the new flow `bahnen` (lanes round in fs-tasks) and the `chain-rebase` hint in `katalog`.
//! Every rule has a counter-test (step struck, in the wrong place, or the key phrase changed: red).

mod support;
use support::workflows::{assert_red, in_order, read, step_body, step_names};

#[path = "workflows_pflege8/part1.rs"]
mod part1;
pub use part1::*;

#[path = "workflows_pflege8/part2.rs"]
mod part2;
pub use part2::*;
