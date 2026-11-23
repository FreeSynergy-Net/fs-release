//! Pattern: none (data: one row of `api-kosten.log`, the cost table of the runner `run.sh`).
//!
//! Row format: `| UTC | Aufgabe | Modell | Tok ein | Tok aus | Tok Cache | USD | Dauer s | Exit | Quelle | RunID |`.

use std::path::Path;

/// The columns of a run that the approval needs.
#[derive(Debug, PartialEq, Eq)]
pub struct LogRow {
    pub task: String,
    pub model: String,
    pub exit: String,
    pub run_id: String,
}

impl LogRow {
    /// The row of `run_id` if the log holds exactly one; `None` for none or several.
    #[must_use]
    pub fn find(log: &str, run_id: &str) -> Option<Self> {
        let mut rows = log
            .lines()
            .filter_map(Self::parse)
            .filter(|r| r.run_id == run_id);
        let first = rows.next()?;
        rows.next().is_none().then_some(first)
    }

    fn parse(line: &str) -> Option<Self> {
        let cells: Vec<&str> = line.split('|').map(str::trim).collect();
        // leading and trailing empty cells around the 11 columns
        if cells.len() != 13 || !cells[0].is_empty() {
            return None;
        }
        Some(Self {
            task: cells[2].into(),
            model: cells[3].into(),
            exit: cells[9].into(),
            run_id: cells[11].into(),
        })
    }
}

/// The texts of the run records `out-<task>-*.json` of the task of `run_id` (the lander reads them; `verify` only compares).
#[must_use]
pub fn run_records(dir: &Path, log: &str, run_id: &str) -> Vec<String> {
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
            n.starts_with(&prefix)
                && p.extension()
                    .is_some_and(|e| e.eq_ignore_ascii_case("json"))
        })
        .filter_map(|p| std::fs::read_to_string(p).ok())
        .collect()
}
