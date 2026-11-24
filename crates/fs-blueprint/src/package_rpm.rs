//! Pattern: Builder (turns a checked blueprint folder, or the definition of `freesynergy-release`, into an `RpmSpec`; the file format is the adapter's job).
//!
//! ADR 0094 table (b): a program is a noarch package `fs-<name>` with its data below `/usr/share/freesynergy/programs/<name>/`, a `.desktop` entry
//! (`Exec=fs-starter <name>`), and `Requires` taken from the blueprint (`fs-starter`, and `fs-module-<x> >= version` per `[[module]]`).

use crate::emit::slot::pascal;
use crate::model::Blueprint;
use crate::rpm_spec::{Req, RpmSpec, SpecFile};
use std::fmt::Write as _;
use std::path::Path;

pub use crate::package_module::{
    filesystem_spec, module_package, module_spec, ModuleInput, FILESYSTEM, MODULES_LIBEXEC,
    MODULES_SHARE,
};
pub use crate::release_args::{check_fingerprint, checked_key, checked_pages_url};

/// Where the data of programs lives on the target system.
pub const PROGRAMS_DIR: &str = "/usr/share/freesynergy/programs";
/// The default licence of the project (`workspace.package.license`).
pub const LICENSE: &str = "MIT OR Apache-2.0";
/// The address of the data repo that serves the dnf source (decision 2026-11-23: FreeSynergy-Net/Repository).
pub const REPOSITORY_URL: &str = "https://github.com/FreeSynergy-Net/Repository";
/// The Pages address of that repo. NOT KNOWN YET (Pages is switched on in order 4): a placeholder, `release_spec` refuses to build with it.
pub const PAGES_URL: &str = "PAGES-URL-NOT-SET";
/// The place of the public key on the target system.
pub const KEY_PATH: &str = "/etc/pki/rpm-gpg/RPM-GPG-KEY-freesynergy";
/// The place of the dnf source on the target system.
pub const REPO_FILE: &str = "/etc/yum.repos.d/freesynergy.repo";

/// A program name that is safe as a file name and as a package name.
pub(crate) fn valid_name(n: &str) -> bool {
    n.split('-').all(|p| {
        !p.is_empty()
            && p.chars()
                .all(|c| c.is_ascii_lowercase() || c.is_ascii_digit())
    })
}

pub(crate) fn tree(from: &Path, to: &str, out: &mut Vec<SpecFile>) -> Result<(), String> {
    let mut list: Vec<_> = std::fs::read_dir(from)
        .map_err(|e| format!("{}: {e}", from.display()))?
        .flatten()
        .collect();
    list.sort_by_key(std::fs::DirEntry::file_name);
    for e in list {
        let meta = std::fs::symlink_metadata(e.path()).map_err(|x| x.to_string())?;
        let name = e.file_name().to_string_lossy().into_owned();
        let dest = format!("{to}/{name}");
        if meta.file_type().is_symlink() {
            return Err(format!("{}: a link is not packaged", e.path().display()));
        } else if meta.is_dir() {
            out.push(SpecFile {
                dest: dest.clone(),
                bytes: None,
                mode: 0o755,
                config: false,
            });
            tree(&e.path(), &dest, out)?;
        } else {
            let bytes = std::fs::read(e.path()).map_err(|x| x.to_string())?;
            out.push(SpecFile {
                dest,
                bytes: Some(bytes),
                mode: 0o644,
                config: false,
            });
        }
    }
    Ok(())
}

/// The `.desktop` entry: `fs-starter <name>` starts the program (ADR 0094).
fn desktop(bp: &Blueprint) -> String {
    let p = &bp.program;
    let line = |t: &Option<String>| {
        t.as_deref()
            .unwrap_or_default()
            .split_whitespace()
            .collect::<Vec<_>>()
            .join(" ")
    };
    let mut s = format!(
        "[Desktop Entry]\nType=Application\nName={}\n",
        pascal(&p.name)
    );
    let _ = write!(
        s,
        "Comment={}\nComment[de]={}\nExec=fs-starter {}\nCategories={};\n",
        line(&p.summary.en),
        line(&p.summary.de),
        p.name,
        p.categories
            .iter()
            .map(|c| line(&Some(c.clone())))
            .collect::<Vec<_>>()
            .join(";")
    );
    if p.icon.is_some() {
        let _ = writeln!(s, "Icon={}", line(&p.icon));
    }
    s
}

