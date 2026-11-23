//! Pattern: none (the guards that need no I/O: tag run, index format).

use anyhow::{bail, ensure, Result};

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

/// `reg-index` 0.6.0 writes index format v1 only: a feature with `dep:` or `?/` would be indexed wrongly (cargo needs `features2` and `v: 2`), so it is refused.
///
/// # Errors
/// A feature value uses `dep:` or `?/`.
pub fn check_features(features: &std::collections::BTreeMap<String, Vec<String>>) -> Result<()> {
    for (name, values) in features {
        if let Some(v) = values
            .iter()
            .find(|v| v.starts_with("dep:") || v.contains("?/"))
        {
            bail!("feature `{name}` uses `{v}`: index format v2 is not supported by reg-index 0.6.0, the entry would be wrong");
        }
    }
    Ok(())
}
