//! Pattern: Template method (the content of the forms description: the part of the rules of the forms host that the blueprint can judge without it).
//!
//! The host reads the file in the exchange form (TOML, format 1 or 2): the keys of the head are required, an unknown key is an error. The blueprint does not
//! carry the whole parser; it refuses what the host refuses at the first look, so that `new` never hands out a description the window cannot open.

/// The keys of the top level the exchange form knows.
const KNOWN: &[&str] = &[
    "schema", "kind", "id", "extends", "exec", "title", "style", "requires", "source", "menu",
    "page", "action", "view", "message",
];

/// The keys the head must carry (all texts).
const REQUIRED: &[&str] = &["id", "extends", "exec", "title"];

/// The cause why the host cannot read `text`, or `None` when its head is in order.
#[must_use]
pub fn unreadable(text: &str) -> Option<String> {
    let doc: toml::Table = match text.parse() {
        Ok(d) => d,
        Err(_) => return Some("toml".to_string()),
    };
    match doc.get("schema").and_then(toml::Value::as_integer) {
        Some(1 | 2) => {}
        Some(_) => return Some("schema-unsupported".to_string()),
        None => return Some("key-missing schema".to_string()),
    }
    if doc.get("kind").and_then(toml::Value::as_str) != Some("frontend") {
        return Some("kind".to_string());
    }
    if let Some(k) = REQUIRED.iter().find(|k| {
        doc.get(**k)
            .and_then(toml::Value::as_str)
            .is_none_or(str::is_empty)
    }) {
        return Some(format!("key-missing {k}"));
    }
    doc.keys()
        .find(|k| !KNOWN.contains(&k.as_str()))
        .map(|k| format!("key-unknown {k}"))
}
