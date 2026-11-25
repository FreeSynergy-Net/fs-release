//! Pattern: Strategy (the emitter of the forms description: a minimal, valid frontend description of the program, written once as a seed).
//!
//! The file lies at `[frontend] layout` and is in the exchange form the forms host reads (format 3: every table of a list carries its element id `elem`, written as the host derives it: `requires:1`, `message:<key>/<lang>`): header, the contract the
//! frontend needs, and the title in both languages. It has no page yet: the author adds the pages. A description the author brought along is never replaced
//! (`new` leaves the seed out when the file is there; the writer never overwrites a seed).

use super::artifact::{Artifact, Class, Emitter};
use super::frontend::HOST;
use super::slot::{Slot, Values};
use crate::error::Coded;
use crate::form::is_repo_path;
use crate::kit::KitCatalog;
use crate::model::Blueprint;
use serde::Serialize;

#[derive(Serialize)]
struct Requires {
    elem: String,
    contract: String,
}

#[derive(Serialize)]
struct Message<'a> {
    elem: String,
    key: String,
    lang: &'a str,
    text: &'a str,
}

#[derive(Serialize)]
struct Description<'a> {
    schema: u32,
    kind: &'a str,
    id: &'a str,
    extends: &'a str,
    exec: &'a str,
    title: String,
    requires: Vec<Requires>,
    message: Vec<Message<'a>>,
}

/// The range of the contract versions the frontend works with: `0.1` becomes `tasks@>=0.1, <0.2`, `2.3` becomes `tasks@>=2.3, <3.0` (the grammar of `fs-forms-layout` `range_bounds`).
fn range(capability: &str, version: &str) -> String {
    let mut parts = version.split('.').map(|p| p.parse::<u64>().unwrap_or(0));
    let (major, minor) = (parts.next().unwrap_or(0), parts.next().unwrap_or(0));
    let above = if major == 0 {
        format!("0.{}", minor + 1)
    } else {
        format!("{}.0", major + 1)
    };
    format!("{capability}@>={major}.{minor}, <{above}")
}

/// True for a path the head line can be written to as TOML (the same rule as `header::form_of`: lower case `.toml`, nothing else).
#[must_use]
#[allow(clippy::case_sensitive_file_extension_comparisons)]
pub fn is_toml(path: &str) -> bool {
    path.ends_with(".toml")
}

/// The forms description of the empty program.
pub struct Layout;

impl Emitter for Layout {
    fn emit(&self, bp: &Blueprint, _: &dyn KitCatalog) -> Result<Vec<Artifact>, Coded> {
        let path = bp.frontend.layout.as_str();
        // another kind of file has no head line (the writer cannot stamp it); the check names a path that is wrong
        if !is_repo_path(path) || !is_toml(path) {
            return Ok(Vec::new());
        }
        let values = Values::of(bp)?;
        let title = format!("{}-title", values.get(Slot::Program));
        let name = values.get(Slot::Pascal);
        let doc = Description {
            schema: 3,
            kind: "frontend",
            id: &bp.frontend.standard,
            extends: values.get(Slot::AppId),
            exec: HOST,
            title: title.clone(),
            requires: vec![Requires {
                elem: "requires:1".to_string(),
                contract: range(
                    values.get(Slot::Capability),
                    values.get(Slot::ContractVersion),
                ),
            }],
            message: vec![
                Message {
                    elem: format!("message:{title}/en"),
                    key: title.clone(),
                    lang: "en",
                    text: name,
                },
                Message {
                    elem: format!("message:{title}/de"),
                    key: title,
                    lang: "de",
                    text: name,
                },
            ],
        };
        let body = toml::to_string(&doc).map_err(|_| Coded::new("blueprint-unserializable"))?;
        Ok(vec![Artifact::new(path, Class::Seed, body)])
    }
}
