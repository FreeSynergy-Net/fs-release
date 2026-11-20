//! Pattern: Strategy (the emitter of `[frontend]`, ADR 0099 point 12: no frontend crate, only the frontend manifest and the package `layout` that carries the exchange file).
//!
//! The manifest is an AppStream addon of the program (`fs:exec = fs-forms-run`, `fs:frontend-id`, `fs:covers`, `fs:contract`; ADR 0094); the package `layout` is a
//! small data file that names the exchange file of the forms description and what it covers. Both are plain data: nothing here is code.

use super::artifact::{Artifact, Class, Emitter};
use super::slot::{Slot, Values};
use crate::error::Coded;
use crate::kit::KitCatalog;
use crate::model::{Blueprint, Covers};

/// The program that runs every forms description: the frontend is data, this is its host.
pub const HOST: &str = "fs-forms-run";

/// Text for an XML element or attribute value.
fn xml(text: &str) -> String {
    let mut out = String::with_capacity(text.len());
    for c in text.chars() {
        match c {
            '&' => out.push_str("&amp;"),
            '<' => out.push_str("&lt;"),
            '>' => out.push_str("&gt;"),
            '"' => out.push_str("&quot;"),
            '\'' => out.push_str("&apos;"),
            other => out.push(other),
        }
    }
    out
}

/// The cover of a frontend as one word: `all`, or the entries separated by commas.
fn covers(c: &Covers) -> String {
    match c {
        Covers::Word(w) => w.clone(),
        Covers::List(l) => l.join(","),
    }
}

/// The id of the frontend: the program and the standard of the frontend, `fs-tasks-simple`.
fn frontend_id(bp: &Blueprint, values: &Values) -> String {
    format!("{}-{}", values.get(Slot::Kebab), bp.frontend.standard)
}

fn manifest(bp: &Blueprint, values: &Values) -> Artifact {
    let id = frontend_id(bp, values);
    let app = values.get(Slot::AppId);
    let body = format!(
        "<component type=\"addon\">\n  <id>{app}.{standard}</id>\n  <extends>{app}</extends>\n  <name>{name} {standard}</name>\n  <summary>{summary}</summary>\n  \
         <requires>\n    <id version=\"{contract}\" compare=\"ge\">{contract_id}</id>\n  </requires>\n  <custom>\n    <value key=\"fs:frontend-id\">{id}</value>\n    \
         <value key=\"fs:exec\">{HOST}</value>\n    <value key=\"fs:layout\">{layout}</value>\n    <value key=\"fs:covers\">{covers}</value>\n    \
         <value key=\"fs:contract\">{contract}</value>\n  </custom>\n</component>\n",
        app = xml(app),
        standard = xml(&bp.frontend.standard),
        name = xml(values.get(Slot::Pascal)),
        summary = xml(values.get(Slot::SummaryEn)),
        contract = xml(values.get(Slot::ContractVersion)),
        contract_id = xml(values.get(Slot::ContractId)),
        id = xml(&id),
        layout = xml(&bp.frontend.layout),
        covers = xml(&covers(&bp.frontend.covers)),
    );
    Artifact::new(
        format!("packaging/frontends/{id}.metainfo.xml"),
        Class::Generated,
        body,
    )
}

/// The package `layout`: its name, the program and version it belongs to, the exchange file it carries and what that file covers.
fn layout_package(bp: &Blueprint, values: &Values) -> Artifact {
    let name = format!("{}-layout", values.get(Slot::Kebab));
    let body = format!(
        "[package]\nname = \"{name}\"\nprogram = \"{program}\"\nversion = \"{version}\"\nhost = \"{HOST}\"\n\n[[file]]\nrole = \"exchange\"\npath = \"{layout}\"\ncovers = \"{covers}\"\n",
        program = values.get(Slot::Program),
        version = values.get(Slot::Version),
        layout = bp.frontend.layout,
        covers = covers(&bp.frontend.covers),
    );
    Artifact::new(
        format!("packaging/layout/{name}.toml"),
        Class::Generated,
        body,
    )
}

/// The frontend of the program as data: the manifest and the package `layout`.
pub struct Frontend;

impl Emitter for Frontend {
    fn emit(&self, bp: &Blueprint, _: &dyn KitCatalog) -> Result<Vec<Artifact>, Coded> {
        let values = Values::of(bp)?;
        Ok(vec![manifest(bp, &values), layout_package(bp, &values)])
    }
}
