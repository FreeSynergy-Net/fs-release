//! Pattern: Value object (an error is a code plus named parameters, never an English sentence; ADR 0099 addendum 2026-11-18, point 8).

use std::fmt;

/// Every code with the names of its parameters, in the order they are set. The list of the addendum is the start, new codes are only appended.
/// The messages `blueprint-error-<code>` (de, en) in `messages.rs` exist for exactly these codes.
pub const CODES: &[(&str, &[&str])] = &[
    ("schema-unsupported", &["found", "known"]),
    ("schema-newer", &["found", "known"]),
    ("parse-error", &["line", "column"]),
    ("field-missing", &["table", "field"]),
    ("field-empty", &["table", "field"]),
    ("unknown-key", &["path"]),
    ("duplicate", &["table", "name"]),
    ("name-rule", &["name"]),
    ("name-case", &["key", "found"]),
    ("semver-invalid", &["key", "found"]),
    ("skeleton-unknown", &["found"]),
    ("skeleton-part-pending", &["part"]),
    ("capability-unknown", &["key"]),
    ("category-main-missing", &[]),
    ("contract-version-invalid", &["found"]),
    ("contract-id-invalid", &["found"]),
    ("module-unknown", &["name"]),
    ("bundle-unknown", &["name"]),
    ("version-unknown", &["module", "version"]),
    ("kind-unknown", &["module", "found"]),
    ("kind-mismatch", &["module", "declared", "catalog"]),
    ("kind-wrong-role", &["module", "expected"]),
    ("bundle-part-unknown", &["bundle", "part"]),
    ("module-in-bundle", &["module", "bundle"]),
    ("type-unknown", &["name"]),
    ("owned-by-module", &["type", "key"]),
    ("standard-missing", &["type", "field"]),
    ("standard-invalid", &["type", "found"]),
    ("media-type-invalid", &["type", "found"]),
    ("field-kind-both", &["field"]),
    ("field-kind-missing", &["field"]),
    ("kit-unknown", &["name"]),
    ("basic-unknown", &["name"]),
    ("words-unknown", &["ref"]),
    ("ruleset-source-unknown", &["name"]),
    ("ruleset-default-missing", &["ruleset"]),
    ("preset-unknown", &["ruleset", "preset"]),
    ("layout-not-found", &["path"]),
    ("covers-invalid", &["found"]),
    ("path-invalid", &["path"]),
    ("path-not-found", &["path"]),
    ("path-generated", &["path"]),
    ("path-overlap", &["path", "first", "second"]),
    ("file-unowned", &["path"]),
    ("main-reserved", &["path"]),
    ("talk-not-found", &["path"]),
    ("envelope-invalid", &["type", "found"]),
    ("generated-changed", &["path"]),
    ("generated-missing", &["path"]),
    ("seed-missing", &["path"]),
    ("slot-unknown", &["name"]),
    ("slot-open", &["path"]),
    ("header-unsupported", &["path"]),
    ("artifact-duplicate", &["path"]),
    ("exception-undeclared", &["path"]),
    ("write-failed", &["path"]),
    ("blueprint-unserializable", &[]),
    ("text-pair", &["key", "missing"]),
    ("text-vars", &["key"]),
];

/// One finding: a code of [`CODES`] and its named parameters.
///
/// Mirror of the `coded` form of fs-types (`Coded` + `Code`, tag v0.0.4): same message key (`blueprint-error-<code>`), same order of the parameters, same
/// log text. The equality is enforced by a test against a checked-in reference file (`tests/data/fs-types-coded.ref`, no crate dependency).
/// To be merged with fs-types at the first reuse.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Coded {
    pub code: &'static str,
    pub params: Vec<(&'static str, String)>,
}

impl Coded {
    #[must_use]
    pub fn new(code: &'static str) -> Self {
        Self {
            code,
            params: Vec::new(),
        }
    }

    /// Appends one named parameter.
    #[must_use]
    pub fn with(mut self, name: &'static str, value: impl Into<String>) -> Self {
        self.params.push((name, value.into()));
        self
    }

    /// The message key, as in fs-types: prefix `blueprint-error-` and the code.
    #[must_use]
    pub fn message_key(&self) -> String {
        format!("blueprint-error-{}", self.code)
    }

    /// The log text in the form of fs-types `Code`: `key (name=value, ...)`; for logs only.
    #[must_use]
    pub fn log_text(&self) -> String {
        let list: Vec<String> = self
            .params
            .iter()
            .map(|(n, v)| format!("{n}={v}"))
            .collect();
        if list.is_empty() {
            self.message_key()
        } else {
            format!("{} ({})", self.message_key(), list.join(", "))
        }
    }

    /// The value of a parameter.
    #[must_use]
    pub fn param(&self, name: &str) -> Option<&str> {
        self.params
            .iter()
            .find(|(n, _)| *n == name)
            .map(|(_, v)| v.as_str())
    }

    /// True when the code is in [`CODES`] and the parameter names are exactly the listed ones in the listed order.
    #[must_use]
    pub fn conforms(&self) -> bool {
        CODES.iter().any(|(c, names)| {
            *c == self.code
                && names.len() == self.params.len()
                && names.iter().zip(&self.params).all(|(a, (b, _))| a == b)
        })
    }
}

impl fmt::Display for Coded {
    /// `code{name=value, ...}`: a code with parameters, no sentence (the sentence is a message, `messages.rs`).
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.code)?;
        if !self.params.is_empty() {
            let list: Vec<String> = self
                .params
                .iter()
                .map(|(n, v)| format!("{n}={v}"))
                .collect();
            write!(f, "{{{}}}", list.join(", "))?;
        }
        Ok(())
    }
}

/// The result of a check: errors make the blueprint red, warnings are shown (a rule in mode `Warn`, a pending skeleton part).
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Report {
    pub errors: Vec<Coded>,
    pub warnings: Vec<Coded>,
}

impl Report {
    #[must_use]
    pub fn is_ok(&self) -> bool {
        self.errors.is_empty()
    }

    /// True when an error with this code is present.
    #[must_use]
    pub fn has(&self, code: &str) -> bool {
        self.errors.iter().any(|e| e.code == code)
    }

    /// The codes of all errors, in order.
    #[must_use]
    pub fn codes(&self) -> Vec<&'static str> {
        self.errors.iter().map(|e| e.code).collect()
    }
}
