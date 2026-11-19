//! Pattern: Value check (is a text a simple MF2 message: plain text and `{$name}` placeholders, the escapes `\\ \{ \} \|`; ADR 0057, ADR 0099 points 8 and 9).
//!
//! The blueprint and the message catalog of the tool use only this subset. Anything else in braces (a function call, a selector, a literal) is not read
//! here and is refused, so a text the check passes is a text every MF2 reader can fill.

/// True when `message` is a simple MF2 message.
#[must_use]
pub fn is_simple(message: &str) -> bool {
    // a message that starts with `.` is a complex message (declarations, matchers) in MF2, never a simple one
    if message.starts_with('.') {
        return false;
    }
    let mut chars = message.chars().peekable();
    while let Some(c) = chars.next() {
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
                let mut name = 0usize;
                while chars
                    .peek()
                    .is_some_and(|n| n.is_ascii_alphanumeric() || *n == '_' || *n == '-')
                {
                    chars.next();
                    name += 1;
                }
                if name == 0 || chars.next() != Some('}') {
                    return false;
                }
            }
            '}' => return false,
            _ => {}
        }
    }
    true
}
