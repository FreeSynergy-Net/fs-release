//! Pattern: Facade (the command `fs-blueprint fmt`: write a blueprint in its canonical form, ADR 0099 point 14).
//!
//! The canonical form: tables and keys in the order of the schema (the order of the canonical text, the text the blueprint hash is made of), the entries of a list in the
//! author's order, UTF-8, LF, one LF at the end. Comments and the form of every value are kept (`fmt_order`): a comment belongs to the line below it and moves with it.
//! Before a file is written the result is read again: it must be the same blueprint (same hash), else nothing is written (`fmt-unsafe`).

use crate::emit::dirstore::{DirStore, Rename};
use crate::emit::write::Store;
use crate::error::{Coded, Report};
use crate::model::Blueprint;
use std::io::Write;
use std::path::{Path, PathBuf};

/// The canonical text of a blueprint.
///
/// # Errors
/// `blueprint-unserializable` (does not happen for a parsed blueprint).
pub fn canonical(bp: &Blueprint) -> Result<String, Coded> {
    let mut text = toml::to_string(bp).map_err(|_| Coded::new("blueprint-unserializable"))?;
    text.truncate(text.trim_end_matches('\n').len());
    text.push('\n');
    Ok(text)
}

/// What `fmt` found out about one file: the canonical text, or the finding that stops it.
fn judge(path: &Path, text: &str) -> Result<String, Coded> {
    judge_with(path, text, crate::fmt_order::in_order)
}

/// [`judge`] with the function that puts the text in order (a test stands in one that breaks the blueprint: the result is read again and refused).
///
/// # Errors
/// The code of the blueprint (it does not load), or `fmt-unsafe{path}` when the ordered text is no TOML or is another blueprint.
pub fn judge_with(
    path: &Path,
    text: &str,
    order: impl Fn(&str, &str) -> Option<String>,
) -> Result<String, Coded> {
    let bp = Blueprint::from_toml(text)?;
    let unsafe_form = || Coded::new("fmt-unsafe").with("path", path.display().to_string());
    let out = order(text, &canonical(&bp)?).ok_or_else(unsafe_form)?;
    match Blueprint::from_toml(&out) {
        Ok(again) if again == bp => Ok(out),
        _ => Err(unsafe_form()),
    }
}

/// Checks (`check`) or writes the canonical form of one file; the write is atomic (a temporary file next to it, then a rename). Returns the exit code.
///
/// # Errors
/// The file cannot be read, or the new content cannot be moved into place (the old file stays).
pub fn fmt_file(
    path: &Path,
    check: bool,
    lang: &str,
    rename: Rename,
    out: &mut dyn Write,
) -> Result<i32, String> {
    let text = std::fs::read_to_string(path)
        .map_err(|e| format!("cannot read {}: {e}", path.display()))?;
    let verdict = judge(path, &text).and_then(|canon| {
        if canon == text {
            Ok(None)
        } else if check {
            Err(Coded::new("fmt-changed").with("path", path.display().to_string()))
        } else {
            Ok(Some(canon))
        }
    });
    match verdict {
        Ok(None) => Ok(0),
        Ok(Some(canon)) => {
            let dir = path
                .parent()
                .filter(|p| !p.as_os_str().is_empty())
                .unwrap_or(Path::new("."));
            let name = path
                .file_name()
                .map(|n| n.to_string_lossy().into_owned())
                .ok_or("no file name")?;
            DirStore::with_rename(dir, rename)
                .put_all(&[(name.as_str(), canon.as_str())])
                .map_err(|p| format!("cannot write {p}"))?;
            Ok(0)
        }
        Err(code) => {
            let report = Report {
                errors: vec![code],
                warnings: Vec::new(),
            };
            crate::cli::print(&report, lang, out);
            Ok(1)
        }
    }
}

/// `fmt [FILE] [--check] [--lang en|de]`: FILE defaults to `scaffold.toml`.
///
/// # Errors
/// A wrong word, or a file that cannot be read or written.
pub(crate) fn run_fmt(words: &[String], out: &mut dyn Write) -> Result<i32, String> {
    let (mut file, mut check, mut lang) = (None::<PathBuf>, false, "en");
    let mut it = words.iter();
    while let Some(w) = it.next() {
        match w.as_str() {
            "--check" => check = true,
            "--lang" => {
                lang = match it.next().map(String::as_str) {
                    Some("en") => "en",
                    Some("de") => "de",
                    _ => return Err("--lang needs en or de".to_string()),
                };
            }
            other if !other.starts_with("--") && file.is_none() => {
                file = Some(PathBuf::from(other));
            }
            other => return Err(format!("unknown word {other}")),
        }
    }
    let path = file.unwrap_or_else(|| PathBuf::from("scaffold.toml"));
    fmt_file(
        &path,
        check,
        lang,
        |from, to| std::fs::rename(from, to),
        out,
    )
}
