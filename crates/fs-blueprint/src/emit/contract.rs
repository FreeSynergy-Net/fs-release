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
        Shape::Text => "Text".to_string(),
        Shape::Int => "Int64".to_string(),
        Shape::Bool => "Bool".to_string(),
        Shape::Kit(k) => format!("Kit.{}", pascal(k)),
    };
    if list {
        format!("List({base})")
    } else {
        base
    }
}

fn write_struct(out: &mut String, t: &TypeDef) -> Result<(), Coded> {
    let bad = || Coded::new("blueprint-unserializable");
    writeln!(out, "\nstruct {} {{", t.name).map_err(|_| bad())?;
    for (n, f) in t.field.iter().enumerate() {
        let (shape, list) = of(f)?;
        writeln!(out, "  {} @{n} :{};", f.name, capnp_type(&shape, list)).map_err(|_| bad())?;
    }
    out.push_str("}\n");
    Ok(())
}

/// `cycle-check` as `cycleCheck`: the name of a method in capnp.
fn camel(word: &str) -> String {
    let mut parts = word.split(['-', '.']).filter(|p| !p.is_empty());
    let first = parts.next().unwrap_or_default().to_string();
    parts.fold(first, |acc, p| acc + &pascal(p))
}

/// The interfaces of `[contract] provides`: an entry `<Interface>.<method>` is the method `method` of the interface (the part before the last dot,
/// in Pascal case), numbered by its place in the list; an entry without a dot is an interface without methods. The order of the blueprint is kept.
fn interfaces(provides: &[String]) -> Vec<(String, Vec<String>)> {
    let mut list: Vec<(String, Vec<String>)> = Vec::new();
    for key in provides {
        let (name, method) = match key.rsplit_once('.') {
            Some((i, m)) => (i, Some(camel(m))),
            None => (key.as_str(), None),
        };
        let name = name.split(['-', '.']).map(pascal).collect::<String>();
        let at = list.iter().position(|(n, _)| *n == name);
        let slot = at.unwrap_or_else(|| {
            list.push((name, Vec::new()));
            list.len() - 1
        });
        if let Some(m) = method {
            list[slot].1.push(m);
        }
    }
    list
}

fn write_interface(out: &mut String, name: &str, methods: &[String]) {
    let _ = writeln!(out, "\ninterface {name} {{");
    for (n, m) in methods.iter().enumerate() {
        let _ = writeln!(out, "  {m} @{n} () -> ();");
    }
    out.push_str("}\n");
}

/// The contract text, or `None` when the program has no types of its own and provides nothing.
///
/// # Errors
/// `field-kind-missing{field}` for a field without a kind.
pub fn text(bp: &Blueprint) -> Result<Option<String>, Coded> {
    let own: Vec<&TypeDef> = bp.own_types().collect();
    if own.is_empty() && bp.contract.provides.is_empty() {
        return Ok(None);
    }
    let mut out = format!(
        "@{};\n# Contract {} of {}: append-only, a field or method only at the end of its struct or interface.\n",
        bp.contract.id, bp.contract.version, bp.program.name
    );
    if own.iter().flat_map(|t| &t.field).any(|f| f.kit.is_some()) {
        let _ = writeln!(out, "using Kit = import \"{KIT_IMPORT}\";");
    }
    for t in own {
        write_struct(&mut out, t)?;
    }
    for (name, methods) in interfaces(&bp.contract.provides) {
        write_interface(&mut out, &name, &methods);
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
