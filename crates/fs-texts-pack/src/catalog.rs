//! The catalog of one language and the manifest all languages share (ADR 0057, mf2 3.0.0).
//!
//! Pattern: Builder (manifest, plural and number data, messages -> the `.mf2b` bytes). Everything below is the writer of `mf2-catalog`; the
//! sequence is the one `mf2::compile_str` follows for one message, here for all messages of a language at once. Output is deterministic
//! (the writer depends only on its input), so a second run gives the same bytes.

use std::collections::BTreeSet;

use mf2_catalog::writer::{catalog, Options};
use mf2_catalog::{content_hash, Manifest};
use mf2_locale_data::number::NumberNeeds;
use mf2_locale_data::{direction, locale_entries, LocaleNeeds, CLDR_VERSION};

use crate::check::Language;
use crate::error::PackError;

fn library(what: &str, why: impl std::fmt::Debug) -> PackError {
    PackError::Library(format!("{what}: {why:?}"))
}

/// The manifest of all languages: the ids of English (the pair check made every language equal), per id its slots, the markup names and
/// the function names of every language put together.
pub(crate) fn manifest(languages: &[Language]) -> Manifest {
    let mut ids = BTreeSet::new();
    let mut functions = BTreeSet::new();
    for l in languages {
        ids.extend(l.messages.keys().cloned());
        for m in l.messages.values() {
            functions.extend(m.functions.iter().cloned());
        }
    }
    let ids: Vec<String> = ids.into_iter().collect();
    let (mut slots, mut markup) = (Vec::new(), Vec::new());
    for id in &ids {
        let mut names = BTreeSet::new();
        for m in languages.iter().filter_map(|l| l.messages.get(id)) {
            names.extend(m.markup.iter().cloned());
        }
        slots.push(
            languages
                .iter()
                .find_map(|l| l.messages.get(id))
                .map_or_else(Vec::new, |m| m.slots.clone()),
        );
        markup.push(names.into_iter().collect());
    }
    Manifest {
        ids,
        slots,
        markup,
        functions: functions.into_iter().collect(),
    }
}

/// The `.mf2b` bytes of `language` against `manifest`: production form (COLD and IDS stripped, the manifest names the ids), with the plural rules
/// and the number data its messages need.
pub(crate) fn write(language: &Language, manifest: &Manifest) -> Result<Vec<u8>, PackError> {
    let tag = language.tag.as_str();
    let mut options = Options::new(tag, direction(tag).map_err(|e| library(tag, e))?);
    options.cldr_version = Some(CLDR_VERSION);
    let mut numbers = NumberNeeds::default();
    for m in language.messages.values() {
        numbers.add_message(&m.message);
    }
    numbers.symbols = true;
    let mut needs = LocaleNeeds::default();
    needs.cardinal = true;
    needs.ordinal = true;
    needs.numbers = numbers;
    options.locale_entries = locale_entries(tag, &needs).map_err(|e| library(tag, e))?;
    let messages: Vec<_> = manifest
        .ids
        .iter()
        .map(|id| language.messages.get(id).map(|m| &m.message))
        .collect();
    catalog(manifest, &messages, &options.stripped()).map_err(|e| library(tag, e))
}

/// The file name of a catalog: `<locale>.<hash>.mf2b`, the hash of its bytes (a catalog is served under it, immutable).
pub(crate) fn file_name(tag: &str, bytes: &[u8]) -> String {
    format!("{tag}.{}.mf2b", content_hash(bytes))
}
