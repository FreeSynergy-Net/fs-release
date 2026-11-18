//! Pattern: Template method (`generate` runs every emitter, stamps the heads, sorts by path and sets the class from the blueprint).

use super::artifact::{Artifact, Class, Emitter};
use super::{files, header};
use crate::check_exc::covers;
use crate::error::Coded;
use crate::form::is_repo_path;
use crate::kit::KitCatalog;
use crate::model::Blueprint;
use crate::verified::Verified;
use std::collections::{BTreeMap, BTreeSet};

/// True for a `main.rs` anywhere in the repo.
fn is_main(path: &str) -> bool {
    path == "main.rs" || path.ends_with("/main.rs")
}

/// The finding for a generated file that an exception covers: `main-reserved` for an entry point, `path-generated` for the rest.
fn taken(path: &str) -> Coded {
    let code = if is_main(path) {
        "main-reserved"
    } else {
        "path-generated"
    };
    Coded::new(code).with("path", path)
}

/// The emitters of the generator, in a fixed order.
#[must_use]
pub fn emitters() -> Vec<Box<dyn Emitter>> {
    files::all()
}

/// Generates the whole output of a blueprint, sorted by path.
///
/// # Errors
/// The first code of a failing emitter, `main-reserved{path}` or `path-generated{path}` (an exception covers a generated file), `line-ending{path}`, `path-invalid`, `artifact-duplicate{path}`, `header-unsupported{path}` or `exception-undeclared{path}`.
pub fn generate(bp: &Verified, catalog: &dyn KitCatalog) -> Result<Vec<Artifact>, Coded> {
    let list = emitters();
    let refs: Vec<&dyn Emitter> = list.iter().map(AsRef::as_ref).collect();
    generate_with(bp, catalog, &refs)
}

/// The paths the generator makes for a blueprint, whether or not it is checked yet (the check needs them to tell an exception over a generated file).
/// Empty when an emitter refuses the blueprint; an exception of the blueprint does not change it (that is what the check looks at).
#[must_use]
pub fn planned_paths(bp: &Blueprint, catalog: &dyn KitCatalog) -> Vec<String> {
    let mut paths: Vec<String> = emitters()
        .iter()
        .filter_map(|e| e.emit(bp, catalog).ok())
        .flatten()
        .map(|a| a.path)
        .collect();
    paths.sort();
    paths
}

/// [`generate`] with the emitters given (tests put their own in). Only a blueprint that passed the check ([`Verified`]) is taken.
///
/// # Errors
/// As [`generate`].
pub fn generate_with(
    bp: &Verified,
    catalog: &dyn KitCatalog,
    emitters: &[&dyn Emitter],
) -> Result<Vec<Artifact>, Coded> {
    build(bp.blueprint(), catalog, emitters)
}

fn build(
    bp: &Blueprint,
    catalog: &dyn KitCatalog,
    emitters: &[&dyn Emitter],
) -> Result<Vec<Artifact>, Coded> {
    let hash = header::blueprint_hash(bp)?;
    let mut sorted: BTreeMap<String, Artifact> = BTreeMap::new();
    // Windows and macOS see `A.rs` and `a.rs` as one file
    let mut lower: BTreeSet<String> = BTreeSet::new();
    let mut raw: Vec<Artifact> = Vec::new();
    for e in emitters {
        raw.extend(e.emit(bp, catalog)?);
    }
    // the entry point first: a folder that holds it is `main-reserved`, not the first generated file in it
    let covered_by_exception = |path: &str| {
        bp.exception
            .iter()
            .any(|x| x.paths.iter().any(|p| covers(p, path)))
    };
    if let Some(a) = raw
        .iter()
        .find(|a| a.class == Class::Generated && is_main(&a.path) && covered_by_exception(&a.path))
    {
        return Err(taken(&a.path));
    }
    {
        for a in raw {
            if !is_repo_path(&a.path) {
                return Err(Coded::new("path-invalid").with("path", a.path));
            }
            let covered = covered_by_exception(&a.path);
            let class = match (covered, a.class) {
                // an exception cannot take a file of the generator: the entry point is reserved (decision 45), the rest is a generated file
                (true, Class::Generated) => return Err(taken(&a.path)),
                (true, _) => Class::Exception,
                (false, Class::Exception) => {
                    return Err(Coded::new("exception-undeclared").with("path", a.path));
                }
                (false, c) => c,
            };
            let content = header::stamp(&a.path, class, &hash, &a.content)?;
            if content.contains('\r') {
                return Err(Coded::new("line-ending").with("path", a.path));
            }
            let art = Artifact::new(a.path.clone(), class, content);
            if !lower.insert(a.path.to_lowercase()) || sorted.insert(a.path.clone(), art).is_some()
            {
                return Err(Coded::new("artifact-duplicate").with("path", a.path));
            }
        }
    }
    Ok(sorted.into_values().collect())
}
