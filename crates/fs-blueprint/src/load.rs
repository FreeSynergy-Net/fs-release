//! Pattern: Adapter (TOML text into the value object; a serde error becomes a code with parameters, never a sentence).

use crate::error::Coded;
use crate::model::{Blueprint, SCHEMA};
use serde::Deserialize;

/// Reads only the key `schema`, whatever else is in the file (a schema-1 file has other tables).
#[derive(Deserialize)]
struct Probe {
    schema: Option<i64>,
}

impl Blueprint {
    /// Parses a blueprint. The schema is checked first: `1` is `schema-unsupported`, above `SCHEMA` is `schema-newer`, no read path for old ones.
    ///
    /// # Errors
    /// One code: `parse-error`, `schema-unsupported`, `schema-newer`, `field-missing` or `unknown-key`.
    pub fn from_toml(src: &str) -> Result<Blueprint, Coded> {
        let probe: Probe = toml::from_str(src).map_err(|e| position(src, &e))?;
        match probe.schema {
            None => return Err(missing("", "schema")),
            Some(found) if found < SCHEMA => {
                return Err(Coded::new("schema-unsupported")
                    .with("found", found.to_string())
                    .with("known", SCHEMA.to_string()));
            }
            Some(found) if found > SCHEMA => {
                return Err(Coded::new("schema-newer")
                    .with("found", found.to_string())
                    .with("known", SCHEMA.to_string()));
            }
            Some(_) => {}
        }
        toml::from_str(src).map_err(|e| map_error(src, &e))
    }
}

fn missing(table: &str, field: &str) -> Coded {
    Coded::new("field-missing")
        .with("table", table)
        .with("field", field)
}

fn position(src: &str, e: &toml::de::Error) -> Coded {
    let at = e.span().map_or(0, |s| s.start).min(src.len());
    let before = &src[..at];
    let line = before.matches('\n').count() + 1;
    let column = before.rsplit('\n').next().map_or(0, |l| l.chars().count()) + 1;
    Coded::new("parse-error")
        .with("line", line.to_string())
        .with("column", column.to_string())
}

/// The name between backticks right after `prefix` in a serde message (`unknown field `x`, ...`).
fn quoted<'a>(message: &'a str, prefix: &str) -> Option<&'a str> {
    let rest = message.strip_prefix(prefix)?.strip_prefix('`')?;
    rest.split('`').next()
}

/// The table a span sits in: the last `[header]` / `[[header]]` line before it. A span that starts at a header (a missing field) names that table.
fn table_of(src: &str, at: usize) -> String {
    let mut table = String::new();
    let mut pos = 0;
    for line in src.split_inclusive('\n') {
        if pos > at {
            break;
        }
        let t = line.trim();
        if t.starts_with('[') && !t.starts_with("[ ") {
            table = t
                .trim_start_matches('[')
                .split(']')
                .next()
                .unwrap_or("")
                .trim()
                .to_string();
        }
        pos += line.len();
    }
    table
}

/// The key of an inline table (`summary = { ... }`) when the span is inside one on its line.
fn inline_key(src: &str, at: usize) -> Option<String> {
    let start = src[..at].rfind('\n').map_or(0, |i| i + 1);
    let line = &src[start..at];
    let (key, rest) = line.split_once('=')?;
    (rest.trim_start().starts_with('{') && !key.trim().is_empty()).then(|| key.trim().to_string())
}

fn map_error(src: &str, e: &toml::de::Error) -> Coded {
    let msg = e.message();
    let at = e.span().map_or(0, |s| s.start).min(src.len());
    if let Some(key) = quoted(msg, "unknown field ") {
        // an unknown TABLE: the span is in its header line, and the header is the path
        let line_start = src[..at].rfind('\n').map_or(0, |i| i + 1);
        if src[line_start..].trim_start().starts_with('[') {
            return Coded::new("unknown-key").with("path", table_of(src, at));
        }
        let mut path = table_of(src, at);
        if let Some(inline) = inline_key(src, at) {
            path = if path.is_empty() {
                inline
            } else {
                format!("{path}.{inline}")
            };
        }
        let path = if path.is_empty() {
            key.to_string()
        } else {
            format!("{path}.{key}")
        };
        return Coded::new("unknown-key").with("path", path);
    }
    if let Some(field) = quoted(msg, "missing field ") {
        // the span of a missing field is the header of its table (`[[module]]`); at the top level it is no header
        let header = e
            .span()
            .and_then(|s| src.get(s))
            .map(str::trim)
            .is_some_and(|t| t.starts_with('[') && t.ends_with(']') && !t.contains('\n'));
        let table = if header {
            table_of(src, at)
        } else {
            String::new()
        };
        return missing(&table, field);
    }
    position(src, e)
}
