//! Test doubles for the catalog and the repo view (only here, never in the library), and helpers to edit a blueprint text.
#![allow(dead_code)]

use fs_blueprint::emit::Emitter;
use fs_blueprint::Blueprint;
use fs_blueprint::{
    check_source, Artifact, BundleInfo, Coded, Kind, KitCatalog, ModuleInfo, RepoView, Report,
    Rules, Verified,
};
use std::collections::BTreeSet;

pub mod synth;

pub const TASKS: &str = include_str!("../data/tasks.toml");
pub const SHIFT: &str = include_str!("../data/shift.toml");

fn strings(v: &[&str]) -> Vec<String> {
    v.iter().map(|s| (*s).to_string()).collect()
}

/// A catalog with what the example blueprint needs. The targets of the example (tags of fs-todo and fs-planning) do not exist yet.
pub struct FakeCatalog;

impl KitCatalog for FakeCatalog {
    fn skeleton_known(&self, v: &str) -> bool {
        v == "0.1.0"
    }
    fn skeleton_pending(&self, _: &str) -> Vec<String> {
        strings(&["session", "update", "mcp-switch"])
    }
    fn capability_known(&self, key: &str) -> bool {
        [
            "tasks",
            "shifts",
            "dependency.evaluate",
            "dependency.cycle-check",
        ]
        .contains(&key)
    }
    fn contract_known(&self, key: &str, version: &str) -> bool {
        version == "0.1" && ["dependency.evaluate", "dependency.cycle-check"].contains(&key)
    }
    fn module(&self, name: &str) -> Option<ModuleInfo> {
        let (kind, tags, words, presets): (Kind, &[&str], &[&str], &[&str]) = match name {
            "fs-todo" => (Kind::Standard, &[], &[], &[]),
            "fs-progress" => (
                Kind::Standard,
                &["0.0.1"],
                &["state"],
                &["rfc8984", "freesynergy"],
            ),
            "fs-notes" => (Kind::Toolkit, &["0.0.1"], &[], &["plain"]),
            "board" | "task-card" => (Kind::Component, &["0.0.29"], &[], &[]),
            _ => return None,
        };
        Some(ModuleInfo {
            kind,
            tags: strings(tags),
            words: strings(words),
            presets: strings(presets),
        })
    }
    fn bundle(&self, name: &str) -> Option<BundleInfo> {
        (name == "fs-planning").then(|| BundleInfo {
            parts: strings(&[
                "readiness",
                "schedule",
                "progress",
                "rank",
                "checklist",
                "claim",
            ]),
            tags: Vec::new(),
            presets: strings(&["rfc8984", "freesynergy"]),
        })
    }
    fn kit_known(&self, name: &str) -> bool {
        ["timeSpan", "personRef"].contains(&name)
    }
    fn basic_known(&self, name: &str) -> bool {
        ["text", "number", "choice", "keywords"].contains(&name)
    }
}

/// A repo view with the files of the example; the talk records are none unless a test adds one.
#[derive(Default)]
pub struct FakeView {
    pub files: BTreeSet<String>,
    pub generated: Vec<String>,
    pub unowned: Vec<String>,
    pub talks: BTreeSet<String>,
    pub contents: std::collections::BTreeMap<String, String>,
}

impl FakeView {
    #[must_use]
    pub fn standard() -> Self {
        let mut v = FakeView::default();
        for f in [
            "forms/board.toml",
            "forms/list.toml",
            "crates/fs-tasks-core/src/secret_guard.rs",
            "crates/fs-tasks-core/src/extra.rs",
            "crates/fs-tasks-core/src/lib.rs",
            "crates/fs-tasks-bin/src/main.rs",
        ] {
            v.files.insert(f.to_string());
        }
        v.generated
            .push("crates/fs-tasks-core/src/lib.rs".to_string());
        v.generated
            .push("crates/fs-tasks-bin/src/main.rs".to_string());
        v
    }
    #[must_use]
    pub fn with_talk(mut self) -> Self {
        self.talks
            .insert("conventions/talks/2026-11-xx-tasks-secret-guard.md".to_string());
        self
    }
}

impl RepoView for FakeView {
    fn exists(&self, path: &str) -> bool {
        self.files.contains(path)
            || self
                .files
                .iter()
                .any(|f| path.ends_with('/') && f.starts_with(path))
    }
    fn covers_generated(&self, path: &str) -> bool {
        self.generated
            .iter()
            .any(|g| g == path || (path.ends_with('/') && g.starts_with(path)))
    }
    fn unowned_files(&self) -> Vec<String> {
        self.unowned.clone()
    }
    fn main_files(&self) -> Vec<String> {
        self.files
            .iter()
            .filter(|f| f.ends_with("/main.rs"))
            .cloned()
            .collect()
    }
    fn read(&self, path: &str) -> Option<String> {
        self.contents.get(path).cloned()
    }
    fn talk_exists(&self, path: &str) -> bool {
        self.talks.contains(path)
    }
}

