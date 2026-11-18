//! Pattern: Facade (the command `fs-blueprint new`: check the blueprint, generate the program, write it; on an error nothing is written).

use crate::cli::{print, read_catalog, Options};
use crate::emit::generate;
use crate::emit::generate::planned_paths;
use crate::emit::write::{write, DirStore};
use crate::error::Report;
use crate::kit::KitCatalog;
use crate::model::Blueprint;
use crate::repo_fs::FsRepo;
use crate::verified::verify;
use std::io::Write;
use std::path::Path;

/// Puts a file of the blueprint's folder into the program folder: made when missing, left when the same, an error when it differs (never overwritten).
fn place(from: &Path, to: &Path) -> Result<(), String> {
    let bytes = std::fs::read(from).map_err(|e| format!("cannot read {}: {e}", from.display()))?;
    match std::fs::read(to) {
        Ok(old) if old == bytes => Ok(()),
        Ok(_) => Err(format!(
            "{} exists and is not the same as {}: nothing was overwritten",
            to.display(),
            from.display()
        )),
        Err(_) => {
            if let Some(dir) = to.parent() {
                std::fs::create_dir_all(dir).map_err(|e| e.to_string())?;
            }
            std::fs::write(to, bytes).map_err(|e| format!("cannot write {}: {e}", to.display()))
        }
    }
}

/// Copies the files below `from` (no links) to `to`.
fn place_tree(from: &Path, to: &Path) -> Result<(), String> {
    let Ok(entries) = std::fs::read_dir(from) else {
        return Ok(());
    };
    let mut list: Vec<_> = entries.flatten().collect();
    list.sort_by_key(std::fs::DirEntry::file_name);
    for e in list {
        let meta = std::fs::symlink_metadata(e.path()).map_err(|e| e.to_string())?;
        let target = to.join(e.file_name());
        if meta.is_dir() {
            place_tree(&e.path(), &target)?;
        } else if meta.is_file() {
            place(&e.path(), &target)?;
        }
    }
    Ok(())
}

fn same_folder(a: &Path, b: &Path) -> bool {
    matches!((a.canonicalize(), b.canonicalize()), (Ok(x), Ok(y)) if x == y)
}

pub(crate) fn run_new(o: &Options, out: &mut dyn Write) -> Result<i32, String> {
    let [file] = o.positional.as_slice() else {
        return Err("new needs the blueprint: fs-blueprint new BAUPLAN [--out DIR]".to_string());
    };
    let catalog = read_catalog(o.catalog.as_ref())?;
    let file = Path::new(file);
    let text = std::fs::read_to_string(file)
        .map_err(|e| format!("cannot read {}: {e}", file.display()))?;
    let source = file
        .parent()
        .filter(|p| !p.as_os_str().is_empty())
        .unwrap_or_else(|| Path::new("."));
    let fail = |report: &Report, out: &mut dyn Write| {
        print(report, o.lang, out);
        Ok(1)
    };
    let bp = match Blueprint::from_toml(&text) {
        Ok(bp) => bp,
        Err(code) => {
            return fail(
                &Report {
                    errors: vec![code],
                    warnings: Vec::new(),
                },
                out,
            )
        }
    };
    let view = FsRepo::new(
        source,
        planned_paths(&bp, &catalog as &dyn KitCatalog),
        o.talks.clone(),
    );
    let verified = match verify(&bp, &catalog, &view, &o.rules) {
        Ok(v) => v,
        Err(report) => return fail(&report, out),
    };
    let artifacts = match generate(&verified, &catalog) {
        Ok(a) => a,
        Err(code) => {
            return fail(
                &Report {
                    errors: vec![code],
                    warnings: Vec::new(),
                },
                out,
            )
        }
    };
    let target = o
        .out_dir
        .clone()
        .unwrap_or_else(|| format!("fs-{}", bp.program.name).into());
    let outcome = match write(&artifacts, &mut DirStore::new(&target)) {
        Ok(outcome) => outcome,
        Err(errors) => {
            return fail(
                &Report {
                    errors,
                    warnings: Vec::new(),
                },
                out,
            )
        }
    };
    if !same_folder(source, &target) {
        place(file, &target.join("scaffold.toml"))?;
        place_tree(&source.join("forms"), &target.join("forms"))?;
    }
    drop(writeln!(
        out,
        "fs-blueprint: made {} (written {}, kept {}); next: cd {} && cargo build",
        bp.program.name,
        outcome.written.len(),
        outcome.kept.len(),
        target.display()
    ));
    Ok(0)
}
