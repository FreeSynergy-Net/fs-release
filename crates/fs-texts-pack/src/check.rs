//! The build-time check: every message in every language, the same variables, every message parses.
//!
//! Pattern: none (pure functions over the sources). Moved here from `fs-components-mf2::check_pairs` (ADR 0057: the check belongs to build time).
//! The variables are those `mf2` finds in a message (`analyze`), so two languages are compared by what they need, not by their wording.

use std::collections::BTreeMap;

use mf2_model::Message;

use crate::locale;
use crate::source::{entries, Source};

/// A parsed message and what the manifest needs to know about it (all names NFC and ascending, as `mf2` gives them).
pub(crate) struct Model {
    pub message: Message<'static>,
    pub slots: Vec<String>,
    pub markup: Vec<String>,
    pub functions: Vec<String>,
}

/// One language: its tag and its messages by id (bytewise ascending, the order of the manifest).
pub(crate) struct Language {
    pub tag: String,
    pub messages: BTreeMap<String, Model>,
}

fn read(source: &Source, problems: &mut Vec<String>) -> Language {
    let tag = &source.language;
    let (list, found) = entries(&source.text);
    problems.extend(found.into_iter().map(|p| format!("{tag}: {p}")));
    let mut messages = BTreeMap::new();
    for e in list {
        let parsed = mf2_syntax::parse_model(&e.message);
        match parsed.message {
            Some(m) if parsed.diagnostics.is_empty() => {
                let a = mf2_syntax::analyze(&m);
                let model = Model {
                    slots: a.externals.iter().map(|n| n.nfc.to_string()).collect(),
                    markup: a.markup.iter().map(|n| n.nfc.to_string()).collect(),
                    functions: a.functions.iter().map(|n| n.nfc.to_string()).collect(),
                    message: m.into_owned(),
                };
                messages.insert(e.id, model);
            }
            _ => problems.push(format!(
                "{tag}: {} (line {}): the message does not parse: {:?}",
                e.id,
                e.line,
                parsed.diagnostics.into_vec()
            )),
        }
    }
    Language {
        tag: tag.clone(),
        messages,
    }
}

/// The parsed languages of `sources`, or every problem found: a line that is no entry, an id twice, a message that does not parse, a message that
/// one language has and `en` lacks (or the reverse), a message whose variables differ from English, a language given twice, no source for `en` or `de`.
/// Every language is compared with `en`.
pub(crate) fn languages(sources: &[Source]) -> Result<Vec<Language>, Vec<String>> {
    let mut problems = Vec::new();
    let mut all: Vec<Language> = Vec::new();
    for source in sources {
        if locale::canonical(&source.language).as_deref() != Ok(source.language.as_str()) {
            problems.push(format!(
                "{:?}: not a canonical language tag",
                source.language
            ));
        } else if all.iter().any(|l| l.tag == source.language) {
            problems.push(format!("{}: given by two sources", source.language));
        } else {
            all.push(read(source, &mut problems));
        }
    }
    for need in ["en", "de"] {
        if !all.iter().any(|l| l.tag == need) {
            problems.push(format!("no source for {need}"));
        }
    }
    if let Some(english) = all.iter().find(|l| l.tag == "en") {
        for other in all.iter().filter(|l| l.tag != "en") {
            compare(english, other, &mut problems);
        }
    }
    if problems.is_empty() {
        Ok(all)
    } else {
        Err(problems)
    }
}

fn compare(english: &Language, other: &Language, problems: &mut Vec<String>) {
    let language = &other.tag;
    for (id, en) in &english.messages {
        match other.messages.get(id) {
            None => problems.push(format!("{id}: in en, missing in {language}")),
            Some(m) if m.slots != en.slots => problems.push(format!(
                "{id}: variables differ, en {:?}, {language} {:?}",
                en.slots, m.slots
            )),
            Some(_) => {}
        }
    }
    for id in other
        .messages
        .keys()
        .filter(|id| !english.messages.contains_key(*id))
    {
        problems.push(format!("{id}: in {language}, missing in en"));
    }
}
