//! Pattern: Visitor (put the tables and keys of a blueprint text in the order of the schema WITHOUT losing a comment; ADR 0099 point 14).
//!
//! The target order is the order of the canonical text (the serialization of the parsed value: the order of the schema). The text of the author is parsed with `toml_edit`
//! (it keeps comments, white space and the form of every value) and only MOVED: the key/value pairs of a table are sorted, the tables get new positions. A comment
//! belongs to the line below it: it moves with that key or table. The entries of a list keep their order; the form of a value (quotes, inline table or table) is
//! the author's. A dotted key (`a.b = 1`) stays where it is.

use toml_edit::{DocumentMut, Item, Table, Value};

/// The order of the keys of a text, as a tree: a map (the keys in order), a list or a leaf.
enum Shape {
    Map(Vec<(String, Shape)>),
    List(Vec<Shape>),
    Leaf,
}

fn shape_of_value(v: &Value) -> Shape {
    match v {
        Value::InlineTable(t) => Shape::Map(
            t.iter()
                .map(|(k, v)| (k.to_string(), shape_of_value(v)))
                .collect(),
        ),
        Value::Array(a) => Shape::List(a.iter().map(shape_of_value).collect()),
        _ => Shape::Leaf,
    }
}

fn shape_of_item(i: &Item) -> Shape {
    match i {
        Item::Table(t) => shape_of_table(t),
        Item::ArrayOfTables(a) => Shape::List(a.iter().map(shape_of_table).collect()),
        Item::Value(v) => shape_of_value(v),
        Item::None => Shape::Leaf,
    }
}

fn shape_of_table(t: &Table) -> Shape {
    Shape::Map(
        t.iter()
            .map(|(k, i)| (k.to_string(), shape_of_item(i)))
            .collect(),
    )
}

fn index_of(order: &[(String, Shape)], key: &str) -> usize {
    order
        .iter()
        .position(|(k, _)| k == key)
        .unwrap_or(usize::MAX)
}

fn move_table(t: &mut Table, order: &[(String, Shape)], position: &mut usize, own_position: bool) {
    if own_position {
        t.set_position(*position);
        *position += 1;
    }
    t.sort_values_by(|a, _, b, _| index_of(order, a).cmp(&index_of(order, b)));
    // the children in the order of the schema, so the positions grow in that order
    for (key, shape) in order {
        if let Some(item) = t.get_mut(key) {
            move_item(item, shape, position);
        }
    }
}

fn move_item(item: &mut Item, shape: &Shape, position: &mut usize) {
    match (item, shape) {
        (Item::Table(t), Shape::Map(order)) => move_table(t, order, position, !t.is_dotted()),
        (Item::ArrayOfTables(a), Shape::List(shapes)) => {
            for (t, s) in a.iter_mut().zip(shapes) {
                if let Shape::Map(order) = s {
                    move_table(t, order, position, true);
                }
            }
        }
        (Item::Value(v), s) => move_value(v, s),
        _ => {}
    }
}

fn move_value(v: &mut Value, shape: &Shape) {
    match (v, shape) {
        (Value::InlineTable(t), Shape::Map(order)) => {
            t.sort_values_by(|a, _, b, _| index_of(order, a).cmp(&index_of(order, b)));
            for (key, s) in order {
                if let Some(child) = t.get_mut(key) {
                    move_value(child, s);
                }
            }
        }
        (Value::Array(a), Shape::List(shapes)) => {
            for (child, s) in a.iter_mut().zip(shapes) {
                move_value(child, s);
            }
        }
        _ => {}
    }
}

/// `text` with its tables and keys in the order of `canonical` (the canonical text of the same blueprint), every comment kept; UTF-8, LF, one LF at the end.
/// `None` when one of the two texts is no TOML.
#[must_use]
pub fn in_order(text: &str, canonical: &str) -> Option<String> {
    let want: DocumentMut = canonical.parse().ok()?;
    let mut doc: DocumentMut = text.parse().ok()?;
    let Shape::Map(order) = shape_of_table(want.as_table()) else {
        return None;
    };
    let mut position = 0;
    move_table(doc.as_table_mut(), &order, &mut position, false);
    let mut out = doc.to_string().replace("\r\n", "\n");
    out.truncate(out.trim_end_matches('\n').len());
    out.push('\n');
    Some(out)
}
