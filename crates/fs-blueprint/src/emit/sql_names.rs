//! Pattern: Value object table (the words SQL keeps for itself: a table or column of such a name is written in double quotes, never bare).

/// The key words of SQLite and the SQL standard that a type or field name can plausibly be (lower case, sorted).
const RESERVED: &[&str] = &[
    "all",
    "alter",
    "and",
    "as",
    "asc",
    "between",
    "by",
    "case",
    "check",
    "collate",
    "column",
    "constraint",
    "create",
    "cross",
    "default",
    "delete",
    "desc",
    "distinct",
    "drop",
    "else",
    "end",
    "except",
    "exists",
    "foreign",
    "from",
    "full",
    "group",
    "having",
    "in",
    "index",
    "inner",
    "insert",
    "intersect",
    "into",
    "is",
    "join",
    "key",
    "left",
    "like",
    "limit",
    "natural",
    "not",
    "null",
    "offset",
    "on",
    "or",
    "order",
    "outer",
    "primary",
    "references",
    "right",
    "select",
    "set",
    "table",
    "then",
    "to",
    "union",
    "unique",
    "update",
    "using",
    "values",
    "view",
    "when",
    "where",
    "with",
];

/// True when the name is a key word of SQL.
#[must_use]
pub fn is_reserved(name: &str) -> bool {
    RESERVED.binary_search(&name).is_ok()
}

/// The name as it stands in the SQL text: bare, or in double quotes when it is a key word.
#[must_use]
pub fn ident(name: &str) -> String {
    if is_reserved(name) {
        format!("\"{name}\"")
    } else {
        name.to_string()
    }
}
