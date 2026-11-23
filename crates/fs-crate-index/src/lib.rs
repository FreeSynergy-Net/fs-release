//! Pattern: Adapter (our guards around the `reg-index` library; the index format itself is not ours).
//!
//! `fs-crate-index publish` puts ONE crate version into the git index registry `freesynergy` (repo `FreeSynergy-Net/fs-crates`):
//! `cargo package --locked` -> `.crate` -> index line (name, version, dependencies, features, SHA-256 checksum) in the layout of the
//! cargo registry index -> the `.crate` file under `crates/{crate}/` -> ONE commit. Pushing is the job of the workflow, not of this tool.
//!
//! Guards (threat model: protects against MISTAKES, not against someone who can change workflow, toolchain or scripts; whoever can write
//! to the registry repo can swap crates, the checksum lies in the same repo):
//! - only a tag run: `--git-ref` must be `refs/tags/vX.Y.Z`; a branch, a pull request or nothing writes nothing;
//! - tag = version of the package (as for every release);
//! - `config.json` of the registry exists and its `dl` leads to exactly the place where the `.crate` is stored;
//! - a version already in the index is never replaced (`reg-index` refuses).

use std::path::{Path, PathBuf};
use std::process::Command;

use anyhow::{bail, ensure, Context, Result};
use clap::{Parser, Subcommand};

#[derive(Parser)]
#[command(
    name = "fs-crate-index",
    version,
    about = "Write one crate version into the git index registry (tag run only)"
)]
struct Cli {
    #[command(subcommand)]
    command: Sub,
}

#[derive(Subcommand)]
enum Sub {
    /// Package the crate, add the index line and the `.crate` file to the registry checkout, commit there (no push).
    Publish(PublishArgs),
}

/// The arguments of `publish`.
#[derive(clap::Args, Clone)]
pub struct PublishArgs {
    /// The full git ref of the run (`${{ github.ref }}`); only `refs/tags/vX.Y.Z` writes.
    #[arg(long)]
    pub git_ref: String,
    /// The package (`-p`) to publish.
    #[arg(long)]
    pub package: String,
    /// Directory of the workspace of the package (default: current directory).
    #[arg(long, default_value = ".")]
    pub workspace: PathBuf,
    /// Checkout of the registry repo (git repository with `config.json`).
    #[arg(long)]
    pub registry_dir: PathBuf,
    /// The URL consumers use for the index (`index = ...` in `.cargo/config.toml`).
    #[arg(long)]
    pub index_url: String,
    /// Where the `.crate` goes inside the registry checkout; `{crate}` and `{version}` are replaced.
    #[arg(long, default_value = "crates/{crate}")]
    pub upload: String,
    /// Directory for the packaging output; must be new or empty (a stale `.crate` must never be taken).
    #[arg(long)]
    pub target_dir: PathBuf,
}

/// Version of a tag run: `refs/tags/v0.0.7` -> `0.0.7`. Everything else is refused.
///
/// # Errors
/// The ref is not a tag `vX.Y.Z` (a branch, a pull request, an empty value, a pre-release or build suffix).
pub fn version_of_ref(git_ref: &str) -> Result<String> {
    let Some(tag) = git_ref.strip_prefix("refs/tags/v") else {
        bail!("`{git_ref}` is not a tag run (refs/tags/vX.Y.Z): the registry is written only from a tag, never from a branch or by hand");
    };
    let parts: Vec<&str> = tag.split('.').collect();
    let plain = parts.len() == 3
        && parts
            .iter()
            .all(|p| !p.is_empty() && p.bytes().all(|b| b.is_ascii_digit()) && (p.len() == 1 || !p.starts_with('0')));
    ensure!(plain, "tag `v{tag}` is not vX.Y.Z (digits only, no pre-release, no build suffix)");
    Ok(tag.to_string())
}

