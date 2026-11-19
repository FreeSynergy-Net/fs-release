//! Pattern: Facade (the command `fs-blueprint fmt`: write a blueprint in its canonical form, ADR 0099 point 14).
//!
//! The canonical form is the TOML text of the parsed value (the same text the blueprint hash is made of): tables and keys in the order of the schema, the
//! order of the entries of a list kept, UTF-8, LF, one LF at the end. It cannot keep comments, so a file that has one is refused and nothing is written.

use crate::emit::dirstore::{DirStore, Rename};
use crate::emit::write::Store;
use crate::error::{Coded, Report};
use crate::model::Blueprint;
use std::io::Write;
use std::path::{Path, PathBuf};

/// True when a TOML text has a comment (a `#` outside of every kind of string).
#[must_use]
pub fn has_comment(src: &str) -> bool {
    #[derive(PartialEq)]
    enum S {
        Plain,
        Basic,
        Literal,
        MultiBasic,
        MultiLiteral,
    }
    let b: Vec<char> = src.chars().collect();
    let (mut i, mut s) = (0, S::Plain);
    let at = |i: usize, t: &str| t.chars().enumerate().all(|(k, c)| b.get(i + k) == Some(&c));
    while i < b.len() {
        match s {
            S::Plain => {
                if b[i] == '#' {
                    return true;
                }
                if at(i, "\"\"\"") {
                    (s, i) = (S::MultiBasic, i + 3);
                    continue;
                }
                if at(i, "'''") {
                    (s, i) = (S::MultiLiteral, i + 3);
                    continue;
                }
                s = match b[i] {
                    '"' => S::Basic,
                    '\'' => S::Literal,
                    _ => S::Plain,
                };
            }
            S::Basic | S::MultiBasic if b[i] == '\\' => i += 1,
            S::Basic if b[i] == '"' || b[i] == '\n' => s = S::Plain,
            S::MultiBasic if at(i, "\"\"\"") => (s, i) = (S::Plain, i + 2),
            S::Literal if b[i] == '\'' || b[i] == '\n' => s = S::Plain,
            S::MultiLiteral if at(i, "'''") => (s, i) = (S::Plain, i + 2),
            _ => {}
        }
        i += 1;
    }
    false
}

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
    let bp = Blueprint::from_toml(text)?;
    if has_comment(text) {
        return Err(Coded::new("fmt-comments").with("path", path.display().to_string()));
    }
    canonical(&bp)
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
