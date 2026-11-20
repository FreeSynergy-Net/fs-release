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
use std::path::{Path, PathBuf};

/// A file of the blueprint's folder and the place it goes to in the program folder.
type Place = (PathBuf, PathBuf);

/// Every file that goes with the blueprint (the blueprint itself and the files below `forms/`), no links, in a fixed order.
fn list_places(file: &Path, source: &Path, target: &Path) -> Vec<Place> {
    fn tree(from: &Path, to: &Path, out: &mut Vec<Place>) {
        let Ok(entries) = std::fs::read_dir(from) else {
            return;
        };
        let mut list: Vec<_> = entries.flatten().collect();
        list.sort_by_key(std::fs::DirEntry::file_name);
        for e in list {
            let Ok(meta) = std::fs::symlink_metadata(e.path()) else {
                continue;
            };
            let to = to.join(e.file_name());
            if meta.is_dir() {
                tree(&e.path(), &to, out);
            } else if meta.is_file() {
                out.push((e.path(), to));
            }
        }
    }
    let mut out = vec![(file.to_path_buf(), target.join("scaffold.toml"))];
    tree(&source.join("forms"), &target.join("forms"), &mut out);
    out
}

/// Before anything is written: a place that holds another content (or something that cannot be compared) is a conflict, and nothing is overwritten.
fn conflicts(places: &[Place]) -> Result<(), String> {
    for (from, to) in places {
        if std::fs::symlink_metadata(to).is_err() {
            continue;
        }
        let same = match (std::fs::read(from), std::fs::read(to)) {
            (Ok(a), Ok(b)) => a == b,
            _ => false,
        };
        if !same {
            return Err(format!(
                "{} exists and is not the same as {}",
                to.display(),
                from.display()
            ));
        }
    }
    Ok(())
}

/// Puts the files in place (made when missing, left when the same).
fn apply(places: &[Place]) -> Result<(), String> {
    // the same look again: nothing is ever overwritten, whatever the caller did before; at this point the program files are already written
    conflicts(places).map_err(|e| {
        format!("{e}: this file was left as it is, but the program files are already written")
    })?;
    for (from, to) in places {
        let bytes =
            std::fs::read(from).map_err(|e| format!("cannot read {}: {e}", from.display()))?;
        if std::fs::read(to).is_ok_and(|old| old == bytes) {
            continue;
        }
        if let Some(dir) = to.parent() {
            std::fs::create_dir_all(dir).map_err(|e| e.to_string())?;
        }
        std::fs::write(to, bytes).map_err(|e| format!("cannot write {}: {e}", to.display()))?;
    }
    Ok(())
}

fn same_folder(a: &Path, b: &Path) -> bool {
    matches!((a.canonicalize(), b.canonicalize()), (Ok(x), Ok(y)) if x == y)
}

/// `new` (the words AFTER `new`) with a hook that runs between the look at the conflicts and the first write (a test makes a file appear there: the race the look cannot prevent).
///
/// # Errors
/// The words are wrong, or a file cannot be read, or a place is taken (the text says whether anything was written).
pub fn new_with_hook(
    words: &[String],
    out: &mut dyn Write,
    between: &dyn Fn(),
) -> Result<i32, String> {
    let o = crate::cli::parse(words)?;
    run_new_with(&o, out, between)
}

pub(crate) fn run_new(o: &Options, out: &mut dyn Write) -> Result<i32, String> {
    run_new_with(o, out, &|| {})
}

fn run_new_with(o: &Options, out: &mut dyn Write, between: &dyn Fn()) -> Result<i32, String> {
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
    // everything that could conflict is looked at BEFORE the first byte is written: on an error the target is as it was
    let places = if same_folder(source, &target) {
        Vec::new()
    } else {
        list_places(file, source, &target)
    };
    conflicts(&places).map_err(|e| format!("{e}: nothing was written"))?;
    between();
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
    apply(&places)?;
    drop(writeln!(
        out,
        "fs-blueprint: made {} (written {}, kept {}); a data package, no code; next: commit {}",
        bp.program.name,
        outcome.written.len(),
        outcome.kept.len(),
        target.display()
    ));
    Ok(0)
}