/// The path of the `.crate` below the registry checkout, from the `dl` template of `config.json`, and the check that `--upload` leads there.
///
/// # Errors
/// `config.json` is missing or unreadable, or `dl` does not end with `<upload>/{crate}-{version}.crate`.
pub fn check_dl(registry_dir: &Path, upload: &str, name: &str, version: &str) -> Result<()> {
    let cfg = reg_index::load_config(registry_dir).context("config.json of the registry (the registry repo must be initialised first)")?;
    let dl = cfg.dl.as_str().replace("%7B", "{").replace("%7D", "}").replace("%7b", "{").replace("%7d", "}");
    let resolved = dl.replace("{crate}", name).replace("{version}", version);
    let want_tail = format!("{}/{name}-{version}.crate", upload.replace("{crate}", name).replace("{version}", version));
    ensure!(
        resolved.ends_with(&want_tail),
        "`dl` of config.json resolves to `{resolved}`, which does not end with `{want_tail}`: the .crate would be stored where cargo does not look"
    );
    Ok(())
}

/// Package, add to the index, commit.
///
/// # Errors
/// Any guard (see the module doc), `cargo package` fails, the version is already in the index, git fails.
pub fn publish(a: &PublishArgs) -> Result<String> {
    let version = version_of_ref(&a.git_ref)?;
    ensure!(
        !a.target_dir.join("package").exists(),
        "--target-dir {} already holds packaging output: use a new directory",
        a.target_dir.display()
    );
    let out = Command::new("cargo")
        .args(["package", "--locked", "-p", &a.package, "--target-dir"])
        .arg(&a.target_dir)
        .current_dir(&a.workspace)
        .output()
        .context("cannot start cargo")?;
    ensure!(out.status.success(), "cargo package failed:\n{}", String::from_utf8_lossy(&out.stderr));
    let crate_file = a.target_dir.join("package").join(format!("{}-{version}.crate", a.package));
    ensure!(
        crate_file.is_file(),
        "tag v{version} does not match the version of `{}`: {} was not produced (tag = version of the package)",
        a.package,
        crate_file.display()
    );
    check_dl(&a.registry_dir, &a.upload, &a.package, &version)?;
    check_features(&reg_index::metadata_from_crate(&a.index_url, &crate_file)?.features)?;
    let upload = a.upload.replace("{crate}", &a.package).replace("{version}", &version);
    let pkg = reg_index::add_from_crate(&a.registry_dir, &a.index_url, &crate_file, Some(&format!("{}/{}", a.registry_dir.display(), a.upload)))
        .context("index entry")?;
    // `reg-index` commits only the index file; the `.crate` goes into the SAME commit (amend, no second commit)
    let rel = format!("{upload}/{}-{version}.crate", a.package);
    git(&a.registry_dir, &["add", &rel])?;
    git(&a.registry_dir, &["commit", "--amend", "--no-edit", "-q"])?;
    // the lock file of `reg-index` is not part of the registry
    let _ = std::fs::remove_file(a.registry_dir.join(".cargo-index-lock"));
    Ok(format!("{} {} cksum {} -> {rel}", pkg.name, pkg.vers, pkg.cksum))
}

fn git(dir: &Path, args: &[&str]) -> Result<()> {
    let out = Command::new("git").arg("-C").arg(dir).args(args).output().context("cannot start git")?;
    ensure!(out.status.success(), "git {} failed: {}", args.join(" "), String::from_utf8_lossy(&out.stderr));
    Ok(())
}

/// Parses the command line, runs it and returns the exit code.
#[must_use]
pub fn main_with(args: Vec<String>) -> i32 {
    let cli = match Cli::try_parse_from(args) {
        Ok(c) => c,
        Err(e) => {
            let _ = e.print();
            return if e.use_stderr() { 2 } else { 0 };
        }
    };
    match cli.command {
        Sub::Publish(a) => match publish(&a) {
            Ok(line) => {
                println!("GREEN crate-index {line}");
                0
            }
            Err(e) => {
                println!("RED crate-index: {e:#}");
                1
            }
        },
    }
}

/// `reg-index` 0.6.0 writes index format v1 only: a feature with `dep:` or `?/` would be indexed wrongly (cargo needs `features2` and `v: 2`), so it is refused.
///
/// # Errors
/// A feature value uses `dep:` or `?/`.
pub fn check_features(features: &std::collections::BTreeMap<String, Vec<String>>) -> Result<()> {
    for (name, values) in features {
        if let Some(v) = values.iter().find(|v| v.starts_with("dep:") || v.contains("?/")) {
            bail!("feature `{name}` uses `{v}`: index format v2 is not supported by reg-index 0.6.0, the entry would be wrong");
        }
    }
    Ok(())
}
