//! Pattern: none (data). Why a `JA` is not accepted.

use std::fmt;

/// Why a `JA` is not accepted.
#[derive(Debug, PartialEq, Eq)]
pub enum Reject {
    FirstLine(String),
    NoSingleLogRow(String),
    NotReviewTask(String),
    NotOpus(String),
    RunFailed(String),
    NoRunRecord,
    RecordModel(String),
    RecordFolder(String),
    NoMatchingReport,
}

impl fmt::Display for Reject {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::FirstLine(l) => write!(f, "line 1 of the verdict is not `JA am <this SHA>`: '{l}'"),
            Self::NoSingleLogRow(r) => write!(f, "the cost log has not exactly one row for run {r}"),
            Self::NotReviewTask(t) => write!(f, "task '{t}' is not a review run (rev-...)"),
            Self::NotOpus(m) => write!(f, "model '{m}' is not Opus"),
            Self::RunFailed(e) => write!(f, "the run ended with exit {e}"),
            Self::NoRunRecord => write!(f, "no record of the run (out-<task>-*.json)"),
            Self::RecordModel(m) => write!(f, "the record names model '{m}', not Opus"),
            Self::RecordFolder(c) => write!(f, "the record ran in '{c}', not in the review folder wt/<task>"),
            Self::NoMatchingReport => write!(f, "the runner recorded no BERICHT.md write that matches the verdict (hand-written file?)"),
        }
    }
}
