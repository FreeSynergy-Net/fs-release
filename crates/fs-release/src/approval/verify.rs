//! Pattern: Chain of Responsibility. The checks of the proof of the runner (see the module doc of `approval`).

use super::reject::Reject;
use super::runner_log::LogRow;
use super::runner_out::RunOut;

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

/// The report names the head: `JA am <full SHA>` (40 hex characters, no prefix: a prefix of 7 characters can be forced onto another commit).
fn names_head(first: &str, sha: &str) -> bool {
    first
        .strip_prefix("JA am ")
        .and_then(|r| r.split(|c: char| !c.is_ascii_hexdigit()).next())
        .is_some_and(|p| p.len() == 40 && p == sha)
}

fn is_full_sha(sha: &str) -> bool {
    sha.len() == 40 && sha.bytes().all(|b| b.is_ascii_hexdigit())
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
    if first != format!("JA am {}", ev.sha) || !is_full_sha(ev.sha) {
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
