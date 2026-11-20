//! Pattern: Value object table (a table or column name stands in the SQL text always in double quotes: no list of key words can be complete).

/// The name as it stands in the SQL text: in double quotes, a quote inside doubled.
#[must_use]
pub fn ident(name: &str) -> String {
    format!("\"{}\"", name.replace('"', "\"\""))
}

#[cfg(test)]
mod tests {
    use super::ident;

    #[test]
    fn every_name_is_quoted() {
        assert_eq!(ident("shift"), "\"shift\"");
        for word in ["transaction", "commit", "order"] {
            assert_eq!(ident(word), format!("\"{word}\""));
        }
    }

    #[test]
    fn a_quote_in_the_name_is_doubled() {
        assert_eq!(ident("a\"b"), "\"a\"\"b\"");
    }
}
