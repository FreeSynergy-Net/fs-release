//! Pattern: Chain of Responsibility (each proof is one check that either passes or names the reason; the first red one stops).
//!
//! `JA am <SHA>` is only accepted with a proof from the runner (`run.sh`, G6 no. 10): a verdict file written by hand (by any process of the same user)
//! has no run behind it. The proof: (1) line 1 of the verdict file is exactly `JA am <full SHA>`; (2) the run id has exactly one row in the cost log
//! with a review task (`rev-…`), an Opus model and exit 0; (3) the run's own record `out-<task>-*.json` names the same model, ran in the review
//! folder `wt/<task>` and wrote, with `Write` to its `BERICHT.md` (the only file a review run may write), a report whose first line names the same
//! commit and whose rest is byte-equal to the rest of the verdict file. What this does not prove (the log has no MODE column; the files belong to the
//! same user as the agents) is written down in the workflow step `ja-fuer-kopf`.

pub mod runner_log;
pub mod runner_out;

use std::fmt;

use runner_log::LogRow;
use runner_out::RunOut;

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

/// What the lander hands in: the verdict file, the wanted head, the run id and the texts the runner left.
pub struct Evidence<'a> {
    pub verdict: &'a str,
    pub sha: &'a str,
    pub run_id: &'a str,
    pub log: &'a str,
    /// The texts of every `out-<task>-*.json` of that task.
    pub records: &'a [String],
}

fn first_and_rest(text: &str) -> (&str, &str) {
    text.split_once('\n').unwrap_or((text, ""))
}

/// The report names the head: `JA am <prefix>` with a prefix (>= 7 characters) of the full SHA.
fn names_head(first: &str, sha: &str) -> bool {
    first
        .strip_prefix("JA am ")
        .and_then(|r| r.split(|c: char| !c.is_ascii_hexdigit()).next())
        .is_some_and(|p| p.len() >= 7 && sha.starts_with(p))
}

fn review_row(ev: &Evidence) -> Result<LogRow, Reject> {
    let row =
        LogRow::find(ev.log, ev.run_id).ok_or_else(|| Reject::NoSingleLogRow(ev.run_id.into()))?;
    if !row.task.starts_with("rev-") {
        return Err(Reject::NotReviewTask(row.task));
    }
    if !row.model.contains("opus") {
        return Err(Reject::NotOpus(row.model));
    }
    if row.exit != "0" {
        return Err(Reject::RunFailed(row.exit));
    }
    Ok(row)
}

fn record_matches(rec: &RunOut, task: &str, verdict_rest: &str, sha: &str) -> Result<bool, Reject> {
    if !rec.model.contains("opus") {
        return Err(Reject::RecordModel(rec.model.clone()));
    }
    if !rec.cwd.ends_with(&format!("/wt/{task}")) {
        return Err(Reject::RecordFolder(rec.cwd.clone()));
    }
    let report = format!("{}/BERICHT.md", rec.cwd);
    Ok(rec.writes.iter().any(|(path, body)| {
        let (first, rest) = first_and_rest(body);
        *path == report && names_head(first, sha) && rest == verdict_rest
    }))
}

/// Accepts the verdict only with the proof of the runner.
///
/// # Errors
/// The first missing proof as a [`Reject`].
pub fn verify(ev: &Evidence) -> Result<(), Reject> {
    let (first, rest) = first_and_rest(ev.verdict);
    if first != format!("JA am {}", ev.sha) || ev.sha.len() != 40 {
        return Err(Reject::FirstLine(first.into()));
    }
    let row = review_row(ev)?;
    if ev.records.is_empty() {
        return Err(Reject::NoRunRecord);
    }
    let mut last = Reject::NoMatchingReport;
    for text in ev.records {
        match record_matches(&RunOut::parse(text), &row.task, rest, ev.sha) {
            Ok(true) => return Ok(()),
            Ok(false) => {}
            Err(e) => last = e,
        }
    }
    Err(last)
}

/// The texts of the run records `out-<task>-*.json` of the task of `run_id` (the lander reads them; `verify` only compares).
#[must_use]
pub fn run_records(dir: &std::path::Path, log: &str, run_id: &str) -> Vec<String> {
    let Some(row) = LogRow::find(log, run_id) else {
        return Vec::new();
    };
    let prefix = format!("out-{}-", row.task);
    let Ok(rd) = std::fs::read_dir(dir) else {
        return Vec::new();
    };
    let mut names: Vec<_> = rd.flatten().map(|e| e.path()).collect();
    names.sort();
    names
        .iter()
        .filter(|p| {
            let n = p.file_name().and_then(|n| n.to_str()).unwrap_or("");
            n.starts_with(&prefix) && n.ends_with(".json")
        })
        .filter_map(|p| std::fs::read_to_string(p).ok())
        .collect()
}
