//! Pattern: Strategy table (what a field of a type is, as the contract and the store both need it: one table, so both stay in step).

use crate::error::Coded;
use crate::form::is_iri;
use crate::model::Field;

/// The shape of a field value, without the `multiple` list.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Shape {
    Text,
    Int,
    Bool,
    /// A value of a kit of fs-types; the name is the kit as written in the blueprint (`timeSpan`).
    Kit(String),
    /// An own type nested into the parent (a child table, a struct inside the parent's struct).
    Nested(String),
    /// A link to an own type (`own`, the key of its table) or to an IRI of a standard (text).
    Ref {
        own: bool,
    },
}

/// The shape of a field. The basic types `integer` and `whole` are whole numbers, `flag` is yes/no, `keywords` is a list of words, the rest is text
/// (dates, durations and links in the text form of their standard).
///
/// # Errors
/// `field-kind-missing{field}` when the field names none of kit, basic type, nested type or reference (the check refuses that before).
pub fn of(f: &Field) -> Result<(Shape, bool), Coded> {
    let missing = || Coded::new("field-kind-missing").with("field", f.name.as_str());
    if let Some(k) = &f.kit {
        return Ok((Shape::Kit(k.clone()), f.multiple));
    }
    if let Some(n) = &f.nested {
        return Ok((Shape::Nested(n.clone()), f.multiple));
    }
    if let Some(r) = &f.reference {
        return Ok((Shape::Ref { own: !is_iri(r) }, f.multiple));
    }
    let b = f.basic.as_deref().ok_or_else(missing)?;
    let shape = match b {
        "integer" | "whole" => Shape::Int,
        "flag" => Shape::Bool,
        _ => Shape::Text,
    };
    Ok((shape, f.multiple || b == "keywords"))
}

/// `TaskItem` becomes `task_item`, `dueDate` becomes `due_date`.
#[must_use]
pub fn snake(name: &str) -> String {
    let mut out = String::with_capacity(name.len() + 4);
    for (i, c) in name.chars().enumerate() {
        if c.is_ascii_uppercase() {
            if i > 0 {
                out.push('_');
            }
            out.push(c.to_ascii_lowercase());
        } else {
            out.push(c);
        }
    }
    out
}

/// `timeSpan` becomes `TimeSpan`.
#[must_use]
pub fn pascal(name: &str) -> String {
    let mut c = name.chars();
    c.next()
        .map(|f| f.to_ascii_uppercase().to_string() + c.as_str())
        .unwrap_or_default()
}
