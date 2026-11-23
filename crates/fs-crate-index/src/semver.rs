//! Pattern: Strategy-free single implementation of the semver gate (ONE copy: the pull job in fs-crates and the early signal in the publisher repos both call it).
//!
//! Baseline = ALWAYS the newest (not yanked) version of the crate in the registry, taken as its unpacked `.crate`; never a git tag. Three cases:
//! 1. the registry is reachable and the crate has NO version: green, said visibly (`no baseline: first registry version`);
//! 2. the registry is reachable and the crate has a version: `cargo-semver-checks check-release --release-type minor --baseline-root <unpacked .crate>`;
//! 3. the registry is not reachable (clone fails, no `config.json`): RED with its own text, never confused with case 1.

use std::path::{Path, PathBuf};
use std::process::Command;

use anyhow::{ensure, Context, Result};
use semver::Version;

/// The arguments of the gate (also of the subcommand `semver`).
#[derive(clap::Args, Clone)]
pub struct SemverArgs {
    /// The registry: a clone URL (cloned read only, shallow) or a checkout directory.
    #[arg(long)]
    pub registry: String,
    #[arg(long)]
    pub package: String,
    /// Source checkout of the new version (the workspace that holds the package).
    #[arg(long)]
    pub source: PathBuf,
    /// The new version; only versions older than this count as baseline (default: every version in the registry).
    #[arg(long)]
    pub version: Option<Version>,
    /// `cargo-semver-checks` binary (the subcommand needs it).
    #[arg(long)]
    pub semver_checks: Option<PathBuf>,
    /// Directory for the clone and the unpacked baseline; may be reused by later calls and may hold other files: each call clones into its own new subdirectory and removes only that again, nothing else in it is touched.
    #[arg(long)]
    pub work_dir: PathBuf,
}

/// What the gate found.
#[derive(Debug, PartialEq, Eq)]
pub enum Outcome {
    /// Case 1: the crate is not in the registry yet.
    NoBaseline,
    /// Case 2: compared with this version, green.
    Compared(Version),
    /// A baseline exists but no `cargo-semver-checks` binary was given: NOT checked (never green: `scan` and the subcommand `semver` are RED then).
    NotRun(Version),
}

impl Outcome {
    /// One line for the log.
    #[must_use]
    pub fn line(&self, pkg: &str) -> String {
        match self {
            Self::NoBaseline => {
                format!("semver {pkg}: no baseline: first registry version (green)")
            }
            Self::Compared(b) => format!("semver {pkg}: compared with registry version {b}: green"),
            Self::NotRun(b) => {
                format!("semver {pkg}: NOT RUN (baseline {b} exists, no --semver-checks)")
            }
        }
    }
}

/// The directory of the registry; for a clone URL also the guard that removes the clone again (only what THIS call made).
fn registry_checkout(a: &SemverArgs) -> Result<(PathBuf, Option<tempfile::TempDir>)> {
    std::fs::create_dir_all(&a.work_dir)?;
    let (dir, guard) = if a.registry.contains("://") {
        // Every call clones into its OWN new directory below the work dir (never a fixed name): nothing that was already there is touched or deleted (a foreign `registry` stays
        // as it is), a clone of an earlier call is never reused (always a fresh read), and the work dir may be shared by several calls. The guard removes our own clone at the end.
        let tmp = tempfile::Builder::new()
            .prefix("registry-")
            .tempdir_in(&a.work_dir)
            .with_context(|| {
                format!(
                    "cannot create a clone directory in {}",
                    a.work_dir.display()
                )
            })?;
        let d = tmp.path().to_path_buf();
        let out = Command::new("git")
            .args(["clone", "-q", "--depth", "1", &a.registry])
            .arg(&d)
            .output()
            .context("cannot start git")?;
        ensure!(
            out.status.success(),
            "REGISTRY NOT REACHABLE ({}): {} (this is not \"no baseline\")",
            a.registry,
            String::from_utf8_lossy(&out.stderr).trim()
        );
        (d, Some(tmp))
    } else {
        (PathBuf::from(&a.registry), None)
    };
    reg_index::load_config(&dir).with_context(|| {
        format!(
            "REGISTRY NOT READABLE ({}): no config.json (this is not \"no baseline\")",
            a.registry
        )
    })?;
    Ok((dir, guard))
}

