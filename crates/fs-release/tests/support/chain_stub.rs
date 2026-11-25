//! Pattern: Test Double (a stand-in `fs-release` for the chain gate of `scripts/pin-refs.sh`: the throw-away release dir has no real pin chain).

use std::fs;
use std::os::unix::fs::PermissionsExt;
use std::path::PathBuf;

use super::World;

/// A tiny executable under the world's root that ignores its arguments and exits with `rc` (idempotent).
pub fn stub(w: &World, name: &str, rc: i32) -> String {
    let p: PathBuf = w.root().join(name);
    fs::write(&p, format!("#!/bin/sh\nexit {rc}\n")).unwrap();
    fs::set_permissions(&p, fs::Permissions::from_mode(0o755)).unwrap();
    p.display().to_string()
}

/// `FS_RELEASE_BIN` for a gate that is green.
pub fn green(w: &World) -> (String, String) {
    ("FS_RELEASE_BIN".into(), stub(w, "chain-green", 0))
}

/// `FS_RELEASE_BIN` for a gate that is red.
pub fn red(w: &World) -> (String, String) {
    ("FS_RELEASE_BIN".into(), stub(w, "chain-red", 1))
}
