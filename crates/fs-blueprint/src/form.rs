//! Pattern: Value object checks (the forms of names, versions, ids, texts; pure functions, no catalog needed).

use crate::model::Text2;
use std::collections::BTreeSet;

fn starts_lower(s: &str) -> bool {
    s.chars().next().is_some_and(|c| c.is_ascii_lowercase())
}

/// `^[a-z][a-z0-9-]*$`
#[must_use]
pub fn is_kebab(s: &str) -> bool {
    starts_lower(s)
        && s.chars()
            .all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == '-')
}

/// `PascalCase`: starts upper case, letters and digits only.
#[must_use]
pub fn is_pascal(s: &str) -> bool {
    s.chars().next().is_some_and(|c| c.is_ascii_uppercase())
        && s.chars().all(|c| c.is_ascii_alphanumeric())
}

/// `camelCase`: starts lower case, letters and digits only.
#[must_use]
pub fn is_camel(s: &str) -> bool {
    starts_lower(s) && s.chars().all(|c| c.is_ascii_alphanumeric())
}

fn number(s: &str) -> bool {
    !s.is_empty() && s.chars().all(|c| c.is_ascii_digit()) && (s == "0" || !s.starts_with('0'))
}

/// `major.minor.patch`, plain numbers (no pre-release: a version is a tag of a repo that follows the version = tag rule).
#[must_use]
pub fn is_semver(s: &str) -> bool {
    let parts: Vec<&str> = s.split('.').collect();
    parts.len() == 3 && parts.iter().all(|p| number(p))
}

/// `major.minor`
#[must_use]
pub fn is_major_minor(s: &str) -> bool {
    let parts: Vec<&str> = s.split('.').collect();
    parts.len() == 2 && parts.iter().all(|p| number(p))
}

/// `0x` and 16 hexadecimal digits (a Cap'n Proto file id).
#[must_use]
pub fn is_file_id(s: &str) -> bool {
    s.strip_prefix("0x")
        .is_some_and(|h| h.len() == 16 && h.chars().all(|c| c.is_ascii_hexdigit()))
}

/// `type/subtype` after RFC 6838, as far as the form goes (no parameters).
#[must_use]
pub fn is_media_type(s: &str) -> bool {
    let token = |t: &str| {
        !t.is_empty()
            && t.chars()
                .all(|c| c.is_ascii_alphanumeric() || "!#$&-^_.+".contains(c))
    };
    s.split_once('/')
        .is_some_and(|(a, b)| token(a) && token(b) && !b.contains('/'))
}

/// A URI of a standard with a section: `https://...#section`.
#[must_use]
pub fn is_standard_uri(s: &str) -> bool {
    (s.starts_with("https://") || s.starts_with("http://"))
        && s.split_once('#')
            .is_some_and(|(h, f)| h.len() > 8 && !f.is_empty())
}

/// The types of the `ActivityStreams` 2.0 vocabulary (W3C Activity Vocabulary: core, activity, actor and object types): the envelope of a type is one of them or a full IRI.
pub const AS2_TYPES: &[&str] = &[
    "Object",
    "Link",
    "Activity",
    "IntransitiveActivity",
    "Collection",
    "OrderedCollection",
    "CollectionPage",
    "OrderedCollectionPage",
    "Accept",
    "Add",
    "Announce",
    "Arrive",
    "Block",
    "Create",
    "Delete",
    "Dislike",
    "Flag",
    "Follow",
    "Ignore",
    "Invite",
    "Join",
    "Leave",
    "Like",
    "Listen",
    "Move",
    "Offer",
    "Question",
    "Read",
    "Reject",
    "Remove",
    "TentativeAccept",
    "TentativeReject",
    "Travel",
    "Undo",
    "Update",
    "View",
    "Application",
    "Group",
    "Organization",
    "Person",
    "Service",
    "Article",
    "Audio",
    "Document",
    "Event",
    "Image",
    "Note",
    "Page",
    "Place",
    "Profile",
    "Relationship",
    "Tombstone",
    "Video",
    "Mention",
];

