//! Pattern: Value object (the blueprint, schema 2: exactly the keys of the addendum, every table `deny_unknown_fields`).

use serde::{Deserialize, Serialize};

/// The schema this tool reads. Schema 1 and everything else is refused (no migration, no read path; user rule 2026-11-18).
pub const SCHEMA: i64 = 2;

/// Human text in two languages, each one an MF2 message (ADR 0057). Both are mandatory; the check says which one is missing.
#[derive(Debug, Clone, Default, PartialEq, Eq, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct Text2 {
    pub en: Option<String>,
    pub de: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct Blueprint {
    pub schema: i64,
    pub program: Program,
    pub contract: Contract,
    #[serde(default)]
    pub module: Vec<Module>,
    #[serde(default)]
    pub bundle: Vec<Bundle>,
    #[serde(default, rename = "type")]
    pub types: Vec<TypeDef>,
    #[serde(default)]
    pub ruleset: Vec<Ruleset>,
    pub frontend: Frontend,
    #[serde(default)]
    pub exception: Vec<Exception>,
}

#[derive(Debug, Clone, PartialEq, Eq, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct Program {
    pub name: String,
    /// The version of the program, `major.minor.patch` (version = tag rule: the release workflow raises it).
    pub version: String,
    pub skeleton: String,
    pub capability: String,
    pub summary: Text2,
    pub description: Text2,
    pub categories: Vec<String>,
    pub icon: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct Contract {
    pub version: String,
    pub id: String,
    #[serde(default)]
    pub provides: Vec<String>,
}

/// A module of kind `standard`, `toolkit` or `component`; `kind` stays a word here so the check can name a wrong one.
#[derive(Debug, Clone, PartialEq, Eq, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct Module {
    pub name: String,
    pub kind: String,
    pub version: String,
}

/// A bundle: a toolkit as a whole (its parts and the type it serves). There is no `[[component]]` table.
#[derive(Debug, Clone, PartialEq, Eq, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct Bundle {
    pub name: String,
    pub version: String,
    pub parts: Vec<String>,
    pub item: String,
}

/// A data type. With `module` it belongs to that module and carries nothing else; without, it is the program's own type.
#[derive(Debug, Clone, PartialEq, Eq, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct TypeDef {
    pub name: String,
    pub module: Option<String>,
    pub standard: Option<String>,
    pub envelope: Option<String>,
    pub media_type: Option<String>,
    pub label: Option<Text2>,
    #[serde(default)]
    pub field: Vec<Field>,
}

#[derive(Debug, Clone, PartialEq, Eq, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct Field {
    pub name: String,
    pub kit: Option<String>,
    pub basic: Option<String>,
    pub standard: Option<String>,
    #[serde(default)]
    pub multiple: bool,
    pub words: Option<String>,
    pub label: Text2,
}

#[derive(Debug, Clone, PartialEq, Eq, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct Ruleset {
    pub name: String,
    #[serde(rename = "type")]
    pub type_name: String,
    pub from: String,
    pub presets: Vec<String>,
    pub default: String,
}

/// `all` or a list of `<Interface>.<method>` / `<Type>.<field>`.
#[derive(Debug, Clone, PartialEq, Eq, Deserialize, Serialize)]
#[serde(untagged)]
pub enum Covers {
    Word(String),
    List(Vec<String>),
}

#[derive(Debug, Clone, PartialEq, Eq, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct Frontend {
    pub standard: String,
    pub layout: String,
    pub covers: Covers,
}

/// A conversation-approved piece of hand-written code. No date field (ADR 0101 E7 with the one addition `paths`).
#[derive(Debug, Clone, PartialEq, Eq, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct Exception {
    pub what: String,
    pub why: String,
    pub checked: Vec<String>,
    pub alternatives: Vec<String>,
    pub talk: String,
    pub paths: Vec<String>,
}
