//! Pattern: Value object table (what a skeleton version means: the tags of the building blocks the generated code is pinned to; a new skeleton version is a new row).

/// The facts of one skeleton version.
#[derive(Debug, PartialEq, Eq)]
pub struct Skeleton {
    /// The version as `[program] skeleton` names it.
    pub id: &'static str,
    /// The tag of fs-rpc (it pins fs-types, fs-interfaces and fs-objects itself).
    pub rpc_tag: &'static str,
    /// The tag of fs-release the CI callers of the generated program are pinned to.
    pub release_tag: &'static str,
}

const TABLE: &[Skeleton] = &[Skeleton {
    id: "0.1.0",
    rpc_tag: "v0.0.2",
    release_tag: "v0.0.15",
}];

/// The skeleton of a version, or `None` when there is no such version.
#[must_use]
pub fn of(id: &str) -> Option<&'static Skeleton> {
    TABLE.iter().find(|s| s.id == id)
}
