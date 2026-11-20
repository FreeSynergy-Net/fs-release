//! Pattern: Template method (the check of the two data tables `[[setting]]` and `[[help]]`: names unique, types known, texts in both languages).

use crate::error::Coded;
use crate::model::Blueprint;
use crate::rules::Rule;
use crate::sink::Sink;

/// `[[setting]]`: a key once, a basic type of the catalog, a label in both languages.
pub fn settings(s: &mut Sink, bp: &Blueprint) {
    s.unique("setting", bp.setting.iter().map(|x| x.key.as_str()));
    for x in &bp.setting {
        if x.key.is_empty() {
            s.err(
                Coded::new("field-empty")
                    .with("table", "setting")
                    .with("field", "key"),
            );
        }
        if !s.catalog.basic_known(&x.type_name) {
            s.rule(
                Rule::ExistingOnly,
                Coded::new("basic-unknown").with("name", x.type_name.as_str()),
            );
        }
        s.text(&format!("setting.{}.label", x.key), &x.label);
    }
}

/// `[[help]]`: a topic once, a text in both languages (a missing one is an error, never an empty text).
pub fn help(s: &mut Sink, bp: &Blueprint) {
    s.unique("help", bp.help.iter().map(|x| x.topic.as_str()));
    for x in &bp.help {
        if x.topic.is_empty() {
            s.err(
                Coded::new("field-empty")
                    .with("table", "help")
                    .with("field", "topic"),
            );
        }
        s.text(&format!("help.{}.text", x.topic), &x.text);
    }
}
