//! Pattern: Adapter (a directory on disk as the store of the output): atomic per file (a temporary file, then a rename) and all-or-nothing in the staging step.

use super::write::{Found, Store};
use std::path::{Path, PathBuf};

/// A directory on disk; repo paths with `/` are put below it.
pub struct DirStore {
    root: PathBuf,
}

/// `<dir>/.<name>.fs-blueprint-tmp`: next to its place, so the rename stays on one volume.
fn temp_of(file: &Path) -> PathBuf {
    let name = file
        .file_name()
        .map(|n| n.to_string_lossy().into_owned())
        .unwrap_or_default();
    file.with_file_name(format!(".{name}.fs-blueprint-tmp"))
}

/// Removes temporary files; one that is already gone is no problem.
fn remove_all<'a>(files: impl Iterator<Item = &'a PathBuf>) {
    for f in files {
        drop(std::fs::remove_file(f));
    }
}

impl DirStore {
    #[must_use]
    pub fn new(root: impl Into<PathBuf>) -> Self {
        Self { root: root.into() }
    }

    fn native(&self, path: &str) -> PathBuf {
        path.split('/').fold(self.root.clone(), |p, c| p.join(c))
    }

    /// Writes the temporary file of one path.
    fn stage(&self, path: &str, content: &str) -> Result<(PathBuf, PathBuf), String> {
        if !self.path_ok(path) {
            return Err("link in path".to_string());
        }
        let file = self.native(path);
        if let Some(dir) = file.parent() {
            std::fs::create_dir_all(dir).map_err(|e| e.to_string())?;
        }
        let temp = temp_of(&file);
        std::fs::write(&temp, content).map_err(|e| e.to_string())?;
        Ok((temp, file))
    }
}

impl Store for DirStore {
    fn read(&self, path: &str) -> Found {
        match std::fs::read(self.native(path)) {
            Ok(bytes) => String::from_utf8(bytes).map_or(Found::Unreadable, Found::Text),
            // a file where a folder should be (NotADirectory on Unix, NotFound on Windows) is "not there" for this path; the staging step then fails on it
            Err(e)
                if matches!(
                    e.kind(),
                    std::io::ErrorKind::NotFound | std::io::ErrorKind::NotADirectory
                ) =>
            {
                Found::Missing
            }
            Err(_) => Found::Unreadable,
        }
    }

    /// No part of the path below the root may be a symbolic link or a junction (not the file, not a folder on the way).
    fn path_ok(&self, path: &str) -> bool {
        let mut at = self.root.clone();
        path.split('/').all(|c| {
            at.push(c);
            std::fs::symlink_metadata(&at).map_or(true, |m| !m.file_type().is_symlink())
        })
    }

    fn put(&mut self, path: &str, content: &str) -> Result<(), String> {
        self.put_all(&[(path, content)])
            .map_err(|_| format!("cannot write {path}"))
    }

    /// Step 1: every file is written to a temporary file next to its place; one failure removes them all and nothing new is visible.
    /// Step 2: the temporary files are moved over their places (a rename replaces an existing file also on Windows).
    fn put_all(&mut self, files: &[(&str, &str)]) -> Result<(), String> {
        let mut staged: Vec<(PathBuf, PathBuf, &str)> = Vec::new();
        for (path, content) in files {
            if let Ok((temp, file)) = self.stage(path, content) {
                staged.push((temp, file, path));
            } else {
                remove_all(staged.iter().map(|(t, _, _)| t));
                return Err((*path).to_string());
            }
        }
        for (i, (temp, file, path)) in staged.iter().enumerate() {
            if std::fs::rename(temp, file).is_err() {
                staged[i..]
                    .iter()
                    .for_each(|(t, _, _)| drop(std::fs::remove_file(t)));
                return Err((*path).to_string());
            }
        }
        Ok(())
    }
}
