//! Pattern: Pipeline (pull path, ADR 0093): `scan` = job 1 (reads, packages, checks, writes a plan), `apply` = job 2 (only enters; the tool comes from its own job and is checked by sha256).
//!
//! The allow list lives in the registry repo itself (`allowed-sources.txt`): one line per source repo, `owner/repo <first-version> <package>...`, the
//! packages in dependency order (a package may depend on the ones before it). A change of that file needs a review (CODEOWNERS, docs/crate-index.md).

use std::fmt::Write as _;
use std::path::{Path, PathBuf};

use anyhow::{bail, ensure, Context, Result};
use semver::Version;

use crate::allow::{list_tags, parse_allow, url_of, Source, TagInfo};
use crate::publish::{git_out, package};
use crate::semver::{gate, work_for, Outcome, SemverArgs};

/// The arguments of `scan`.
#[derive(clap::Args, Clone)]
pub struct ScanArgs {
    /// Allow list (`allowed-sources.txt` of the registry repo).
    #[arg(long)]
    pub allow: PathBuf,
    /// Checkout of the registry repo (read only here).
    #[arg(long)]
    pub registry_dir: PathBuf,
    /// Base URL of the source repos.
    #[arg(long, default_value = "https://github.com")]
    pub source_base: String,
    /// New, empty directory for the clones and the packaging output.
    #[arg(long)]
    pub work_dir: PathBuf,
    /// New, empty directory for the result: `plan.txt` and the `.crate` files (the hand-over to `apply`).
    #[arg(long)]
    pub out: PathBuf,
    /// `cargo-semver-checks` binary (pinned, checksum checked by the workflow). REQUIRED (r3, M2): without it a crate with a baseline cannot be checked, and then nothing is planned (RED).
    #[arg(long, required = true)]
    pub semver_checks: Option<PathBuf>,
}

/// Result of a scan: how many crates are in the plan and the first problem (the scan stops there; earlier tags stay in the plan).
#[derive(Debug)]
pub struct ScanReport {
    pub entries: usize,
    pub notes: Vec<String>,
    pub error: Option<String>,
}

fn in_index(reg: &Path, name: &str, version: &Version) -> Result<bool> {
    Ok(!reg_index::list(reg, name, Some(&format!("={version}")))?.is_empty())
}

/// The commit a tag had when it was entered (from the commit messages of the registry), if any.
fn recorded_commit(reg: &Path, repo: &str, tag: &str) -> Result<Option<String>> {
    let log = git_out(reg, &["log", "--format=%B"])?;
    let key = format!("source: {repo} {tag} commit ");
    Ok(log
        .lines()
        .find_map(|l| l.strip_prefix(&key))
        .map(|r| r.split_whitespace().next().unwrap_or("").to_string()))
}

/// Job 1: finds new tags of the allowed repos, checks them, packages them, writes `plan.txt` and the `.crate` files. Nothing is written to the registry.
///
/// # Errors
/// Only for unusable arguments; a problem with a tag is reported in `ScanReport::error` (and the scan stops there).
pub fn scan(a: &ScanArgs) -> Result<ScanReport> {
    let sources = parse_allow(&std::fs::read_to_string(&a.allow).context("allow list")?)?;
    for d in [&a.work_dir, &a.out] {
        ensure!(
            !d.exists() || d.read_dir()?.next().is_none(),
            "{} must be new or empty",
            d.display()
        );
        std::fs::create_dir_all(d)?;
    }
    let mut report = ScanReport {
        entries: 0,
        notes: Vec::new(),
        error: None,
    };
    let mut plan = String::new();
    'all: for s in &sources {
        let tags = match list_tags(&url_of(&a.source_base, &s.repo)) {
            Ok(t) => t,
            Err(e) => {
                report.error = Some(format!("{e:#}"));
                break;
            }
        };
        for t in tags.iter().filter(|t| t.version >= s.from) {
            match scan_tag(a, s, t, &mut plan, &mut report) {
                Ok(()) => {}
                Err(e) => {
                    report.error = Some(format!("{} {}: {e:#}", s.repo, t.name));
                    break 'all;
                }
            }
        }
    }
    std::fs::write(a.out.join("plan.txt"), plan)?;
    // `reg-index` leaves a lock file when it reads: the registry checkout stays as it was
    let _ = std::fs::remove_file(a.registry_dir.join(".cargo-index-lock"));
    Ok(report)
}

fn scan_tag(
    a: &ScanArgs,
    s: &Source,
    t: &TagInfo,
    plan: &mut String,
    rep: &mut ScanReport,
) -> Result<()> {
    let Some(commit) = &t.commit else {
        bail!("the tag is not annotated (lightweight): releases are annotated tags");
    };
    if let Some(old) = recorded_commit(&a.registry_dir, &s.repo, &t.name)? {
        ensure!(
            &old == commit,
            "the tag was MOVED: entered at commit {old}, now at {commit}; nothing is entered again"
        );
    }
    let mut pending = Vec::new();
    for p in &s.packages {
        if !in_index(&a.registry_dir, p, &t.version)? {
            pending.push(p.clone());
        }
    }
    if pending.is_empty() {
        return Ok(());
    }
    let dir = a
        .work_dir
        .join(format!("{}-{}", s.repo.replace('/', "_"), t.name));
    git_out(
        Path::new("."),
        &[
            "clone",
            "-q",
            &url_of(&a.source_base, &s.repo),
            &dir.to_string_lossy(),
        ],
    )?;
    ensure!(
        git_out(&dir, &["rev-parse", &format!("refs/tags/{}", t.name)])?.trim() == t.tag_object,
        "the tag changed while it was being read"
    );
    git_out(
        &dir,
        &["merge-base", "--is-ancestor", commit, "origin/main"],
    )
    .context("the tag commit is not on main of the source repo")?;
    git_out(&dir, &["checkout", "-q", "--detach", commit])?;
    let files = package(
        &dir,
        &s.packages,
        &t.version.to_string(),
        &a.work_dir.join(format!(
            "target-{}",
            dir.file_name().unwrap().to_string_lossy()
        )),
    )?;
    for (p, f) in s.packages.iter().zip(&files) {
        if !pending.contains(p) {
            continue;
        }
        let o = gate(&SemverArgs {
            registry: a.registry_dir.to_string_lossy().into_owned(),
            package: p.clone(),
            source: dir.clone(),
            version: Some(t.version.clone()),
            semver_checks: a.semver_checks.clone(),
            work_dir: work_for(&a.work_dir, &format!("{p}-{}", t.version)),
        })?;
        ensure!(
            !matches!(o, Outcome::NotRun(_)),
            "semver gate NOT RUN for {p} (baseline exists, no --semver-checks): not planned, NOT RUN is never green"
        );
        rep.notes.push(o.line(p));
        let fname = f.file_name().unwrap().to_string_lossy().into_owned();
        std::fs::copy(f, a.out.join(&fname))?;
        let _ = writeln!(
            plan,
            "{}\t{}\t{}\t{commit}\t{p}\t{}\t{fname}",
            s.repo, t.name, t.tag_object, t.version
        );
        rep.entries += 1;
    }
    Ok(())
}
