//! Pattern: Template method (the content of the forms description: the part of the rules of the forms host that the blueprint can judge without it).
//!
//! The host reads the file in the exchange form (TOML, format 3 or 4, `fs-forms-layout` `FORMAT_MIN` and `FORMAT_KNOWN`): the keys of the head are required, an
//! unknown key is an error, and every table of a list carries its element id (`elem`, format 3: the host reports `elem-missing`). Format 1 and 2 are refused by
//! the host (`format-old`, no second reading path), a format above 4 too (`format-newer`). The blueprint does not carry the whole parser; it refuses what the host
//! refuses at the first look, so that `new` never hands out a description the window cannot open.
//! Limits, said honestly: `elem` is checked only on the tables of the seven lists of the head, not on nested tables (`page.place`, `menu.entries`, `view.order`, schemes), and
//! its characters (`elem-invalid`) are the host's to judge; the places, bindings and blocks are not judged at all. `contract` of `requires` and `source` is checked for its
//! spelling only (`name@>=a.b, <c.d`, as `range_bounds` of the host reads it), not for its numbers.

/// The oldest format the host reads (`fs_forms_layout::FORMAT_MIN`; 3 is a subset of 4).
pub const FORMAT_MIN: i64 = 3;

/// The newest format the host reads (`fs_forms_layout::FORMAT_KNOWN`; 4 adds lanes: the key `source` of a binding and module scopes).
pub const FORMAT_KNOWN: i64 = 4;

/// The keys of the top level the exchange form knows (`fs-forms-exchange` `read.rs`, `layout_doc`; `sample` is handed to `fs-forms-testdata`).
const KNOWN: &[&str] = &[
    "schema", "kind", "id", "extends", "exec", "title", "style", "requires", "source", "menu",
    "page", "action", "view", "message", "sample",
];

/// The lists of the top level whose tables each carry an `elem`.
const LISTS: &[&str] = &[
    "requires", "source", "menu", "page", "action", "view", "message",
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
        Some(FORMAT_MIN..=FORMAT_KNOWN) => {}
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
    if let Some(k) = doc.keys().find(|k| !KNOWN.contains(&k.as_str())) {
        return Some(format!("key-unknown {k}"));
    }
    let tables = || {
        LISTS
            .iter()
            .filter_map(|l| doc.get(*l).and_then(toml::Value::as_array))
            .flatten()
    };
    if tables().any(|t| {
        t.get("elem")
            .and_then(toml::Value::as_str)
            .is_none_or(str::is_empty)
    }) {
        return Some("key-missing elem".to_string());
    }
    for l in ["requires", "source"] {
        let list = doc
            .get(l)
            .and_then(toml::Value::as_array)
            .into_iter()
            .flatten();
        for t in list {
            let text = t
                .get("contract")
                .and_then(toml::Value::as_str)
                .unwrap_or_default();
            if !range_spelling_ok(text) {
                return Some(format!("range-invalid {text}"));
            }
        }
    }
    None
}

/// The spelling of a contract range: `name@>=a.b, <c.d` (one name, `@`, a lower bound `>=`, comma and one space, an upper bound `<`).
fn range_spelling_ok(text: &str) -> bool {
    let Some((name, req)) = text.split_once('@') else {
        return false;
    };
    let Some(bounds) = req.strip_prefix(">=").and_then(|r| r.split_once(", <")) else {
        return false;
    };
    let version = |v: &str| {
        !v.is_empty()
            && v.split('.').count() <= 3
            && v.split('.')
                .all(|p| !p.is_empty() && p.bytes().all(|b| b.is_ascii_digit()))
    };
    !name.is_empty()
        && !name.contains(char::is_whitespace)
        && version(bounds.0)
        && version(bounds.1)
}
