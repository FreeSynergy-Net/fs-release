//! Pattern: none (test). Default path of the lift step `pins`: it hands its own binary over as `FS_RELEASE_BIN` unless one is named.

use fs_release::lift::bin_env;
use std::path::Path;

const EXE: &str = "/own/fs-release";

fn kv(k: &str, v: &str) -> (String, String) {
    (k.to_string(), v.to_string())
}

#[test]
fn nothing_named_sets_the_own_binary() {
    let got = bin_env(&[kv("OTHER", "x")], false, Some(Path::new(EXE)));
    assert_eq!(got, vec![kv("FS_RELEASE_BIN", EXE)]);
}

#[test]
fn named_in_opts_or_process_env_stays_untouched() {
    assert!(bin_env(&[kv("FS_RELEASE_BIN", "/x")], false, Some(Path::new(EXE))).is_empty());
    assert!(bin_env(&[], true, Some(Path::new(EXE))).is_empty());
}

#[test]
fn unknown_own_path_sets_nothing() {
    assert!(bin_env(&[], false, None).is_empty());
}
