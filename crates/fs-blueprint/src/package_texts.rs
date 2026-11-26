//! Pattern: Builder (the optional `texts/` of a module folder, checked and packed with the library of `fs-release pack-texts` into `catalog/texts/` of the module package; a link anywhere below `texts/` is refused).

use crate::package_module::{check_plain_dir, dir};
use crate::rpm_spec::SpecFile;
use std::path::Path;

/// The languages a module that ships `texts/` must carry (ENTSCHEID person Entscheid (b): en and de until the language packages exist).
const TEXT_LANGUAGES: [&str; 2] = ["en", "de"];

/// Optional `texts/<lang>.mf2` of the module: checked and packed with the library of `fs-release pack-texts` into `catalog/texts/` (`manifest.mf2m`, `<tag>.<hash>.mf2b`).
/// The `dates/` of the packer are language packages and stay out. No `texts/` folder: nothing changes.
pub(crate) fn pack_texts(
    folder: &Path,
    base: &str,
    files: &mut Vec<SpecFile>,
) -> Result<(), String> {
    let src = folder.join("texts");
    if std::fs::symlink_metadata(&src).is_err() {
        return Ok(());
    }
    check_plain_dir(&src)?;
    no_links(&src)?;
    let sources = fs_texts_pack::read_dir(&src).map_err(|e| format!("{}: {e}", src.display()))?;
    for l in TEXT_LANGUAGES {
        if !sources.iter().any(|s| s.language == l) {
            return Err(format!(
                "{}: {l}.mf2 is missing (en and de are both required)",
                src.display()
            ));
        }
    }
    let packed =
        fs_texts_pack::pack(&sources, &[]).map_err(|e| format!("{}: {e}", src.display()))?;
    let own = format!("{base}/catalog/texts");
    if files
        .iter()
        .any(|f| f.dest == own || f.dest.starts_with(&format!("{own}/")))
    {
        return Err("catalog/texts exists next to texts/: the packed texts would collide".into());
    }
    files.push(dir(&format!("{base}/catalog/texts")));
    for (name, bytes) in packed
        .files
        .into_iter()
        .filter(|(n, _)| !n.starts_with("dates/"))
    {
        files.push(SpecFile {
            dest: format!("{base}/catalog/texts/{name}"),
            bytes: Some(bytes),
            mode: 0o644,
            config: false,
        });
    }
    Ok(())
}

/// Every entry below `dir`, at any depth, must be a plain file or folder: a link is refused (it could pull a file of the build machine into a signed package).
fn no_links(dir: &Path) -> Result<(), String> {
    let list = std::fs::read_dir(dir).map_err(|e| format!("{}: {e}", dir.display()))?;
    for e in list.flatten() {
        let meta = std::fs::symlink_metadata(e.path())
            .map_err(|x| format!("{}: {x}", e.path().display()))?;
        if meta.file_type().is_symlink() {
            return Err(format!("{}: a link is not packaged", e.path().display()));
        }
        if meta.is_dir() {
            no_links(&e.path())?;
        }
    }
    Ok(())
}
