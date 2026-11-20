//! Pattern: Strategy (the tables of the program's own types: one typed column per field, no JSON column; ADR 0086).
//!
//! A field with `multiple` (and a `keywords` field) is a child table of its own: `<type>_<field>(owner, pos, value)`. A kit value is stored in the
//! canonical text form of its kit (the catalog knows no column layout of a kit yet; see the report).

use super::artifact::{Artifact, Class, Emitter};
use super::shape::{of, snake, Shape};
use super::sql_names::ident;
use crate::error::Coded;
use crate::kit::KitCatalog;
use crate::model::{Blueprint, TypeDef};
use std::fmt::Write;

/// The path of the table text of a program.
#[must_use]
pub fn path_of(bp: &Blueprint) -> String {
    format!("schema/{}.sql", bp.program.name)
}

fn sql_type(shape: &Shape) -> &'static str {
    match shape {
        Shape::Text | Shape::Kit(_) => "TEXT",
        Shape::Int => "INTEGER",
        Shape::Bool => "BOOLEAN",
    }
}

fn write_tables(out: &mut String, t: &TypeDef) -> Result<(), Coded> {
    let bad = || Coded::new("blueprint-unserializable");
    let table = snake(&t.name);
    let mut columns = vec![format!("{} INTEGER PRIMARY KEY", ident("id"))];
    let mut children = String::new();
    for f in &t.field {
        let (shape, list) = of(f)?;
        let name = snake(&f.name);
        // a column `id` of the field would be the key column twice (a list field is a child table and has no column)
        if !list && name == "id" {
            return Err(Coded::new("column-collides")
                .with("type", t.name.as_str())
                .with("field", f.name.as_str()));
        }
        if list {
            writeln!(
                children,
                "\nCREATE TABLE {} (\n  {owner} INTEGER NOT NULL REFERENCES {}({}),\n  {pos} INTEGER NOT NULL,\n  {value} {} NOT NULL,\n  PRIMARY KEY ({owner}, {pos})\n);",
                ident(&format!("{table}_{name}")),
                ident(&table),
                ident("id"),
                sql_type(&shape),
                owner = ident("owner"),
                pos = ident("pos"),
                value = ident("value")
            )
            .map_err(|_| bad())?;
        } else {
            columns.push(format!("{} {}", ident(&name), sql_type(&shape)));
        }
    }
    writeln!(
        out,
        "\nCREATE TABLE {} (\n  {}\n);",
        ident(&table),
        columns.join(",\n  ")
    )
    .map_err(|_| bad())?;
    out.push_str(&children);
    Ok(())
}

/// The table text, or `None` when the program has no types of its own.
///
/// # Errors
/// `field-kind-missing{field}` for a field without a kind.
pub fn text(bp: &Blueprint) -> Result<Option<String>, Coded> {
    let own: Vec<&TypeDef> = bp.own_types().collect();
    if own.is_empty() {
        return Ok(None);
    }
    let mut out = format!(
        "-- Tables of {} for contract {}: typed columns, no JSON column.\n",
        bp.program.name, bp.contract.version
    );
    for t in own {
        write_tables(&mut out, t)?;
    }
    Ok(Some(out))
}

/// The table emitter.
pub struct Store;

impl Emitter for Store {
    fn emit(&self, bp: &Blueprint, _: &dyn KitCatalog) -> Result<Vec<Artifact>, Coded> {
        Ok(text(bp)?
            .map(|body| Artifact::new(path_of(bp), Class::Generated, body))
            .into_iter()
            .collect())
    }
}
