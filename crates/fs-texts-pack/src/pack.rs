//! The packer: sources in, a folder of files out.
//!
//! Pattern: Builder (the check, the manifest, then per language catalog and date blob; the result is a plain map of path -> bytes, written in one go).
//! Layout of the output: `manifest.mf2m`, `<locale>.<hash>.mf2b` per language, `dates/<locale>/icu.blob` per language. The manifest is built from ALL
//! sources, so it is the same whichever languages are asked for (a language package never changes the manifest of another).

use std::collections::BTreeMap;
use std::path::Path;

use crate::catalog;
use crate::check;
use crate::dates;
use crate::error::PackError;
use crate::locale;
use crate::source::Source;

/// The packed files: relative path -> content, in a fixed order.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct Packed {
    /// Relative path (always with `/`) and bytes.
    pub files: BTreeMap<String, Vec<u8>>,
}

/// Checks `sources` (every language, every pair) and packs the languages in `wanted` (empty: all of them).
///
/// # Errors
/// `Check` with every problem of the sources (a missing pair is an error, never skipped), `NoSuchLanguage` for a wanted language without a source,
/// `Library` when `mf2` refuses the data.
pub fn pack(sources: &[Source], wanted: &[String]) -> Result<Packed, PackError> {
    let languages = check::languages(sources).map_err(PackError::Check)?;
    let mut chosen: Vec<String> = Vec::new();
    for w in wanted {
        let tag = locale::canonical(w).map_err(|why| PackError::Check(vec![why]))?;
        if !languages.iter().any(|l| l.tag == tag) {
            return Err(PackError::NoSuchLanguage(tag));
        }
        chosen.push(tag);
    }
    let manifest = catalog::manifest(&languages);
    let mut files = BTreeMap::new();
    files.insert("manifest.mf2m".to_owned(), manifest.write());
    for l in languages
        .iter()
        .filter(|l| chosen.is_empty() || chosen.contains(&l.tag))
    {
        let bytes = catalog::write(l, &manifest)?;
        files.insert(catalog::file_name(&l.tag, &bytes), bytes);
        files.insert(dates::path(&l.tag), dates::blob(&l.tag)?);
    }
    Ok(Packed { files })
}

fn is_catalog(name: &str) -> bool {
    Path::new(name).extension().is_some_and(|e| e == "mf2b")
}

impl Packed {
    /// Writes the files below `dir` (created). An older catalog of a packed language under another hash is removed first, so the folder holds
    /// exactly one catalog per language.
    ///
    /// # Errors
    /// `Io` when a folder or file cannot be written.
    pub fn write(&self, dir: &Path) -> Result<(), PackError> {
        let io = |what: &Path, e: std::io::Error| PackError::Io(format!("{}: {e}", what.display()));
        std::fs::create_dir_all(dir).map_err(|e| io(dir, e))?;
        for name in self.files.keys().filter(|n| is_catalog(n)) {
            let prefix = name.split('.').next().unwrap_or("");
            for old in std::fs::read_dir(dir).map_err(|e| io(dir, e))?.flatten() {
                let old = old.file_name().to_string_lossy().into_owned();
                if is_catalog(&old) && old.split('.').next() == Some(prefix) {
                    std::fs::remove_file(dir.join(&old)).map_err(|e| io(&dir.join(&old), e))?;
                }
            }
        }
        for (name, bytes) in &self.files {
            let path = dir.join(name);
            if let Some(parent) = path.parent() {
                std::fs::create_dir_all(parent).map_err(|e| io(parent, e))?;
            }
            std::fs::write(&path, bytes).map_err(|e| io(&path, e))?;
        }
        Ok(())
    }
}
