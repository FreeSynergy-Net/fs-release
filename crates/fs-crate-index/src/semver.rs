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
    /// New directory for the clone and the unpacked baseline.
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
    /// A baseline exists but no `cargo-semver-checks` binary was given: NOT checked (never reported as green).
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

fn registry_checkout(a: &SemverArgs) -> Result<PathBuf> {
    std::fs::create_dir_all(&a.work_dir)?;
    let dir = if a.registry.contains("://") {
        let d = a.work_dir.join("registry");
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
        d
    } else {
        PathBuf::from(&a.registry)
    };
    reg_index::load_config(&dir).with_context(|| {
        format!(
            "REGISTRY NOT READABLE ({}): no config.json (this is not \"no baseline\")",
            a.registry
        )
    })?;
    Ok(dir)
}

/// The lint level of `cargo-semver-checks` stands in the `Cargo.toml` of the CHECKED package (`[package.metadata.cargo-semver-checks.lints]`), so a branch could switch the gate
/// off by itself. Therefore every manifest of the source is read first: only `enum_variant_added = "warn"` may be there, anything else is RED.
///
/// # Errors
/// A manifest sets a lint level other than `enum_variant_added = "warn"`, or has other `cargo-semver-checks` keys.
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
    for top in ["package", "workspace"] {
        let Some(cfg) = doc
            .get(top)
            .and_then(|t| t.get("metadata"))
            .and_then(|m| m.get("cargo-semver-checks"))
        else {
            continue;
        };
        let lints = cfg.get("lints").and_then(toml::Value::as_table);
        let only_lints = cfg
            .as_table()
            .is_some_and(|t| t.keys().all(|k| k == "lints"));
        let lints_ok = lints.is_none_or(|l| {
            l.iter()
                .all(|(k, v)| k == "enum_variant_added" && v.as_str() == Some("warn"))
        });
        ensure!(
            only_lints && lints_ok,
            "{}: [{top}.metadata.cargo-semver-checks] changes the gate (only `enum_variant_added = \"warn\"` is allowed): the checked branch must not release itself",
            manifest.display()
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
    let reg = registry_checkout(a)?;
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
