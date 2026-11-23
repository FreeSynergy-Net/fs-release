//! Pattern: Pipeline (pull path, ADR 0093): `scan` = job 1 (reads, packages, checks, writes a plan), `apply` = job 2 (only enters; no cargo).
//!
//! The allow list lives in the registry repo itself (`allowed-sources.txt`): one line per source repo, `owner/repo <first-version> <package>...`, the
//! packages in dependency order (a package may depend on the ones before it). A change of that file needs a review (CODEOWNERS, docs/crate-index.md).

use std::path::{Path, PathBuf};
use std::process::Command;

use anyhow::{bail, ensure, Context, Result};
use semver::Version;

use crate::guard::version_of_ref;
use crate::publish::{git_out, package, publish, PublishArgs};

/// One line of the allow list.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Source {
    pub repo: String,
    pub from: Version,
    pub packages: Vec<String>,
}

/// One tag of a source repo as `git ls-remote --tags` shows it.
#[derive(Debug, Clone)]
pub struct TagInfo {
    pub name: String,
    pub version: Version,
    pub tag_object: String,
    /// The commit the tag points to (`^{}` line); `None` = a lightweight tag.
    pub commit: Option<String>,
}

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
    /// `cargo-semver-checks` binary (pinned, checksum checked by the workflow). Without it the gate is reported as NOT RUN.
    #[arg(long)]
    pub semver_checks: Option<PathBuf>,
}

/// The arguments of `apply`.
#[derive(clap::Args, Clone)]
pub struct ApplyArgs {
    #[arg(long)]
    pub allow: PathBuf,
    /// The `--out` directory of `scan`.
    #[arg(long)]
    pub dir: PathBuf,
    #[arg(long)]
    pub registry_dir: PathBuf,
    #[arg(long)]
    pub index_url: String,
    #[arg(long, default_value = "https://github.com")]
    pub source_base: String,
}

/// Result of a scan: how many crates are in the plan and the first problem (the scan stops there; earlier tags stay in the plan).
#[derive(Debug)]
pub struct ScanReport {
    pub entries: usize,
    pub notes: Vec<String>,
    pub error: Option<String>,
}

/// Parses the allow list.
///
/// # Errors
/// A line has no repo, no first version or no package, or a name has unexpected characters.
pub fn parse_allow(text: &str) -> Result<Vec<Source>> {
    let mut out = Vec::new();
    for (n, raw) in text.lines().enumerate() {
        let line = raw.split('#').next().unwrap_or("").trim();
        if line.is_empty() {
            continue;
        }
        let mut it = line.split_whitespace();
        let repo = it.next().unwrap_or_default();
        let from = it
            .next()
            .with_context(|| format!("line {}: first version missing", n + 1))?;
        let packages: Vec<String> = it.map(str::to_string).collect();
        let name_ok = |s: &str| {
            !s.is_empty()
                && s.bytes()
                    .all(|b| b.is_ascii_alphanumeric() || b"-_.".contains(&b))
        };
        let repo_ok = repo
            .split_once('/')
            .is_some_and(|(o, r)| name_ok(o) && name_ok(r) && !r.contains('/'));
        ensure!(repo_ok, "line {}: `{repo}` is not owner/repo", n + 1);
        ensure!(!packages.is_empty(), "line {}: no package", n + 1);
        ensure!(
            packages.iter().all(|p| name_ok(p)),
            "line {}: bad package name",
            n + 1
        );
        out.push(Source {
            repo: repo.to_string(),
            from: Version::parse(from).with_context(|| format!("line {}: `{from}`", n + 1))?,
            packages,
        });
    }
    Ok(out)
}

fn url_of(base: &str, repo: &str) -> String {
    format!("{}/{repo}.git", base.trim_end_matches('/'))
}

