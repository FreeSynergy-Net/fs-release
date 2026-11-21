//! Pattern: Value object table (what a skeleton version means: the tag the generated CI is pinned to; a new skeleton version is a new row).

/// The facts of one skeleton version.
#[derive(Debug, PartialEq, Eq)]
pub struct Skeleton {
    /// The version as `[program] skeleton` names it.
    pub id: &'static str,
    /// The tag of fs-release the CI callers of the generated program are pinned to.
    pub release_tag: &'static str,
}

/// The tag of fs-release that holds this generator: the one place the generated CI callers and the `cargo install` of fs-blueprint take it from.
/// It must name the release that contains this code; the drift check of an older tag cannot match the output of this one.
pub const RELEASE_TAG: &str = "v0.0.17";

const TABLE: &[Skeleton] = &[Skeleton {
    id: "0.1.0",
    release_tag: RELEASE_TAG,
}];

/// The skeleton of a version, or `None` when there is no such version.
#[must_use]
pub fn of(id: &str) -> Option<&'static Skeleton> {
    TABLE.iter().find(|s| s.id == id)
}
