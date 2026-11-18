//! Pattern: Adapter (the repo as a directory tree on disk: what the check asks of it; nothing is followed through a link, nothing is changed).

use crate::emit::header::class_of;
use crate::emit::Class;
use crate::form::is_repo_path;
use crate::kit::RepoView;
use std::path::{Path, PathBuf};

/// The view of a repo on disk. `generated` are the paths the generator makes for its blueprint; `talks` is a checkout of the documentation repo (the talk records).
pub struct FsRepo {
    root: PathBuf,
    generated: Vec<String>,
    talks: Option<PathBuf>,
}

/// Folders that are not part of the repo's own files.
const SKIP: &[&str] = &[".git", "target", "node_modules"];

impl FsRepo {
    #[must_use]
    pub fn new(root: impl Into<PathBuf>, generated: Vec<String>, talks: Option<PathBuf>) -> Self {
        Self {
            root: root.into(),
            generated,
            talks,
        }
    }

    fn native(&self, path: &str) -> PathBuf {
        path.trim_end_matches('/')
            .split('/')
            .fold(self.root.clone(), |p, c| p.join(c))
    }

    /// Every file below `dir` (repo paths with `/`, sorted), without links, the skipped folders and the folders named in `skip_dirs`.
    fn walk(&self, dir: &str, skip_dirs: &[&str]) -> Vec<String> {
        let mut out = Vec::new();
        let mut todo = vec![dir.to_string()];
        while let Some(d) = todo.pop() {
            let at = if d.is_empty() {
                self.root.clone()
            } else {
                self.native(&d)
            };
            let Ok(entries) = std::fs::read_dir(&at) else {
                continue;
            };
            for e in entries.flatten() {
                let name = e.file_name().to_string_lossy().into_owned();
                let rel = if d.is_empty() {
                    name.clone()
                } else {
                    format!("{d}/{name}")
                };
                let Ok(meta) = std::fs::symlink_metadata(e.path()) else {
                    continue;
                };
                if meta.file_type().is_symlink() {
                    continue;
                }
                if meta.is_dir() {
                    if !SKIP.contains(&name.as_str()) && !skip_dirs.contains(&name.as_str()) {
                        todo.push(rel);
                    }
                } else {
                    out.push(rel);
                }
            }
        }
        out.sort();
        out
    }
}

impl RepoView for FsRepo {
    fn exists(&self, path: &str) -> bool {
        is_repo_path(path)
            && std::fs::symlink_metadata(self.native(path)).is_ok_and(|m| {
                if path.ends_with('/') {
                    m.is_dir()
                } else {
                    true
                }
            })
    }

    fn covers_generated(&self, path: &str) -> bool {
        self.generated
            .iter()
            .any(|g| g == path || (path.ends_with('/') && g.starts_with(path)))
    }

    /// Every file under `crates/` (not in a `tests` folder) that has no head line of the generator.
    fn unowned_files(&self) -> Vec<String> {
        self.walk("crates", &["tests"])
            .into_iter()
            .filter(|f| {
                !std::fs::read_to_string(self.native(f))
                    .is_ok_and(|c| matches!(class_of(&c), Some(Class::Generated | Class::Seed)))
            })
            .collect()
    }

    fn main_files(&self) -> Vec<String> {
        self.walk("", &[])
            .into_iter()
            .filter(|f| f == "main.rs" || f.ends_with("/main.rs"))
            .collect()
    }

    fn read(&self, path: &str) -> Option<String> {
        if !is_repo_path(path) {
            return None;
        }
        std::fs::read_to_string(self.native(path)).ok()
    }

    fn talk_exists(&self, path: &str) -> bool {
        let Some(root) = &self.talks else {
            return false;
        };
        is_repo_path(path) && !path.ends_with('/') && Path::new(root).join(path).is_file()
    }
}
