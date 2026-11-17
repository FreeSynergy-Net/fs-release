//! Pattern: none (test). Workflow maintenance run 2: the new flows `stoerung-diagnose` (read-only diagnosis, no fix in the same run),
//! `branches-aufraeumen` (list and user script, the agents delete nothing), `menschen-aufnahme` (consent only by the person) and the
//! extended `upstream-anfrage` (proof, duplicates, English text without AI words, git-bug id, no pull request, no comment).
//! Every rule has a counter-test on the real file (step struck, in the wrong place, actor changed or the key phrase changed: red).

mod support;
use support::workflows::{assert_red, in_order, read, step_body, step_names};

#[path = "workflows_pflege9/part1.rs"]
mod part1;
pub use part1::*;

#[path = "workflows_pflege9/part2.rs"]
mod part2;
pub use part2::*;

#[path = "workflows_pflege9/part3.rs"]
mod part3;
pub use part3::*;

#[path = "workflows_pflege9/part4.rs"]
mod part4;
