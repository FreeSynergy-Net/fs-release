//! Pattern: none (a poll with a clear blocker text).

use std::path::PathBuf;
use std::time::{Duration, Instant};

use anyhow::{ensure, Context, Result};

/// The arguments of `wait`.
#[derive(clap::Args, Clone)]
pub struct WaitArgs {
    /// Registry repo to read (clone URL).
    #[arg(long)]
    pub registry_url: String,
    #[arg(long)]
    pub package: String,
    #[arg(long)]
    pub version: String,
    /// Give up after this many minutes (0 = look once).
    #[arg(long, default_value_t = 20)]
    pub minutes: u64,
    /// Seconds between two looks.
    #[arg(long, default_value_t = 120)]
    pub interval_s: u64,
    /// Page of the index workflow in the registry repo, named in the blocker text.
    #[arg(
        long,
        default_value = "https://github.com/FreeSynergy-Net/fs-crates/actions/workflows/index.yml"
    )]
    pub workflow_page: String,
    /// New directory for the read-only clone.
    #[arg(long)]
    pub work_dir: PathBuf,
}

/// The blocker text (one click for the user).
#[must_use]
pub fn blocker(a: &WaitArgs) -> String {
    format!(
        "BLOCKED: {} {} is not in the registry index after {} min. The index workflow in fs-crates is switched off or late (GitHub switches scheduled workflows off after 60 days without activity in the repo; cron can be late). One click: open {} , press `Enable workflow` if it is off, then `Run workflow`.",
        a.package, a.version, a.minutes, a.workflow_page
    )
}

/// Looks until the version is in the index (reads only, no right needed). Returns the blocker text as the error after the time is up.
///
/// # Errors
/// The version did not appear in time (the error text is the blocker), or the clone fails.
pub fn wait(a: &WaitArgs) -> Result<()> {
    let start = Instant::now();
    let mut n = 0;
    loop {
        n += 1;
        let dir = a.work_dir.join(format!("look-{n}"));
        let st = std::process::Command::new("git")
            .args(["clone", "-q", "--depth", "1", &a.registry_url])
            .arg(&dir)
            .status()
            .context("cannot start git")?;
        ensure!(st.success(), "cannot read {}", a.registry_url);
        if !reg_index::list(&dir, &a.package, Some(&format!("={}", a.version)))?.is_empty() {
            return Ok(());
        }
        if start.elapsed() >= Duration::from_secs(a.minutes * 60) {
            anyhow::bail!("{}", blocker(a));
        }
        std::thread::sleep(Duration::from_secs(a.interval_s));
    }
}
