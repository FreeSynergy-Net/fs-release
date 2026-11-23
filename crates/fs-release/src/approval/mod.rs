//! Pattern: Chain of Responsibility (each proof is one check that either passes or names the reason; the first red one stops).
//!
//! `JA am <SHA>` is only accepted with a proof from the runner (`run.sh`, G6 no. 10): a verdict file written by hand (by any process of the same user)
//! has no run behind it. The proof: (1) line 1 of the verdict file is exactly `JA am <full SHA>`; (2) the run id has exactly one row in the cost log
//! with a review task (`rev-…`), an Opus model and exit 0; (3) the run's own record `out-<task>-*.json` names the same model, ran in the review
//! folder `wt/<task>` and wrote, with `Write` to its `BERICHT.md` (the only file a review run may write), a report whose first line names the same
//! commit and whose rest is byte-equal to the rest of the verdict file. What this does not prove (the log has no MODE column; the files belong to the
//! same user as the agents) is written down in the workflow step `ja-fuer-kopf`.
pub mod reject;
pub mod runner_log;
pub mod runner_out;
pub mod verify;

pub use reject::Reject;
pub use runner_log::run_records;
pub use verify::{verify, Evidence};
