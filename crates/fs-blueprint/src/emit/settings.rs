//! Pattern: Strategy (the emitter of `[[setting]]`: the settings of the program as one data file, `packaging/settings.toml`; no widget, the frontend shows them).
//!
//! The file is the table as the blueprint states it (key, type, default, label in both languages); a program without settings gets no file.

use super::artifact::{Artifact, Class, Emitter};
use crate::error::Coded;
use crate::kit::KitCatalog;
use crate::model::{Blueprint, Setting};
use serde::Serialize;

/// The path of the settings file of a program.
pub const PATH: &str = "packaging/settings.toml";

#[derive(Serialize)]
struct File<'a> {
    setting: &'a [Setting],
}

/// The settings of the program as data.
pub struct Settings;

impl Emitter for Settings {
    fn emit(&self, bp: &Blueprint, _: &dyn KitCatalog) -> Result<Vec<Artifact>, Coded> {
        if bp.setting.is_empty() {
            return Ok(Vec::new());
        }
        // a label that is missing in one language stops here: the file never carries an empty text
        for x in &bp.setting {
            for (lang, t) in [("en", &x.label.en), ("de", &x.label.de)] {
                if t.is_none() {
                    return Err(Coded::new("text-pair")
                        .with("key", format!("setting.{}.label", x.key))
                        .with("missing", lang));
                }
            }
        }
        let body = toml::to_string(&File {
            setting: &bp.setting,
        })
        .map_err(|_| Coded::new("blueprint-unserializable"))?;
        Ok(vec![Artifact::new(PATH, Class::Generated, body)])
    }
}
