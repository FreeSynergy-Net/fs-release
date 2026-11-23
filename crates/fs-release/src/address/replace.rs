//! Pattern: Command (rewrites the work tree of one repo, only the safe forms; no commit, no push).

use std::path::Path;
use std::process::Command;

use super::allow::Allow;
use super::class::Class;
use super::mention::swap_safe;
use super::scan::repo_name;

/// What one run changed.
#[derive(Debug, Default, PartialEq, Eq)]
pub struct Report {
    pub files: usize,
    pub lines: usize,
    /// Files with an owner mention that were left alone because of their class (Cargo sources, lock files, catalog data).
    pub left_by_class: usize,
}

/// Swaps `old` for `new` in the safe forms of all tracked text files of `dir`.
///
/// Never touches Cargo sources (`Cargo.toml`, `.cargo/`, `deny.toml`, `scripts/lock-sources.toml`, ...), `Cargo.lock` and the
/// signed catalog data; lines that `allow` covers stay. A repo without anything to change is `Ok` with zero files.
///
/// # Errors
/// Not a git repo, the repo is `Catalog` (only the `katalog` workflow changes it), or a file cannot be read or written.
pub fn replace_tree(dir: &Path, old: &str, new: &str, allow: &mut Allow) -> Result<Report, String> {
    let repo = repo_name(dir)?;
    if repo == "Catalog" {
        return Err(
            "Catalog: refused, the catalog data change only through the workflow `katalog`".into(),
        );
    }
    let list = Command::new("git")
        .arg("-C")
        .arg(dir)
        .args(["ls-files", "-z"])
        .output()
        .map_err(|e| format!("git ls-files: {e}"))?;
    if !list.status.success() {
        return Err(format!("{repo}: git ls-files failed"));
    }
    let mut rep = Report::default();
    for rel in String::from_utf8_lossy(&list.stdout)
        .split('\0')
        .filter(|s| !s.is_empty())
    {
        let full = dir.join(rel);
        let Ok(raw) = std::fs::read(&full) else {
            continue;
        };
        let Ok(text) = String::from_utf8(raw) else {
            continue;
        };
        if !text
            .to_ascii_lowercase()
            .contains(&old.to_ascii_lowercase())
        {
            continue;
        }
        if Class::of(rel).replace_forbidden() {
            rep.left_by_class += 1;
            continue;
        }
        if let Some(changed) = rewrite(&text, rel, &repo, old, new, allow, &mut rep) {
            std::fs::write(&full, changed).map_err(|e| format!("{}: {e}", full.display()))?;
            rep.files += 1;
        }
    }
    Ok(rep)
}

fn rewrite(
    text: &str,
    rel: &str,
    repo: &str,
    old: &str,
    new: &str,
    allow: &mut Allow,
    rep: &mut Report,
) -> Option<String> {
    let mut out = String::with_capacity(text.len());
    let mut changed = false;
    for line in text.split_inclusive('\n') {
        let (body, eol) = line.strip_suffix('\n').map_or((line, ""), |b| (b, "\n"));
        match swap_safe(body, old, new) {
            Some(n) if !allow.allows(repo, rel, body) => {
                out.push_str(&n);
                rep.lines += 1;
                changed = true;
            }
            _ => out.push_str(body),
        }
        out.push_str(eol);
    }
    changed.then_some(out)
}
