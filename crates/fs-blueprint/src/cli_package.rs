//! Pattern: Facade (the command `fs-blueprint package`: `rpm BAUPLAN` makes the noarch RPM of a program, `release --key FILE` the package of the dnf source).

use crate::cli::{print, read_catalog, Options};
use crate::emit::generate::planned_paths;
use crate::kit::KitCatalog;
use crate::model::Blueprint;
use crate::package_rpm::{
    filesystem_spec, module_spec, program_spec, release_spec, ModuleInput, PAGES_URL,
};
use crate::repo_fs::FsRepo;
use crate::rpm_spec::RpmSpec;
use crate::verified::verify;
use std::io::Write;
use std::path::{Path, PathBuf};

/// The words that belong to `package` only; taken out before the general options.
struct Extra {
    starter_min: Option<String>,
    release: String,
    pages: String,
    key: Option<PathBuf>,
    fingerprint: Option<String>,
    version: Option<String>,
    source_date: Option<u32>,
    binary: Option<PathBuf>,
    name: Option<String>,
    arch: Option<String>,
}

fn split(words: &[String]) -> Result<(Extra, Vec<String>), String> {
    let mut x = Extra {
        starter_min: None,
        release: "1".into(),
        pages: PAGES_URL.into(),
        key: None,
        fingerprint: None,
        version: None,
        source_date: std::env::var("SOURCE_DATE_EPOCH")
            .ok()
            .and_then(|v| v.parse().ok()),
        binary: None,
        name: None,
        arch: None,
    };
    let (mut rest, mut it) = (Vec::new(), words.iter());
    while let Some(w) = it.next() {
        let mut v = || {
            it.next()
                .cloned()
                .ok_or_else(|| format!("{w} needs a value"))
        };
        match w.as_str() {
            "--starter-min" => x.starter_min = Some(v()?),
            "--rpm-release" => x.release = v()?,
            "--pages-url" => x.pages = v()?,
            "--key" => x.key = Some(PathBuf::from(v()?)),
            "--fingerprint" => x.fingerprint = Some(v()?),
            "--pkg-version" => x.version = Some(v()?),
            "--binary" => x.binary = Some(PathBuf::from(v()?)),
            "--name" => x.name = Some(v()?),
            "--arch" => x.arch = Some(v()?),
            "--source-date" => {
                x.source_date = Some(
                    v()?.parse()
                        .map_err(|_| "--source-date needs seconds".to_string())?,
                );
            }
            _ => rest.push(w.clone()),
        }
    }
    Ok((x, rest))
}

fn finish(mut spec: RpmSpec, x: &Extra, o: &Options, out: &mut dyn Write) -> Result<i32, String> {
    spec.source_date = x.source_date;
    let dir = o.out_dir.clone().unwrap_or_else(|| PathBuf::from("."));
    let path = spec.write_into(&dir)?;
    drop(writeln!(
        out,
        "fs-blueprint: made {} (not signed: signing is the CI)",
        path.display()
    ));
    Ok(0)
}

fn rpm(x: &Extra, o: &Options, out: &mut dyn Write) -> Result<i32, String> {
    let [file] = o.positional.as_slice() else {
        return Err(
            "package rpm needs the blueprint: fs-blueprint package rpm BAUPLAN [--out DIR]".into(),
        );
    };
    let catalog = read_catalog(o.catalog.as_ref())?;
    let file = Path::new(file);
    let text = std::fs::read_to_string(file)
        .map_err(|e| format!("cannot read {}: {e}", file.display()))?;
    let folder = file
        .parent()
        .filter(|p| !p.as_os_str().is_empty())
        .unwrap_or_else(|| Path::new("."));
    let bp = match Blueprint::from_toml(&text) {
        Ok(bp) => bp,
        Err(code) => {
            print(
                &crate::error::Report {
                    errors: vec![code],
                    warnings: Vec::new(),
                },
                o.lang,
                out,
            );
            return Ok(1);
        }
    };
    let repo = FsRepo::new(
        folder,
        planned_paths(&bp, &catalog as &dyn KitCatalog),
        o.talks.clone(),
    );
    if let Err(report) = verify(&bp, &catalog, &repo, &o.rules) {
        print(&report, o.lang, out);
        return Ok(1);
    }
    finish(
        program_spec(&bp, folder, x.starter_min.as_deref(), &x.release)?,
        x,
        o,
        out,
    )
}

fn release(x: &Extra, o: &Options, out: &mut dyn Write) -> Result<i32, String> {
    let key = x
        .key
        .as_ref()
        .ok_or("package release needs --key FILE (the armored PUBLIC key)")?;
    let bytes = std::fs::read(key).map_err(|e| format!("cannot read {}: {e}", key.display()))?;
    let fp = x.fingerprint.as_deref().ok_or(
        "package release needs --fingerprint <40 hex> (the expected fingerprint of the key)",
    )?;
    let version = x.version.as_deref().unwrap_or(env!("CARGO_PKG_VERSION"));
    finish(
        release_spec(&x.pages, &bytes, fp, version, &x.release)?,
        x,
        o,
        out,
    )
}

fn filesystem(x: &Extra, o: &Options, out: &mut dyn Write) -> Result<i32, String> {
    let version = x.version.as_deref().unwrap_or(env!("CARGO_PKG_VERSION"));
    finish(filesystem_spec(version, &x.release), x, o, out)
}

fn module(x: &Extra, o: &Options, out: &mut dyn Write) -> Result<i32, String> {
    let [folder] = o.positional.as_slice() else {
        return Err("package module needs the module folder: fs-blueprint package module FOLDER --name N --pkg-version V --binary FILE [--arch CPU] [--out DIR]".into());
    };
    let name = x
        .name
        .as_deref()
        .ok_or("package module needs --name (the host module name)")?;
    let version = x
        .version
        .as_deref()
        .ok_or("package module needs --pkg-version")?;
    let binary = x
        .binary
        .as_deref()
        .ok_or("package module needs --binary FILE (the built service)")?;
    let arch = x.arch.as_deref().unwrap_or(std::env::consts::ARCH);
    let input = ModuleInput {
        name,
        version,
        release: &x.release,
        binary,
        arch,
    };
    finish(module_spec(Path::new(folder), &input)?, x, o, out)
}

/// `package` (the words AFTER `package`). Exit codes as the other commands: 0 made, 1 the blueprint has findings, 2 wrong words or an error.
///
/// # Errors
/// The words are wrong, a file cannot be read or the package cannot be made.
pub fn run_package(words: &[String], out: &mut dyn Write) -> Result<i32, String> {
    let Some((kind, rest)) = words.split_first() else {
        return Err("package needs a kind: rpm | release | filesystem | module".into());
    };
    let (x, rest) = split(rest)?;
    let o = crate::cli::parse(&rest)?;
    match kind.as_str() {
        "rpm" => rpm(&x, &o, out),
        "release" => release(&x, &o, out),
        "filesystem" => filesystem(&x, &o, out),
        "module" => module(&x, &o, out),
        other => Err(format!(
            "unknown package kind {other} (rpm | release | filesystem | module)"
        )),
    }
}
