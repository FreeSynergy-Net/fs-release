//! Pattern: Strategy (the contract text of the program's own types as a capnp schema; the code from it stays with capnpc-rust, ADR 0099 point 13).
//!
//! Append-only: the number `@N` of a field is its place in the type, so a field is only ever added at the end (a new field in the middle moves every
//! number after it, which is a break of the contract; the version of `[contract]` names the state). The file id is `[contract] id`.

use super::artifact::{Artifact, Class, Emitter};
use super::shape::{of, pascal, Shape};
use crate::error::Coded;
use crate::kit::KitCatalog;
use crate::model::{Blueprint, TypeDef};
use std::fmt::Write;

/// Where the kits of fs-types live as capnp structs (their schema is not in the catalog yet; see the report).
pub const KIT_IMPORT: &str = "/fs-types/kit.capnp";

/// The path of the contract text of a program.
#[must_use]
pub fn path_of(bp: &Blueprint) -> String {
    format!("schema/{}.capnp", bp.program.name)
}

fn capnp_type(shape: &Shape, list: bool) -> String {
    let base = match shape {
        Shape::Text | Shape::Ref { own: false } => "Text".to_string(),
        Shape::Int | Shape::Ref { own: true } => "Int64".to_string(),
        Shape::Bool => "Bool".to_string(),
        Shape::Kit(k) => format!("Kit.{}", pascal(k)),
        Shape::Nested(n) => n.clone(),
    };
    if list {
        format!("List({base})")
    } else {
        base
    }
}

/// The deepest nesting that is written (the check refuses a cycle; this is the stop for a blueprint that was not checked).
const MAX_DEPTH: usize = 16;

/// A struct with its fields, then the structs of its nested types inside it (so a field appended later keeps every earlier line where it was).
fn write_struct(out: &mut String, bp: &Blueprint, t: &TypeDef, depth: usize) -> Result<(), Coded> {
    let bad = || Coded::new("blueprint-unserializable");
    if depth > MAX_DEPTH {
        return Err(Coded::new("nested-cycle").with("type", t.name.as_str()));
    }
    let pad = "  ".repeat(depth);
    writeln!(out, "\n{pad}struct {} {{", t.name).map_err(|_| bad())?;
    let mut inner = Vec::new();
    for (n, f) in t.field.iter().enumerate() {
        let (shape, list) = of(f)?;
        writeln!(out, "{pad}  {} @{n} :{};", f.name, capnp_type(&shape, list))
            .map_err(|_| bad())?;
        if let Shape::Nested(name) = shape {
            inner.push(
                bp.own_type(&name)
                    .ok_or_else(|| Coded::new("type-unknown").with("name", name.as_str()))?,
            );
        }
    }
    for child in inner {
        write_struct(out, bp, child, depth + 1)?;
    }
    writeln!(out, "{pad}}}").map_err(|_| bad())?;
    Ok(())
}

/// The contract text of the program's own types, or `None` when it has none. The interfaces of `provides` are NOT written here: the contract lives
/// in fs-interfaces (one source); the program only names it.
///
/// # Errors
/// `field-kind-missing{field}` for a field without a kind.
pub fn text(bp: &Blueprint) -> Result<Option<String>, Coded> {
    let own: Vec<&TypeDef> = bp.top_types().collect();
    if own.is_empty() {
        return Ok(None);
    }
    let mut out = format!(
        "@{};\n# Contract {} of {}: append-only, a field only at the end of its struct.\n",
        bp.contract.id, bp.contract.version, bp.program.name
    );
    // nested types stand inside their parent but use the import as well
    if bp
        .own_types()
        .flat_map(|t| &t.field)
        .any(|f| f.kit.is_some())
    {
        let _ = writeln!(out, "using Kit = import \"{KIT_IMPORT}\";");
    }
    for t in own {
        write_struct(&mut out, bp, t, 0)?;
    }
    Ok(Some(out))
}

/// The contract emitter.
pub struct Contract;

impl Emitter for Contract {
    fn emit(&self, bp: &Blueprint, _: &dyn KitCatalog) -> Result<Vec<Artifact>, Coded> {
        Ok(text(bp)?
            .map(|body| Artifact::new(path_of(bp), Class::Generated, body))
            .into_iter()
            .collect())
    }
}
