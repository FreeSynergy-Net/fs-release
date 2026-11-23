//! Pattern: Pipeline (job 2 of the pull path: only enters; no source build code, the one cargo call is `cargo metadata --no-deps` with a fixed toolchain).

use std::path::PathBuf;

use anyhow::{ensure, Context, Result};
use semver::Version;

use crate::allow::{list_tags, parse_allow, url_of, TagInfo};
use crate::guard::version_of_ref;
use crate::publish::{publish, PublishArgs};

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

fn source_line(repo: &str, t: &TagInfo, commit: &str) -> String {
    format!(
        "{repo} {} commit {commit} tag-object {}",
        t.name, t.tag_object
    )
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
        // the name inside the `.crate` is the word of the (untrusted) job `package`: it must be the package of the plan line, which is on the allow list
        let inner = reg_index::metadata_from_crate(&a.index_url, a.dir.join(file))
            .with_context(|| format!("reading `{file}`"))?;
        ensure!(
            inner.name == pkg,
            "`{file}` contains the crate `{}`, but the plan line (and the allow list) say `{pkg}`: nothing is entered",
            inner.name
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