/// The package `fs-<name>` of a blueprint folder (`scaffold.toml` and `forms/`). `starter_min` is the version of `fs-starter` the program needs; unknown = none.
///
/// # Errors
/// A text when the name is unsafe, a file cannot be read or a link is in the folder.
pub fn program_spec(
    bp: &Blueprint,
    folder: &Path,
    starter_min: Option<&str>,
    release: &str,
) -> Result<RpmSpec, String> {
    let p = &bp.program;
    if !valid_name(&p.name) {
        return Err(format!(
            "the program name {:?} is not usable as a package name",
            p.name
        ));
    }
    let base = format!("{PROGRAMS_DIR}/{}", p.name);
    let file = |dest: String, bytes: Vec<u8>, mode| SpecFile {
        dest,
        bytes: Some(bytes),
        mode,
        config: false,
    };
    let scaffold = std::fs::read(folder.join("scaffold.toml")).map_err(|e| e.to_string())?;
    let mut files = vec![
        SpecFile {
            dest: base.clone(),
            bytes: None,
            mode: 0o755,
            config: false,
        },
        file(format!("{base}/scaffold.toml"), scaffold, 0o644),
    ];
    if folder.join("forms").is_dir() {
        files.push(SpecFile {
            dest: format!("{base}/forms"),
            bytes: None,
            mode: 0o755,
            config: false,
        });
        tree(&folder.join("forms"), &format!("{base}/forms"), &mut files)?;
    }
    let app = format!(
        "/usr/share/applications/net.freesynergy.{}.desktop",
        pascal(&p.name)
    );
    files.push(file(app, desktop(bp).into_bytes(), 0o644));
    let mut requires = vec![
        Req {
            name: "fs-starter".into(),
            at_least: starter_min.map(Into::into),
        },
        Req {
            name: FILESYSTEM.into(),
            at_least: None,
        },
    ];
    for m in &bp.module {
        requires.push(Req {
            name: module_package(&m.name)?,
            at_least: Some(m.version.clone()),
        });
    }
    let text = |t: &Option<String>| {
        t.as_deref()
            .unwrap_or_default()
            .split_whitespace()
            .collect::<Vec<_>>()
            .join(" ")
    };
    Ok(RpmSpec {
        name: format!("fs-{}", p.name),
        arch: "noarch".into(),
        version: p.version.clone(),
        release: release.into(),
        license: LICENSE.into(),
        summary: text(&p.summary.en),
        description: text(&p.description.en),
        url: Some(REPOSITORY_URL.into()),
        files,
        requires,
        source_date: None,
    })
}

/// The dnf source: ONE section `[freesynergy]`, on (decision 9f, 2026-11-23: one channel, it holds pre-releases too; no nightly section, no `enabled=0`). Packages and the repo data are checked (`gpgcheck`, `repo_gpgcheck`).
#[must_use]
pub fn repo_file(pages: &str) -> String {
    format!(
        "[freesynergy]\nname=FreeSynergy\nbaseurl={pages}/rpm/stable\nenabled=1\ngpgcheck=1\nrepo_gpgcheck=1\ngpgkey=file://{KEY_PATH}\n"
    )
}

/// The package `freesynergy-release`: the dnf source and the PUBLIC key. Its version follows the fs-blueprint version; when the key or the URL changes, `--pkg-version` MUST be raised (else dnf sees no update).
///
/// # Errors
/// A text when `pages` is refused (see `checked_pages_url`), or `key` is not an armored public key with the expected fingerprint (a secret key must never end up in a package).
pub fn release_spec(
    pages: &str,
    key: &[u8],
    fingerprint: &str,
    version: &str,
    release: &str,
) -> Result<RpmSpec, String> {
    let pages = checked_pages_url(pages)?;
    let pages = pages.as_str();
    let key = checked_key(key, fingerprint)?;
    let cfg = |dest: &str, bytes: Vec<u8>| SpecFile {
        dest: dest.into(),
        bytes: Some(bytes),
        mode: 0o644,
        config: true,
    };
    Ok(RpmSpec {
        name: "freesynergy-release".into(),
        arch: "noarch".into(),
        version: version.into(),
        release: release.into(),
        license: LICENSE.into(),
        summary: "FreeSynergy package source and signing key".into(),
        description: "The package source of FreeSynergy (one channel; it holds pre-releases too).\nAnd the public key that signs the packages and the repo data.".into(),
        url: Some(REPOSITORY_URL.into()),
        files: vec![
            cfg(REPO_FILE, repo_file(pages).into_bytes()),
            SpecFile { dest: KEY_PATH.into(), bytes: Some(key), mode: 0o644, config: false },
        ],
        requires: Vec::new(),
        source_date: None,
    })
}
