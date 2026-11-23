//! Test support: the fs-release clone the pin script checks pins against (`FS_RELEASE_REPO`, default: the script's repo = the world's `rel`).

use super::{git, World};
use std::fs;

/// Makes `<rel>` a git repo with a linear history, one commit and one tag per entry (oldest first).
pub fn tag_release(w: &World, tags: &[&str]) {
    let rel = w.rel();
    git(&rel, &["init", "--quiet"]);
    for t in tags {
        fs::write(rel.join("tag.txt"), t).unwrap();
        git(&rel, &["add", "-A"]);
        git(&rel, &["commit", "--quiet", "-m", t]);
        git(&rel, &["tag", t]);
    }
}
