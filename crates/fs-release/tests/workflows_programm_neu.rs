//! Pattern: none (test). The flow `programm-neu`: the 17 steps in the phases P0 to P6, an ADR reference and a `kind` on every step,
//! every objective agent step followed by an objective script check, jumps back as `switch`, the human stays human (server run,
//! acceptance), the missing flow `facette-anlegen` only as a TODO stub. Each rule has a counter-test on a changed copy.

mod support;
use support::workflows::{assert_red, read, step_body, step_names};

#[path = "workflows_programm_neu/part1.rs"]
mod part1;
pub use part1::*;

#[path = "workflows_programm_neu/part2.rs"]
mod part2;
pub use part2::*;

#[path = "workflows_programm_neu/part3.rs"]
mod part3;
pub use part3::*;