/// The lint level of `cargo-semver-checks` stands in the `Cargo.toml` of the CHECKED package, so a branch could switch the gate off by itself. Therefore every manifest of the
/// source is read first: ANY `cargo-semver-checks` table (`[package|workspace.metadata.cargo-semver-checks]`, `[lints|workspace.lints.cargo-semver-checks]`) is RED.
///
/// Not even `enum_variant_added = "warn"` is allowed (r3, review M3): with `warn` the tool reports the hit but does NOT fail, and the filter by type (only `BasicType` and
/// `CommandKind` may get a new variant, ADR 0093) is not built, so a new variant of any exhaustive enum (`ZoneKind`) would be green. Until that filter exists a new variant
/// in `BasicType` or `CommandKind` is a deliberate, visible break.
///
/// # Errors
/// A manifest has a `cargo-semver-checks` table.
pub fn check_manifest_lints(source: &Path) -> Result<()> {
    let mut stack = vec![source.to_path_buf()];
    while let Some(dir) = stack.pop() {
        for e in std::fs::read_dir(&dir)? {
            let p = e?.path();
            let name = p
                .file_name()
                .and_then(|n| n.to_str())
                .unwrap_or_default()
                .to_string();
            if p.is_dir() {
                if !matches!(name.as_str(), "target" | ".git" | ".cargo") {
                    stack.push(p);
                }
            } else if name == "Cargo.toml" {
                manifest_lints_ok(&p)?;
            }
        }
    }
    Ok(())
}

fn manifest_lints_ok(manifest: &Path) -> Result<()> {
    let text = std::fs::read_to_string(manifest)?;
    let doc: toml::Table = text
        .parse()
        .with_context(|| format!("{} is not TOML", manifest.display()))?;
    for (top, sub) in [
        ("package", "metadata"),
        ("workspace", "metadata"),
        ("lints", ""),
        ("workspace", "lints"),
    ] {
        let t = doc.get(top);
        let t = if sub.is_empty() {
            t
        } else {
            t.and_then(|t| t.get(sub))
        };
        ensure!(
            t.and_then(|t| t.get("cargo-semver-checks")).is_none(),
            "{}: a `cargo-semver-checks` table ({top}{}{sub}) changes the gate (no lint table is allowed, not even `warn`): the checked branch must not release itself",
            manifest.display(),
            if sub.is_empty() { "" } else { "." }
        );
    }
    Ok(())
}

/// Runs the gate.
///
/// # Errors
/// The registry is not reachable or not readable, the baseline `.crate` is missing, or `cargo-semver-checks` is red.
pub fn gate(a: &SemverArgs) -> Result<Outcome> {
    check_manifest_lints(&a.source)?;
    let (reg, _clone) = registry_checkout(a)?;
    let pkg = &a.package;
    let base = reg_index::list(&reg, pkg, None)?
        .into_iter()
        .filter(|p| !p.yanked && a.version.as_ref().is_none_or(|v| &p.vers < v))
        .map(|p| p.vers)
        .max();
    let _ = std::fs::remove_file(reg.join(".cargo-index-lock"));
    let Some(base) = base else {
        return Ok(Outcome::NoBaseline);
    };
    let Some(bin) = &a.semver_checks else {
        return Ok(Outcome::NotRun(base));
    };
    let krate = reg.join(format!("crates/{pkg}/{pkg}-{base}.crate"));
    ensure!(
        krate.is_file(),
        "baseline {} is in the index but its .crate is missing",
        krate.display()
    );
    let into = a.work_dir.join(format!("baseline-{pkg}-{base}"));
    std::fs::create_dir_all(&into)?;
    let tar = Command::new("tar")
        .arg("-xzf")
        .arg(&krate)
        .arg("-C")
        .arg(&into)
        .output()
        .context("tar")?;
    ensure!(
        tar.status.success(),
        "cannot unpack the baseline {}",
        krate.display()
    );
    let out = Command::new(bin)
        .args([
            "semver-checks",
            "check-release",
            "--release-type",
            "minor",
            "--baseline-root",
        ])
        .arg(into.join(format!("{pkg}-{base}")))
        .args(["-p", pkg])
        .current_dir(&a.source)
        .output()
        .context("cannot start cargo-semver-checks")?;
    ensure!(
        out.status.success(),
        "semver gate RED for {pkg} against registry version {base}:\n{}{}",
        String::from_utf8_lossy(&out.stdout),
        String::from_utf8_lossy(&out.stderr)
    );
    Ok(Outcome::Compared(base))
}

/// Path helper for callers: where the gate wants its work.
#[must_use]
pub fn work_for(dir: &Path, pkg: &str) -> PathBuf {
    dir.join(format!("semver-{pkg}"))
}
