//! The text source files: one `<language>.mf2` per language, many messages.
//!
//! Pattern: Value object + a small parser. `MessageFormat` 2 defines the message, not a file of messages, so the container is the plainest one
//! (the format `fs-components-mf2` reads today, moved here with the build-time check, ADR 0057): `id = message`, a message goes on over the
//! following lines when they are indented, lines starting with `#` and blank lines between messages are skipped. An id is letters, digits, `-`, `_`, `.`.

use std::path::Path;

use crate::error::PackError;
use crate::locale;

/// The text of one language file with its canonical language tag (BCP 47, for example `de` or `de-AT`).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Source {
    /// The canonical tag (from the file name `de.mf2`).
    pub language: String,
    /// The content: `id = message` entries.
    pub text: String,
}

/// An entry: id, message (MF2) and the 1-based line it starts on.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct Entry {
    pub id: String,
    pub message: String,
    pub line: usize,
}

fn id_ok(id: &str) -> bool {
    !id.is_empty()
        && id
            .bytes()
            .all(|b| b.is_ascii_alphanumeric() || matches!(b, b'-' | b'_' | b'.'))
}

/// The entries of a text and the problems on the way (a line that is no entry, an id given twice, an empty message).
pub(crate) fn entries(text: &str) -> (Vec<Entry>, Vec<String>) {
    let (mut out, mut problems): (Vec<Entry>, Vec<String>) = (Vec::new(), Vec::new());
    let text = text.strip_prefix('\u{feff}').unwrap_or(text);
    for (i, raw) in text.lines().enumerate() {
        let n = i + 1;
        let continued = raw.starts_with([' ', '\t']) && !raw.trim().is_empty();
        let line = raw.trim_end();
        if continued {
            match out.last_mut() {
                Some(last) => {
                    last.message.push('\n');
                    last.message.push_str(line.trim_start());
                }
                None => problems.push(format!("line {n}: indented line before any entry")),
            }
        } else if line.is_empty() || line.starts_with('#') {
            // a blank line or a comment: nothing to take
        } else if let Some((id, rest)) = line.split_once('=') {
            let id = id.trim();
            if !id_ok(id) {
                problems.push(format!("line {n}: `{id}` is no identifier"));
            } else if out.iter().any(|e| e.id == id) {
                problems.push(format!("line {n}: {id} is given twice"));
            } else {
                out.push(Entry {
                    id: id.to_owned(),
                    message: rest.trim().to_owned(),
                    line: n,
                });
            }
        } else {
            problems.push(format!("line {n}: no `=`"));
        }
    }
    for e in &out {
        if e.message.trim().is_empty() {
            problems.push(format!("line {}: {} has no message", e.line, e.id));
        }
    }
    (out, problems)
}

/// All `*.mf2` files of `dir` (the language is the file name without the extension), sorted by name. No file at all is an error.
///
/// # Errors
/// The folder or a file cannot be read, a file name is no language tag, or two files name the same language.
pub fn read_dir(dir: &Path) -> Result<Vec<Source>, PackError> {
    let io = |what: &Path, e: std::io::Error| PackError::Io(format!("{}: {e}", what.display()));
    let mut found: Vec<Source> = Vec::new();
    let mut problems = Vec::new();
    for item in std::fs::read_dir(dir).map_err(|e| io(dir, e))? {
        let path = item.map_err(|e| io(dir, e))?.path();
        if path.extension().and_then(|e| e.to_str()) != Some("mf2") || !path.is_file() {
            continue;
        }
        let stem = path.file_stem().and_then(|s| s.to_str()).unwrap_or("");
        match locale::canonical(stem) {
            Ok(language) if found.iter().any(|s| s.language == language) => {
                problems.push(format!(
                    "{}: language {language} is given by two files",
                    path.display()
                ));
            }
            Ok(language) => {
                let text = std::fs::read_to_string(&path).map_err(|e| io(&path, e))?;
                found.push(Source { language, text });
            }
            Err(why) => problems.push(format!("{}: {why}", path.display())),
        }
    }
    if found.is_empty() && problems.is_empty() {
        problems.push(format!("{}: no *.mf2 file", dir.display()));
    }
    if !problems.is_empty() {
        return Err(PackError::Check(problems));
    }
    found.sort_by(|a, b| a.language.cmp(&b.language));
    Ok(found)
}