/// Checks a text with the fakes (the talk record exists when `talk` is true) and the rule set `projekt`.
pub fn run(src: &str, talk: bool) -> Report {
    run_with(
        src,
        if talk {
            FakeView::standard().with_talk()
        } else {
            FakeView::standard()
        },
        &Rules::projekt(),
    )
}

#[allow(clippy::needless_pass_by_value)]
pub fn run_with(src: &str, view: FakeView, rules: &Rules) -> Report {
    check_source(src, &FakeCatalog, &view, rules)
}

/// `src` with the one place `from` replaced by `to`; panics when `from` is not there exactly once (a test must really change something).
pub fn edit(src: &str, from: &str, to: &str) -> String {
    assert_eq!(
        src.matches(from).count(),
        1,
        "the text to change must appear exactly once: {from:?}"
    );
    src.replacen(from, to, 1)
}

/// The codes of the errors of a text (talk record present, so the talk code does not hide anything).
pub fn codes(src: &str) -> Vec<&'static str> {
    run(src, true).codes()
}

/// How many errors of a code a report has.
pub fn only(report: &Report, code: &str) -> usize {
    report.errors.iter().filter(|e| e.code == code).count()
}

impl FakeView {
    /// A view whose files are exactly the contents of a store (for the drift check).
    #[must_use]
    pub fn of_store(store: &synth::MemStore) -> Self {
        FakeView {
            contents: store.files.clone(),
            ..FakeView::default()
        }
    }
}

/// A catalog that knows nothing (with the rules off nothing is asked of it) and a view where every file is there: the check that lets a test blueprint pass to the generator.
pub struct Lenient;

impl KitCatalog for Lenient {
    fn skeleton_known(&self, _: &str) -> bool {
        true
    }
    fn skeleton_pending(&self, _: &str) -> Vec<String> {
        Vec::new()
    }
    fn capability_known(&self, _: &str) -> bool {
        true
    }
    fn contract_known(&self, _: &str, _: &str) -> bool {
        true
    }
    fn module(&self, _: &str) -> Option<ModuleInfo> {
        None
    }
    fn bundle(&self, _: &str) -> Option<BundleInfo> {
        None
    }
    fn kit_known(&self, _: &str) -> bool {
        false
    }
    fn basic_known(&self, _: &str) -> bool {
        false
    }
}

pub struct AllView;

impl RepoView for AllView {
    fn exists(&self, _: &str) -> bool {
        true
    }
    fn covers_generated(&self, _: &str) -> bool {
        false
    }
    fn unowned_files(&self) -> Vec<String> {
        Vec::new()
    }
    fn main_files(&self) -> Vec<String> {
        Vec::new()
    }
    fn read(&self, _: &str) -> Option<String> {
        None
    }
    fn talk_exists(&self, _: &str) -> bool {
        true
    }
}

/// The rule set `projekt` with every rule off: only the structure is checked.
pub fn rules_off() -> Rules {
    fs_blueprint::Rule::ALL
        .into_iter()
        .fold(Rules::projekt(), |r, rule| {
            r.with(rule, fs_blueprint::Mode::Off)
        })
}

/// Verifies a test blueprint (structure only) and hands it to `f`; a structure error is the first code.
pub fn with_verified<T>(
    bp: &Blueprint,
    f: impl FnOnce(&Verified) -> Result<T, Coded>,
) -> Result<T, Coded> {
    match fs_blueprint::verify(bp, &Lenient, &AllView, &rules_off()) {
        Ok(v) => f(&v),
        Err(report) => Err(report.errors[0].clone()),
    }
}

/// `generate` as the tests call it: after the check of the structure (the generator takes nothing else).
pub fn generate(bp: &Blueprint, catalog: &dyn KitCatalog) -> Result<Vec<Artifact>, Coded> {
    with_verified(bp, |v| fs_blueprint::emit::generate(v, catalog))
}

pub fn generate_with(
    bp: &Blueprint,
    catalog: &dyn KitCatalog,
    emitters: &[&dyn Emitter],
) -> Result<Vec<Artifact>, Coded> {
    with_verified(bp, |v| {
        fs_blueprint::emit::generate::generate_with(v, catalog, emitters)
    })
}

/// The drift check of a test blueprint.
pub fn drift(bp: &Blueprint, catalog: &dyn KitCatalog, view: &dyn RepoView) -> Report {
    with_verified(bp, |v| {
        Ok(fs_blueprint::emit::check::check(v, catalog, view))
    })
    .unwrap_or_else(|c| Report {
        errors: vec![c],
        warnings: Vec::new(),
    })
}
