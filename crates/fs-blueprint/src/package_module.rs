//! Pattern: Builder (turns a module folder, or the definition of `freesynergy-filesystem`, into an `RpmSpec`; the file format is the adapter's job).
//!
//! ENTSCHEID-install §1: a module is an arch package `fs-module-<name>` with its service below `/usr/libexec/freesynergy/modules/<name>/`.

use crate::package_rpm::{tree, valid_name, LICENSE, PROGRAMS_DIR, REPOSITORY_URL};
use crate::rpm_spec::{Req, RpmSpec, SpecFile};
use std::path::Path;

/// The package that owns the shared folders; every module and program package requires it (INST-4a).
pub const FILESYSTEM: &str = "freesynergy-filesystem";
/// Where the host looks for modules: one folder per module, `service` inside (INST-2a: one place).
pub const MODULES_LIBEXEC: &str = "/usr/libexec/freesynergy/modules";
/// Where the self-description (`AppStream` metainfo) of a module lives.
pub const MODULES_SHARE: &str = "/usr/share/freesynergy/modules";

/// The package name of a module: `fs-module-<name>`. `name` is the HOST module name (`planning`), never the repo or crate name
/// (`fs-modules-planning` would become `fs-module-modules-planning`; there is no stripping, a name with `fs-` is refused).
///
/// # Errors
/// A text when the name is unsafe or starts with `fs-`.
pub fn module_package(name: &str) -> Result<String, String> {
    if name.starts_with("fs-") {
        return Err(format!(
            "the module name {name:?} starts with fs-: use the name of the host module (planning), not the repo name"
        ));
    }
    if !valid_name(name) {
        return Err(format!(
            "the module name {name:?} is not usable as a package name"
        ));
    }
    Ok(format!("fs-module-{name}"))
}

pub(crate) fn dir(dest: &str) -> SpecFile {
    SpecFile {
        dest: dest.into(),
        bytes: None,
        mode: 0o755,
        config: false,
    }
}

/// The package `freesynergy-filesystem`: it owns the shared folders and has no files (so `rpm -qf` names a owner for each folder, `unowned-dirs.sh` stays empty). Version follows fs-blueprint.
#[must_use]
pub fn filesystem_spec(version: &str, release: &str) -> RpmSpec {
    RpmSpec {
        name: FILESYSTEM.into(),
        arch: "noarch".into(),
        version: version.into(),
        release: release.into(),
        license: LICENSE.into(),
        summary: "FreeSynergy shared folders".into(),
        description: "Owns the folders that modules, programs and frontends of FreeSynergy share."
            .into(),
        url: Some(REPOSITORY_URL.into()),
        files: [
            "/usr/libexec/freesynergy",
            MODULES_LIBEXEC,
            "/usr/share/freesynergy",
            PROGRAMS_DIR,
            MODULES_SHARE,
            "/usr/share/freesynergy/frontends",
        ]
        .iter()
        .map(|d| dir(d))
        .collect(),
        requires: Vec::new(),
        source_date: None,
    }
}

/// The CPUs a module package may be built for.
const ARCHES: [&str; 2] = ["x86_64", "aarch64"];

/// What `module_spec` needs besides the folder.
pub struct ModuleInput<'a> {
    /// Host module name (`planning`).
    pub name: &'a str,
    pub version: &'a str,
    pub release: &'a str,
    /// The built `service` binary.
    pub binary: &'a Path,
    /// `x86_64` or `aarch64`.
    pub arch: &'a str,
}

fn read(p: &Path) -> Result<Vec<u8>, String> {
    std::fs::read(p).map_err(|e| format!("{}: {e}", p.display()))
}

/// A file of the module folder that is a plain file: a link is refused (it could pull a file of the build machine into a signed package).
pub(crate) fn read_plain(p: &Path) -> Result<Vec<u8>, String> {
    let meta = std::fs::symlink_metadata(p).map_err(|e| format!("{}: {e}", p.display()))?;
    if meta.file_type().is_symlink() {
        return Err(format!("{}: a link is not packaged", p.display()));
    }
    if !meta.is_file() {
        return Err(format!("{}: not a plain file", p.display()));
    }
    read(p)
}

/// The folder `catalog` must be a real folder, not a link.
fn check_plain_dir(p: &Path) -> Result<(), String> {
    let meta = std::fs::symlink_metadata(p).map_err(|e| format!("{}: {e}", p.display()))?;
    if meta.file_type().is_symlink() {
        return Err(format!("{}: a link is not packaged", p.display()));
    }
    if !meta.is_dir() {
        return Err(format!("{}: not a folder", p.display()));
    }
    Ok(())
}

/// The package `fs-module-<name>` of a module folder (`catalog/commands.json`, `catalog/forms.json`, optional `catalog/settings.json`,
/// `metainfo.xml`, `LICENSE-MIT`, `LICENSE-APACHE`) and the built binary. Layout and modes: ENTSCHEID-install §1.
/// The binary is `service` 0755 root:root, everything else 0644; the package owns its own folders (`%dir`), the shared ones belong to `freesynergy-filesystem`.
///
/// # Errors
/// A text when the name or CPU is unusable, a required file is missing, or a link is in the folder.
pub fn module_spec(folder: &Path, i: &ModuleInput) -> Result<RpmSpec, String> {
    let pkg = module_package(i.name)?;
    if !ARCHES.contains(&i.arch) {
        return Err(format!(
            "the CPU {:?} is not supported (x86_64 | aarch64)",
            i.arch
        ));
    }
    let base = format!("{MODULES_LIBEXEC}/{}", i.name);
    let lic = format!("/usr/share/licenses/{pkg}");
    let file = |dest: String, bytes, mode| SpecFile {
        dest,
        bytes: Some(bytes),
        mode,
        config: false,
    };
    let mut files = vec![
        dir(&base),
        file(format!("{base}/service"), read(i.binary)?, 0o755),
        dir(&format!("{base}/catalog")),
    ];
    check_plain_dir(&folder.join("catalog"))?;
    for need in ["commands.json", "forms.json"] {
        if !folder.join("catalog").join(need).is_file() {
            return Err(format!("catalog/{need} is missing in {}", folder.display()));
        }
    }
    tree(
        &folder.join("catalog"),
        &format!("{base}/catalog"),
        &mut files,
    )?;
    files.push(file(
        format!("{MODULES_SHARE}/{}.metainfo.xml", i.name),
        read_plain(&folder.join("metainfo.xml"))?,
        0o644,
    ));
    files.push(dir(&lic));
    for l in ["LICENSE-MIT", "LICENSE-APACHE"] {
        files.push(file(
            format!("{lic}/{l}"),
            read_plain(&folder.join(l))?,
            0o644,
        ));
    }
    Ok(RpmSpec {
        summary: format!("FreeSynergy module {}", i.name),
        description: format!(
            "The module {} of FreeSynergy: the service, its commands and forms.",
            i.name
        ),
        name: pkg,
        arch: i.arch.into(),
        version: i.version.into(),
        release: i.release.into(),
        license: LICENSE.into(),
        url: Some(REPOSITORY_URL.into()),
        files,
        requires: vec![Req {
            name: FILESYSTEM.into(),
            at_least: None,
        }],
        source_date: None,
    })
}
