//! Pattern: Value check (is a text a simple MF2 message: plain text and `{$name}` placeholders, the escapes `\\ \{ \} \|`; ADR 0057, ADR 0099 points 8 and 9).
//!
//! The blueprint and the message catalog of the tool use only this subset of MF2 (Unicode `MessageFormat` 2, "simple message"); the subset is NARROWER than MF2 (no
//! white space inside a placeholder, no annotation, no markup, no complex message, names of ASCII letters only): a text outside of it is refused here even where
//! MF2 would accept it, and a text inside of it is a valid MF2 simple message. A name is `name-start *name-char` of MF2 (limited to ASCII): it starts with a letter or `_`,
//! the rest are letters, digits, `_`, `-` or `.`; `{$1}` and `{$-x}` are no variables.

use std::collections::BTreeSet;

/// The white space of MF2 (`s` = white space and the bidi marks U+061C, U+200E, U+200F, U+2066 to U+2069): a message that starts with it and then a `.` is a complex message (declarations), never a simple one.
fn is_mf2_space(c: char) -> bool {
    matches!(
        c,
        ' ' | '\t' | '\r' | '\n' | '\u{3000}' | '\u{61C}' | '\u{200E}' | '\u{200F}' | '\u{2066}'
            ..='\u{2069}'
    )
}

fn is_name_char(c: char) -> bool {
    c.is_ascii_alphanumeric() || matches!(c, '_' | '-' | '.')
}

/// True when `name` is a name of the subset (ASCII): a letter or `_` first, then letters, digits, `_`, `-`, `.`.
#[must_use]
pub fn is_name(name: &str) -> bool {
    let mut chars = name.chars();
    chars
        .next()
        .is_some_and(|c| c.is_ascii_alphabetic() || c == '_')
        && chars.all(is_name_char)
}

/// The variables `{$name}` of a message (the names as they stand; whether they are valid names is [`is_simple`]'s business).
#[must_use]
pub fn variables(message: &str) -> BTreeSet<String> {
    let mut out = BTreeSet::new();
    let mut rest = message;
    while let Some(i) = rest.find("{$") {
        let tail = &rest[i + 2..];
        let end = tail.find(|c: char| !is_name_char(c)).unwrap_or(tail.len());
        out.insert(tail[..end].to_string());
        rest = &tail[end..];
    }
    out
}

/// True when `message` is a simple MF2 message.
#[must_use]
pub fn is_simple(message: &str) -> bool {
    // a message that starts with `.` (after white space) is a complex message (declarations, matchers) in MF2, never a simple one
    if message.trim_start_matches(is_mf2_space).starts_with('.') {
        return false;
    }
    let mut chars = message.chars().peekable();
    while let Some(c) = chars.next() {
        // NUL and the control characters other than tab, line feed and carriage return are no text of the subset
        if c.is_control() && !matches!(c, '\t' | '\n' | '\r') {
            return false;
        }
        match c {
            '\\' => {
                if !matches!(chars.next(), Some('\\' | '{' | '}' | '|')) {
                    return false;
                }
            }
            '{' => {
                if chars.next() != Some('$') {
                    return false;
                }
                let mut name = String::new();
                while let Some(n) = chars.next_if(|n| is_name_char(*n)) {
                    name.push(n);
                }
                if !is_name(&name) || chars.next() != Some('}') {
                    return false;
                }
            }
            '}' => return false,
            _ => {}
        }
    }
    true
}
