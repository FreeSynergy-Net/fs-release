//! Pattern: Strategy (the emitter of `[[help]]`: the help texts of the program as one data file, `packaging/help.toml`; one entry per topic, both languages).
//!
//! A topic whose text is missing in one language is never written with an empty text: the emitter stops with `text-pair`, the check reports the same finding.

use super::artifact::{Artifact, Class, Emitter};
use crate::error::Coded;
use crate::kit::KitCatalog;
use crate::model::{Blueprint, Help};
use serde::Serialize;

/// The path of the help file of a program.
pub const PATH: &str = "packaging/help.toml";

#[derive(Serialize)]
struct File<'a> {
    help: &'a [Help],
}

/// The help texts of the program as data.
pub struct HelpTexts;

impl Emitter for HelpTexts {
    fn emit(&self, bp: &Blueprint, _: &dyn KitCatalog) -> Result<Vec<Artifact>, Coded> {
        if bp.help.is_empty() {
            return Ok(Vec::new());
        }
        for x in &bp.help {
            for (lang, t) in [("en", &x.text.en), ("de", &x.text.de)] {
                if t.is_none() {
                    return Err(Coded::new("text-pair")
                        .with("key", format!("help.{}.text", x.topic))
                        .with("missing", lang));
                }
            }
        }
        let body = toml::to_string(&File { help: &bp.help })
            .map_err(|_| Coded::new("blueprint-unserializable"))?;
        Ok(vec![Artifact::new(PATH, Class::Generated, body)])
    }
}
