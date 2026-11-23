//! Pattern: Specification (the ONE place that names the owners of our repos; every owner comparison of the chain asks it).
//!
//! Security boundary: only these two owners count as "ours". It is never "any owner". The old owner stays until part 2 of the
//! address wave is done (a freed name could serve old tag pins to strangers); then this list loses it in one edit.

use crate::slug;

/// The owners of our repos: the new one first, the old one second.
pub const OWNERS: [&str; 2] = ["FreeSynergy-Net", "FreeSynergy"];

/// The name of the release repo (the only repo whose `.fs/preapproved.toml` counts).
pub const RELEASE_REPO: &str = "fs-release";

/// The owner part of a slug (`FreeSynergy-Net` of `FreeSynergy-Net/fs-x`), when the slug is valid.
fn owner_of(s: &str) -> Option<&str> {
    slug::validate(s).ok()?;
    s.split_once('/').map(|(o, _)| o)
}

/// True when the slug is valid and its owner is one of [`OWNERS`] (exact, case sensitive).
#[must_use]
pub fn is_ours(s: &str) -> bool {
    owner_of(s).is_some_and(|o| OWNERS.contains(&o))
}

/// True for the release repo under either owner (`FreeSynergy/fs-release`, `FreeSynergy-Net/fs-release`), nothing else.
#[must_use]
pub fn is_release_repo(s: &str) -> bool {
    is_ours(s) && slug::name(s) == RELEASE_REPO
}

/// True when both slugs are the same repo: identical, or both ours (either owner) with the same name. A slug of any other owner
/// only ever equals itself.
#[must_use]
pub fn same_repo(a: &str, b: &str) -> bool {
    a == b || (is_ours(a) && is_ours(b) && slug::name(a) == slug::name(b))
}
