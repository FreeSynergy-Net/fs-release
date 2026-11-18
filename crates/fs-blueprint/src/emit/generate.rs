//! Pattern: Template method (`generate` runs every emitter, stamps the heads, sorts by path and sets the class from the blueprint).

use super::artifact::{Artifact, Class, Emitter};
use super::{entry, header};
use crate::check_exc::covers;
use crate::error::Coded;
use crate::form::is_repo_path;
use crate::kit::KitCatalog;
use crate::model::Blueprint;
use std::collections::BTreeMap;

/// The finding for a generated file that an exception covers: `main-reserved` for an entry point, `path-generated` for the rest.
fn taken(path: &str) -> Coded {
    let code = if path == "main.rs" || path.ends_with("/main.rs") {
        "main-reserved"
    } else {
        "path-generated"
    };
    Coded::new(code).with("path", path)
}

/// The emitters of the generator, in a fixed order.
#[must_use]
pub fn emitters() -> Vec<Box<dyn Emitter>> {
    entry::all()
}

/// Generates the whole output of a blueprint, sorted by path.
///
/// # Errors
/// The first code of a failing emitter, `main-reserved{path}` or `path-generated{path}` (an exception covers a generated file), `line-ending{path}`, `path-invalid`, `artifact-duplicate{path}`, `header-unsupported{path}` or `exception-undeclared{path}`.
pub fn generate(bp: &Blueprint, catalog: &dyn KitCatalog) -> Result<Vec<Artifact>, Coded> {
    let list = emitters();
    let refs: Vec<&dyn Emitter> = list.iter().map(AsRef::as_ref).collect();
    generate_with(bp, catalog, &refs)
}

/// [`generate`] with the emitters given (tests put their own in).
///
/// # Errors
/// As [`generate`].
pub fn generate_with(
    bp: &Blueprint,
    catalog: &dyn KitCatalog,
    emitters: &[&dyn Emitter],
) -> Result<Vec<Artifact>, Coded> {
    let hash = header::blueprint_hash(bp)?;
    let mut sorted: BTreeMap<String, Artifact> = BTreeMap::new();
    for e in emitters {
        for a in e.emit(bp, catalog)? {
            if !is_repo_path(&a.path) {
                return Err(Coded::new("path-invalid").with("path", a.path));
            }
            let covered = bp
                .exception
                .iter()
                .any(|x| x.paths.iter().any(|p| covers(p, &a.path)));
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
            if sorted.insert(a.path.clone(), art).is_some() {
                return Err(Coded::new("artifact-duplicate").with("path", a.path));
            }
        }
    }
    Ok(sorted.into_values().collect())
}
