//! Pattern: none (the guards that need no I/O: tag run, index format).

use std::path::PathBuf;

use anyhow::{bail, ensure, Result};
use reg_index::IndexPackage;

/// Version of a tag run: `refs/tags/v0.0.7` -> `0.0.7`. Everything else is refused.
///
/// # Errors
/// The ref is not a tag `vX.Y.Z` (a branch, a pull request, an empty value, a pre-release or build suffix).
pub fn version_of_ref(git_ref: &str) -> Result<String> {
    let Some(tag) = git_ref.strip_prefix("refs/tags/v") else {
        bail!("`{git_ref}` is not a tag run (refs/tags/vX.Y.Z): the registry is written only from a tag, never from a branch or by hand");
    };
    let parts: Vec<&str> = tag.split('.').collect();
    let plain = parts.len() == 3
        && parts.iter().all(|p| {
            !p.is_empty()
                && p.bytes().all(|b| b.is_ascii_digit())
                && (p.len() == 1 || !p.starts_with('0'))
        });
    ensure!(
        plain,
        "tag `v{tag}` is not vX.Y.Z (digits only, no pre-release, no build suffix)"
    );
    Ok(tag.to_string())
}

/// Index format v2 (registry-index standard): a feature whose values use `dep:` or `?/` belongs in `features2`, and the line carries `v: 2`
/// (cargo before 1.60 skips such a line, newer cargo merges `features` and `features2`). `reg-index` 0.6.0 writes only format v1, so the adapter
/// corrects the line it got. Returns whether the line was changed.
pub fn split_v2(pkg: &mut IndexPackage) -> bool {
    let needs_v2 = |values: &Vec<String>| {
        values
            .iter()
            .any(|v| v.starts_with("dep:") || v.contains("?/"))
    };
    let moved: Vec<String> = pkg
        .features
        .iter()
        .filter(|(_, v)| needs_v2(v))
        .map(|(k, _)| k.clone())
        .collect();
    if moved.is_empty() {
        return false;
    }
    let mut f2 = pkg.features2.take().unwrap_or_default();
    for k in moved {
        if let Some(v) = pkg.features.remove(&k) {
            f2.insert(k, v);
        }
    }
    pkg.features2 = Some(f2);
    pkg.v = Some(2);
    true
}

/// Repo-relative path of the index file of a crate (layout of the cargo registry index: `1/a`, `2/ab`, `3/a/abc`, `ab/cd/abcd...`).
#[must_use]
pub fn index_file(name: &str) -> PathBuf {
    let n = name.to_lowercase();
    match n.len() {
        1 => PathBuf::from("1").join(&n),
        2 => PathBuf::from("2").join(&n),
        3 => PathBuf::from("3").join(&n[..1]).join(&n),
        _ => PathBuf::from(&n[0..2]).join(&n[2..4]).join(&n),
    }
}