/// All release tags `vX.Y.Z` of a repo, oldest first (other tags are ignored).
///
/// # Errors
/// `git ls-remote` fails.
pub fn list_tags(url: &str) -> Result<Vec<TagInfo>> {
    let out = Command::new("git")
        .args(["ls-remote", "--tags", url])
        .output()
        .context("cannot start git")?;
    ensure!(
        out.status.success(),
        "git ls-remote {url} failed: {}",
        String::from_utf8_lossy(&out.stderr)
    );
    let text = String::from_utf8_lossy(&out.stdout);
    let mut tags: Vec<TagInfo> = Vec::new();
    for line in text.lines() {
        let Some((sha, r)) = line.split_once('\t') else {
            continue;
        };
        let (r, peeled) = r.strip_suffix("^{}").map_or((r, false), |x| (x, true));
        let Ok(v) = version_of_ref(r) else { continue };
        let Ok(version) = Version::parse(&v) else {
            continue;
        };
        let name = r.trim_start_matches("refs/tags/").to_string();
        if peeled {
            if let Some(t) = tags.iter_mut().find(|t| t.name == name) {
                t.commit = Some(sha.to_string());
            }
        } else {
            tags.push(TagInfo {
                name,
                version,
                tag_object: sha.to_string(),
                commit: None,
            });
        }
    }
    tags.sort_by(|a, b| a.version.cmp(&b.version));
    Ok(tags)
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

fn source_line(repo: &str, t: &TagInfo, commit: &str) -> String {
    format!(
        "{repo} {} commit {commit} tag-object {}",
        t.name, t.tag_object
    )
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
        semver_gate(a, &dir, p, &t.version, rep)?;
        let fname = f.file_name().unwrap().to_string_lossy().into_owned();
        std::fs::copy(f, a.out.join(&fname))?;
        plan.push_str(&format!(
            "{}\t{}\t{}\t{commit}\t{p}\t{}\t{fname}\n",
            s.repo, t.name, t.tag_object, t.version
        ));
        rep.entries += 1;
    }
    Ok(())
}

/// Gate BEFORE the entry: breaking change without a version step. Baseline = the newest older `.crate` in the registry checkout itself.
fn semver_gate(
    a: &ScanArgs,
    src: &Path,
    pkg: &str,
    version: &Version,
    rep: &mut ScanReport,
) -> Result<()> {
    let older = reg_index::list(&a.registry_dir, pkg, None)?
        .into_iter()
        .filter(|p| !p.yanked && &p.vers < version)
        .map(|p| p.vers)
        .max();
    let Some(base) = older else {
        rep.notes.push(format!(
            "semver {pkg} {version}: first version in the registry, no baseline"
        ));
        return Ok(());
    };
    let Some(bin) = &a.semver_checks else {
        rep.notes.push(format!(
            "semver {pkg} {version}: NOT RUN (no --semver-checks)"
        ));
        return Ok(());
    };
    let krate = a
        .registry_dir
        .join(format!("crates/{pkg}/{pkg}-{base}.crate"));
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
        .current_dir(src)
        .output()
        .context("cannot start cargo-semver-checks")?;
    ensure!(
        out.status.success(),
        "semver gate RED for {pkg} {version} against {base}:\n{}{}",
        String::from_utf8_lossy(&out.stdout),
        String::from_utf8_lossy(&out.stderr)
    );
    rep.notes
        .push(format!("semver {pkg} {version} against {base}: green"));
    Ok(())
}

/// Job 2: enters the planned crates one after the other (dependency order, one commit each). Re-checks every line against the allow list and the tag.
/// No push here (the workflow pushes once).
///
/// # Errors
/// A plan line is not allowed, the tag moved between the jobs, or `publish` refuses.
pub fn apply(a: &ApplyArgs) -> Result<Vec<String>> {
    let sources = parse_allow(&std::fs::read_to_string(&a.allow).context("allow list")?)?;
    let plan = std::fs::read_to_string(a.dir.join("plan.txt")).context("plan.txt")?;
    let mut done = Vec::new();
    for line in plan.lines() {
        let f: Vec<&str> = line.split('\t').collect();
        ensure!(f.len() == 7, "bad plan line `{line}`");
        let (repo, tag, tag_obj, commit, pkg, version, file) =
            (f[0], f[1], f[2], f[3], f[4], f[5], f[6]);
        let s = sources
            .iter()
            .find(|s| s.repo == repo && s.packages.iter().any(|p| p == pkg))
            .with_context(|| format!("`{repo}` / `{pkg}` is not on the allow list"))?;
        ensure!(
            version_of_ref(&format!("refs/tags/{tag}"))? == version,
            "tag {tag} is not version {version}"
        );
        ensure!(
            Version::parse(version)? >= s.from,
            "{tag} is older than the first version of the allow list"
        );
        ensure!(
            file == format!("{pkg}-{version}.crate"),
            "unexpected file name `{file}`"
        );
        let now = list_tags(&url_of(&a.source_base, repo))?
            .into_iter()
            .find(|t| t.name == tag)
            .with_context(|| format!("tag {tag} of {repo} is gone"))?;
        ensure!(
            now.tag_object == tag_obj && now.commit.as_deref() == Some(commit),
            "tag {tag} of {repo} moved between the jobs"
        );
        let msg = publish(&PublishArgs {
            git_ref: format!("refs/tags/{tag}"),
            crate_file: a.dir.join(file),
            registry_dir: a.registry_dir.clone(),
            index_url: a.index_url.clone(),
            upload: "crates/{crate}".into(),
            source: source_line(repo, &now, commit),
        })?;
        done.push(msg);
    }
    Ok(done)
}
