//! Pattern: Adapter (the crate `rpm` 0.30.2 behind one small value: `RpmSpec` in, an RPM file out; nothing else in the tool knows the crate).
//!
//! Never signs: a local build is for tests; signing is the CI (`rpmsign`), a release is built by the CI from the tag only.

use rpm::{BuildConfig, Dependency, FileOptions, PackageBuilder};
use std::path::{Path, PathBuf};

/// A file or a folder of the package, with its place on the target system.
pub struct SpecFile {
    pub dest: String,
    /// `None` makes a folder.
    pub bytes: Option<Vec<u8>>,
    pub mode: u16,
    /// A configuration file: an edit of the user is kept on update (`%config(noreplace)`).
    pub config: bool,
}

/// How a package depends on another: `Requires` (must be there) or `Recommends` (installed by default, may be left out).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum ReqKind {
    #[default]
    Requires,
    Recommends,
}

/// A dependency entry: a name, and `>= version` when a version is given.
#[derive(Default)]
pub struct Req {
    pub name: String,
    pub at_least: Option<String>,
    pub kind: ReqKind,
}

/// Everything a package needs (data: `noarch`; a module with a binary: its CPU).
pub struct RpmSpec {
    pub name: String,
    /// `noarch` for data, `x86_64` or `aarch64` for a package with a binary.
    pub arch: String,
    pub version: String,
    pub release: String,
    pub license: String,
    pub summary: String,
    pub description: String,
    pub url: Option<String>,
    pub files: Vec<SpecFile>,
    pub requires: Vec<Req>,
    /// Seconds since 1970 for a reproducible package (`SOURCE_DATE_EPOCH`); `None` = now.
    pub source_date: Option<u32>,
}

impl RpmSpec {
    /// The file name `<name>-<version>-<release>.<arch>.rpm`.
    #[must_use]
    pub fn file_name(&self) -> String {
        format!(
            "{}-{}-{}.{}.rpm",
            self.name, self.version, self.release, self.arch
        )
    }

    /// Builds the package into `out_dir` (made when missing) and returns the path.
    ///
    /// # Errors
    /// A text when the crate refuses a value or the file cannot be written.
    pub fn write_into(&self, out_dir: &Path) -> Result<PathBuf, String> {
        let e = |x: rpm::Error| format!("rpm: {x}");
        let mut config = BuildConfig::v4().compression(rpm::CompressionType::Gzip);
        if let Some(t) = self.source_date {
            config = config.source_date(t);
        }
        let mut b = PackageBuilder::new(
            &self.name,
            &self.version,
            &self.license,
            &self.arch,
            &self.summary,
        );
        b.using_config(config)
            .release(&self.release)
            .description(&self.description);
        // rpmlint E no-changelogname-tag and no-buildhost-tag: both tags must exist. The host is a fixed word (reproducible, no real host name leaks); the
        // date of the entry follows --source-date.
        let when = self.source_date.unwrap_or_else(|| {
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .map_or(0, |d| u32::try_from(d.as_secs()).unwrap_or(0))
        });
        b.build_host("fs-blueprint");
        b.add_changelog_entry(
            "FreeSynergy",
            format!(
                "- {} {}, made by fs-blueprint package",
                self.name, self.version
            ),
            when,
        );
        if let Some(u) = &self.url {
            b.url(u);
        }
        for f in &self.files {
            match &f.bytes {
                None => b
                    .with_dir_entry(FileOptions::dir(&f.dest).permissions(f.mode))
                    .map_err(e)?,
                Some(bytes) => {
                    let o = FileOptions::new(&f.dest).permissions(f.mode);
                    let o = if f.config { o.config().noreplace() } else { o };
                    b.with_file_contents(bytes.clone(), o).map_err(e)?
                }
            };
        }
        for r in &self.requires {
            let dep = match &r.at_least {
                Some(v) => Dependency::greater_eq(&r.name, v),
                None => Dependency::any(&r.name),
            };
            match r.kind {
                ReqKind::Requires => b.requires(dep),
                ReqKind::Recommends => b.recommends(dep),
            };
        }
        let pkg = b.build().map_err(e)?;
        std::fs::create_dir_all(out_dir).map_err(|x| format!("{}: {x}", out_dir.display()))?;
        let path = out_dir.join(self.file_name());
        pkg.write_file(&path).map_err(e)?;
        Ok(path)
    }
}
