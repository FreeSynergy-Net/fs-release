//! Pattern: Adapter (our steps around the `reg-index` library; the index format itself is not ours).

use std::fmt::Write as _;
use std::path::{Path, PathBuf};
use std::process::Command;

use anyhow::{ensure, Context, Result};

use crate::guard::{index_file, split_v2, version_of_ref};

/// The arguments of `publish` (job 2 of the workflow: no cargo, no build script runs here).
#[derive(clap::Args, Clone)]
pub struct PublishArgs {
    /// The full git ref of the tag (`refs/tags/vX.Y.Z`); only that writes.
    #[arg(long)]
    pub git_ref: String,
    /// The packaged `.crate` file.
    #[arg(long)]
    pub crate_file: PathBuf,
    /// Checkout of the registry repo (git repository with `config.json`).
    #[arg(long)]
    pub registry_dir: PathBuf,
    /// The URL consumers use for the index (`index = ...` in `.cargo/config.toml`).
    #[arg(long)]
    pub index_url: String,
    /// Where the `.crate` goes inside the registry checkout; `{crate}` and `{version}` are replaced.
    #[arg(long, default_value = "crates/{crate}")]
    pub upload: String,
    /// Text for the commit message: where the crate comes from (repo, tag, tag commit).
    #[arg(long, default_value = "")]
    pub source: String,
}

/// `cargo package --locked` for all given packages in ONE call (interdependent packages verify against each other), in a new target dir.
/// Returns the `.crate` files in the order of `packages`. This is job 1: it never gets a write token.
///
/// # Errors
/// The target dir is not new, cargo fails, or a package has another version than the tag (`<package>-<version>.crate` is missing).
pub fn package(
    workspace: &Path,
    packages: &[String],
    version: &str,
    target_dir: &Path,
) -> Result<Vec<PathBuf>> {
    ensure!(
        !target_dir.join("package").exists(),
        "target dir {} already holds packaging output: use a new directory",
        target_dir.display()
    );
    let mut cmd = Command::new("cargo");
    cmd.args(["package", "--locked", "--target-dir"])
        .arg(target_dir)
        .current_dir(workspace);
    for p in packages {
        cmd.args(["-p", p]);
    }
    let out = cmd.output().context("cannot start cargo")?;
    ensure!(
        out.status.success(),
        "cargo package failed:\n{}",
        String::from_utf8_lossy(&out.stderr)
    );
    packages
        .iter()
        .map(|p| {
            let f = target_dir
                .join("package")
                .join(format!("{p}-{version}.crate"));
            ensure!(
                f.is_file(),
                "tag v{version} does not match the version of `{p}`: {} was not produced (tag = version of the package)",
                f.display()
            );
            Ok(f)
        })
        .collect()
}

/// Checks that the `dl` template of `config.json` leads to `<upload>/{crate}-{version}.crate`.
///
/// # Errors
/// `config.json` is missing or unreadable, or `dl` does not end with `<upload>/{crate}-{version}.crate`.
pub fn check_dl(registry_dir: &Path, upload: &str, name: &str, version: &str) -> Result<()> {
    let cfg = reg_index::load_config(registry_dir)
        .context("config.json of the registry (the registry repo must be initialised first)")?;
    let dl = cfg
        .dl
        .as_str()
        .replace("%7B", "{")
        .replace("%7D", "}")
        .replace("%7b", "{")
        .replace("%7d", "}");
    let resolved = dl.replace("{crate}", name).replace("{version}", version);
    let want_tail = format!(
        "{}/{name}-{version}.crate",
        upload
            .replace("{crate}", name)
            .replace("{version}", version)
    );
    ensure!(
        resolved.ends_with(&want_tail),
        "`dl` of config.json resolves to `{resolved}`, which does not end with `{want_tail}`: the .crate would be stored where cargo does not look"
    );
    Ok(())
}

/// Add the crate to the index (format v2 where needed), put the `.crate` next to it, ONE commit.
///
/// # Errors
/// Any guard (tag run, tag = version, `dl`), the version is already in the index, git fails.
pub fn publish(a: &PublishArgs) -> Result<String> {
    let version = version_of_ref(&a.git_ref)?;
    let meta = reg_index::metadata_from_crate(&a.index_url, &a.crate_file)?;
    ensure!(
        meta.vers.to_string() == version,
        "tag v{version} does not match the version {} of `{}` (tag = version of the package)",
        meta.vers,
        meta.name
    );
    check_dl(&a.registry_dir, &a.upload, &meta.name, &version)?;
    let upload = a
        .upload
        .replace("{crate}", &meta.name)
        .replace("{version}", &version);
    let mut pkg = reg_index::add_from_crate(
        &a.registry_dir,
        &a.index_url,
        &a.crate_file,
        Some(&format!("{}/{}", a.registry_dir.display(), a.upload)),
    )
    .context("index entry")?;
    // `reg-index` writes format v1 only: put `dep:` / `?/` features into `features2` with `v: 2` (registry-index standard)
    let rel_index = index_file(&pkg.name);
    if split_v2(&mut pkg) {
        let path = a.registry_dir.join(&rel_index);
        let text = std::fs::read_to_string(&path)?;
        let mut out = String::new();
        for line in text.lines() {
            let mut p: reg_index::IndexPackage = serde_json::from_str(line)?;
            if p.vers == pkg.vers {
                p = pkg.clone();
            }
            out.push_str(&serde_json::to_string(&p)?);
            out.push('\n');
        }
        std::fs::write(&path, out)?;
    }
    // `reg-index` commits only the index file; the `.crate` goes into the SAME commit (amend, no second commit)
    let rel = format!("{upload}/{}-{version}.crate", pkg.name);
    git(
        &a.registry_dir,
        &["add", &rel, &rel_index.to_string_lossy()],
    )?;
    let subject = git_out(&a.registry_dir, &["log", "-1", "--format=%s"])?;
    let mut msg = subject.trim().to_string();
    if !a.source.is_empty() {
        let _ = write!(msg, "\n\nsource: {}", a.source);
    }
    if let Ok(run) = std::env::var("GITHUB_RUN_ID") {
        let _ = write!(msg, "\nrun: {run}");
    }
    git(
        &a.registry_dir,
        &["commit", "--amend", "-q", "--no-edit", "-m", &msg],
    )?;
    // the lock file of `reg-index` is not part of the registry
    let _ = std::fs::remove_file(a.registry_dir.join(".cargo-index-lock"));
    Ok(format!(
        "{} {} cksum {} -> {rel}",
        pkg.name, pkg.vers, pkg.cksum
    ))
}

pub(crate) fn git(dir: &Path, args: &[&str]) -> Result<()> {
    git_out(dir, args).map(|_| ())
}

pub(crate) fn git_out(dir: &Path, args: &[&str]) -> Result<String> {
    let out = Command::new("git")
        .arg("-C")
        .arg(dir)
        .args(args)
        .output()
        .context("cannot start git")?;
    ensure!(
        out.status.success(),
        "git {} failed: {}",
        args.join(" "),
        String::from_utf8_lossy(&out.stderr)
    );
    Ok(String::from_utf8_lossy(&out.stdout).into_owned())
}
