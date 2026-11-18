//! Pattern: Strategy + port (what the check asks the outside world: the catalog of building blocks and a view of the repo; tests use fakes).

/// The three kinds of a module (ADR 0101): 1 `standard`, 2 `toolkit`, 3 `component`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Kind {
    Standard,
    Toolkit,
    Component,
}

impl Kind {
    #[must_use]
    pub fn parse(word: &str) -> Option<Kind> {
        match word {
            "standard" => Some(Kind::Standard),
            "toolkit" => Some(Kind::Toolkit),
            "component" => Some(Kind::Component),
            _ => None,
        }
    }

    #[must_use]
    pub fn as_str(self) -> &'static str {
        match self {
            Kind::Standard => "standard",
            Kind::Toolkit => "toolkit",
            Kind::Component => "component",
        }
    }
}

/// A module as the catalog knows it at the pinned state.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ModuleInfo {
    pub kind: Kind,
    /// The tags of the module repo (versions without `v`).
    pub tags: Vec<String>,
    /// Word lists the module provides.
    pub words: Vec<String>,
    /// Preset sets of its rule set (empty when it has none).
    pub presets: Vec<String>,
}

/// A bundle (a toolkit as a whole) as the catalog knows it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BundleInfo {
    pub parts: Vec<String>,
    pub tags: Vec<String>,
    pub presets: Vec<String>,
}

/// Which building blocks exist (fs-types kits and basic types, modules, bundles, skeleton versions, capabilities). Standard: lists built in per
/// skeleton version, compared with the pinned tags; a fake in the tests.
pub trait KitCatalog {
    fn skeleton_known(&self, version: &str) -> bool;
    /// Parts of the skeleton that do not exist yet in this version (reported, never silently left out).
    fn skeleton_pending(&self, version: &str) -> Vec<String>;
    fn capability_known(&self, key: &str) -> bool;
    fn module(&self, name: &str) -> Option<ModuleInfo>;
    fn bundle(&self, name: &str) -> Option<BundleInfo>;
    fn kit_known(&self, name: &str) -> bool;
    fn basic_known(&self, name: &str) -> bool;
}

/// What the check needs to see of the repo and of the talk records (paths are relative, with `/`).
pub trait RepoView {
    fn exists(&self, path: &str) -> bool;
    /// True when the path (a file, or a folder with an end slash) is or contains a generated file.
    fn covers_generated(&self, path: &str) -> bool;
    /// Hand-written files under `crates/` without generator head, not a seed, not in `tests/`.
    fn unowned_files(&self) -> Vec<String>;
    /// Every `main.rs` of the repo: the entry point is purely generated (user decision 45), so no exception may cover one.
    fn main_files(&self) -> Vec<String>;
    /// The content of a file of the repo (text), or `None` when it is not there. The drift check reads the generated files and the seeds with it.
    fn read(&self, path: &str) -> Option<String>;
    /// The talk record of an exception (a path in the documentation repo).
    fn talk_exists(&self, path: &str) -> bool;
}
