//! Helpers shared by the `package_*` tests: data folders, scratch dirs, the CLI, and the system `rpm`.
#![allow(dead_code)]
use fs_blueprint::cli::run;
use std::path::{Path, PathBuf};
use std::process::Command;

pub fn data(name: &str) -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("tests/data/repos")
        .join(name)
}

pub fn scratch(name: &str) -> PathBuf {
    let d =
        Path::new(env!("CARGO_TARGET_TMPDIR")).join(format!("pkg-{name}-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&d);
    d
}

pub fn cli(words: &[&str]) -> (i32, String, String) {
    let words: Vec<String> = words.iter().map(|w| (*w).to_string()).collect();
    let (mut out, mut err) = (Vec::new(), Vec::new());
    let code = run(&words, &mut out, &mut err);
    (
        code,
        String::from_utf8(out).unwrap(),
        String::from_utf8(err).unwrap(),
    )
}

/// `rpm` must be there: a missing tool is a RED test, never a silent pass (review S5). The one way out is the explicit `FS_SKIP_RPM_TESTS=1`, and never when `CI` is set; it is said loudly.
pub fn have_rpm() -> bool {
    if Command::new("rpm").arg("--version").output().is_ok() {
        return true;
    }
    let skip = std::env::var("FS_SKIP_RPM_TESTS").is_ok_and(|v| v == "1");
    assert!(
        skip && std::env::var_os("CI").is_none(),
        "rpm is not installed: install it (CI installs it) or set FS_SKIP_RPM_TESTS=1 outside CI"
    );
    eprintln!("SKIPPED: rpm is not installed (FS_SKIP_RPM_TESTS=1)");
    false
}

pub const FP: &str = "16A4A3EE563AEF51FCF0C55DED7B57C4A1904342";

pub fn rpm(args: &[&str], file: &Path) -> String {
    let o = Command::new("rpm").args(args).arg(file).output().unwrap();
    assert!(
        o.status.success(),
        "rpm {args:?}: {}",
        String::from_utf8_lossy(&o.stderr)
    );
    String::from_utf8(o.stdout).unwrap()
}

pub fn make_demo(out: &Path, more: &[&str]) -> PathBuf {
    let bp = data("ok").join("scaffold.toml");
    let mut w = vec![
        "package",
        "rpm",
        bp.to_str().unwrap(),
        "--out",
        out.to_str().unwrap(),
    ];
    w.extend_from_slice(more);
    let (code, o, e) = cli(&w);
    assert_eq!(code, 0, "{o}{e}");
    out.join("fs-demo-0.0.1-1.noarch.rpm")
}
