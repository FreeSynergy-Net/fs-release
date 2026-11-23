//! Pattern: none (data: one row of `api-kosten.log`, the cost table of the runner `run.sh`).
//!
//! Row format: `| UTC | Aufgabe | Modell | Tok ein | Tok aus | Tok Cache | USD | Dauer s | Exit | Quelle | RunID |`.

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