/// A full IRI: a scheme, `://` and a rest without white space (the envelope of an extension type).
#[must_use]
pub fn is_iri(s: &str) -> bool {
    s.split_once("://").is_some_and(|(scheme, rest)| {
        scheme
            .chars()
            .next()
            .is_some_and(|c| c.is_ascii_alphabetic())
            && scheme
                .chars()
                .all(|c| c.is_ascii_alphanumeric() || "+-.".contains(c))
            && !rest.is_empty()
            && !s.contains(char::is_whitespace)
    })
}

/// An entry of `covers`: `Interface.method` or `Type.field` (`Pascal.camel`).
#[must_use]
pub fn is_covers_entry(s: &str) -> bool {
    s.split_once('.')
        .is_some_and(|(a, b)| is_pascal(a) && is_camel(b))
}

/// The variables `{$name}` of an MF2 message (see [`crate::mf2::variables`]).
#[must_use]
pub fn variables(message: &str) -> BTreeSet<String> {
    crate::mf2::variables(message)
}

/// The names Windows reserves for devices: they cannot be a file or folder name, with or without an extension (`NUL`, `nul.txt`, `Com1`, `CONIN$`, `COM¹`).
fn is_reserved_name(segment: &str) -> bool {
    let stem = segment.split('.').next().unwrap_or(segment).to_uppercase();
    matches!(
        stem.as_str(),
        "CON" | "PRN" | "AUX" | "NUL" | "CONIN$" | "CONOUT$"
    ) || ["COM", "LPT"].iter().any(|p| {
        // COM1..COM9 and LPT1..LPT9, also with the superscript digits 1 2 3 that Windows reserves too
        stem.strip_prefix(p).is_some_and(|n| {
            let mut c = n.chars();
            matches!(
                (c.next(), c.next()),
                (Some('1'..='9' | '¹' | '²' | '³'), None)
            )
        })
    })
}

/// One part of a path: not `.`/`..`, no character that Windows (or any shell) refuses, no end in a dot or a space (Windows drops it, so two names
/// would be one file), no reserved device name.
fn is_repo_segment(segment: &str) -> bool {
    !matches!(segment, "." | "..")
        && !segment.ends_with(['.', ' '])
        && !segment.contains(['\\', '*', '?', ':', '"', '<', '>', '|', '\0'])
        && !segment.chars().any(char::is_control)
        && !is_reserved_name(segment)
}

/// A relative path with `/` that is the same file on every platform: not empty, no leading `/`, no `\`, no `..`, no drive colon or `\\?\` prefix, no `*` or `?`,
/// no segment Windows would change or refuse (see [`is_repo_segment`]). An end slash means a folder.
#[must_use]
pub fn is_repo_path(p: &str) -> bool {
    let parts: Vec<&str> = p.split('/').collect();
    let last = parts.len() - 1;
    !p.is_empty()
        && !p.starts_with('/')
        && parts.iter().enumerate().all(|(i, c)| {
            (c.is_empty() && i == last && i > 0) || (!c.is_empty() && is_repo_segment(c))
        })
}

/// What is wrong with a [`Text2`]: the missing languages and whether the variables differ.
#[derive(Debug, PartialEq, Eq)]
pub enum TextFault {
    Missing(&'static str),
    Vars,
    /// Not a simple MF2 message (the language).
    Syntax(&'static str),
}

/// The faults of a pair of texts (`de` and `en` both there, the same variables in both).
#[must_use]
pub fn text_faults(t: &Text2) -> Vec<TextFault> {
    let mut out = Vec::new();
    if t.en.as_deref().is_none_or(str::is_empty) {
        out.push(TextFault::Missing("en"));
    }
    if t.de.as_deref().is_none_or(str::is_empty) {
        out.push(TextFault::Missing("de"));
    }
    for (lang, text) in [("en", &t.en), ("de", &t.de)] {
        if text.as_deref().is_some_and(|m| !crate::mf2::is_simple(m)) {
            out.push(TextFault::Syntax(lang));
        }
    }
    if let (Some(en), Some(de)) = (&t.en, &t.de) {
        if !en.is_empty() && !de.is_empty() && variables(en) != variables(de) {
            out.push(TextFault::Vars);
        }
    }
    out
}
