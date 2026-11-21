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
use std::collections::BTreeSet;
use std::fmt::Write;

/// The path of the table text of a program.
#[must_use]
pub fn path_of(bp: &Blueprint) -> String {
    format!("schema/{}.sql", bp.program.name)
}

fn sql_type(shape: &Shape) -> &'static str {
    match shape {
        Shape::Text | Shape::Kit(_) | Shape::Ref { own: false } => "TEXT",
        Shape::Int | Shape::Ref { own: true } | Shape::Nested(_) => "INTEGER",
        Shape::Bool => "BOOLEAN",
    }
}

/// The deepest nesting that is written (the check refuses a cycle; this is the stop for a blueprint that was not checked).
const MAX_DEPTH: usize = 16;

/// The tables of one type, its child tables after it. `parent` is the table a nested type hangs on and whether it is a single value.
struct Tables<'a> {
    bp: &'a Blueprint,
    out: String,
    names: BTreeSet<String>,
}

impl Tables<'_> {
    fn emit(
        &mut self,
        table: &str,
        t: &TypeDef,
        parent: Option<(&str, bool)>,
        depth: usize,
    ) -> Result<(), Coded> {
        let bad = || Coded::new("blueprint-unserializable");
        if depth > MAX_DEPTH {
            return Err(Coded::new("nested-cycle").with("type", t.name.as_str()));
        }
        if !self.names.insert(table.to_string()) {
            return Err(Coded::new("table-collides").with("table", table));
        }
        let mut columns = vec![format!("{} INTEGER PRIMARY KEY", ident("id"))];
        if let Some((p, _)) = parent {
            columns.push(format!(
                "{} INTEGER NOT NULL REFERENCES {}({})",
                ident("owner"),
                ident(p),
                ident("id")
            ));
            columns.push(format!("{} INTEGER NOT NULL", ident("pos")));
        }
        let reserved: &[&str] = if parent.is_some() {
            &["id", "owner", "pos"]
        } else {
            &["id"]
        };
        let mut children = Tables {
            bp: self.bp,
            out: String::new(),
            names: std::mem::take(&mut self.names),
        };
        for f in &t.field {
            let (shape, list) = of(f)?;
            let name = snake(&f.name);
            // a column of the field would be a key column twice (a list field is a child table and has no column)
            if !list && !matches!(shape, Shape::Nested(_)) && reserved.contains(&name.as_str()) {
                return Err(Coded::new("column-collides")
                    .with("type", t.name.as_str())
                    .with("field", f.name.as_str()));
            }
            let child = format!("{table}_{name}");
            match &shape {
                Shape::Nested(n) => {
                    let target = self
                        .bp
                        .own_type(n)
                        .ok_or_else(|| Coded::new("type-unknown").with("name", n.as_str()))?;
                    children.emit(&child, target, Some((table, !list)), depth + 1)?;
                }
                _ if list => {
                    if !children.names.insert(child.clone()) {
                        return Err(Coded::new("table-collides").with("table", child.as_str()));
                    }
                    children
                        .out
                        .push_str(&Self::link_table(table, &child, f, &shape));
                }
                _ => columns.push(Self::column(&name, f, &shape)),
            }
        }
        self.names = children.names;
        if matches!(parent, Some((_, true))) {
            columns.push(format!("UNIQUE ({})", ident("owner")));
        }
        writeln!(
            self.out,
            "\nCREATE TABLE {} (\n  {}\n);",
            ident(table),
            columns.join(",\n  ")
        )
        .map_err(|_| bad())?;
        self.out.push_str(&children.out);
        Ok(())
    }

    /// A column: the key of an own type is a foreign key, an IRI or a value is plain.
    fn column(name: &str, f: &crate::model::Field, shape: &Shape) -> String {
        let fk = match (&f.reference, shape) {
            (Some(r), Shape::Ref { own: true }) => {
                format!(" REFERENCES {}({})", ident(&snake(r)), ident("id"))
            }
            _ => String::new(),
        };
        format!("{} {}{fk}", ident(name), sql_type(shape))
    }

    /// The child table of a list: `(owner, pos, value)`, for a reference list `(owner, pos, target)`.
    fn link_table(table: &str, child: &str, f: &crate::model::Field, shape: &Shape) -> String {
        let (value, fk) = match (&f.reference, shape) {
            (Some(r), Shape::Ref { own: true }) => (
                "target",
                format!(" REFERENCES {}({})", ident(&snake(r)), ident("id")),
            ),
            (Some(_), _) => ("target", String::new()),
            _ => ("value", String::new()),
        };
        format!(
            "\nCREATE TABLE {} (\n  {owner} INTEGER NOT NULL REFERENCES {}({}),\n  {pos} INTEGER NOT NULL,\n  {} {} NOT NULL{fk},\n  PRIMARY KEY ({owner}, {pos})\n);\n",
            ident(child),
            ident(table),
            ident("id"),
            ident(value),
            sql_type(shape),
            owner = ident("owner"),
            pos = ident("pos"),
        )
    }
}

fn write_tables(
    out: &mut String,
    bp: &Blueprint,
    t: &TypeDef,
    names: &mut BTreeSet<String>,
) -> Result<(), Coded> {
    let mut tables = Tables {
        bp,
        out: String::new(),
        names: std::mem::take(names),
    };
    let result = tables.emit(&snake(&t.name), t, None, 0);
    *names = tables.names;
    result?;
    out.push_str(&tables.out);
    Ok(())
}

/// The table text, or `None` when the program has no types of its own.
///
/// # Errors
/// `field-kind-missing{field}` for a field without a kind.
pub fn text(bp: &Blueprint) -> Result<Option<String>, Coded> {
    let own: Vec<&TypeDef> = bp.top_types().collect();
    if own.is_empty() {
        return Ok(None);
    }
    let mut out = format!(
        "-- Tables of {} for contract {}: typed columns, no JSON column.\n",
        bp.program.name, bp.contract.version
    );
    let mut names = BTreeSet::new();
    for t in own {
        write_tables(&mut out, bp, t, &mut names)?;
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
