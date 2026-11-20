//! End to end: ONE blueprint becomes a data package through the real command `fs-blueprint new` (no Rust source, ADR 0086), on Linux and on Windows.
//! The test is `#[ignore]` in the normal run; the workflow `blueprint-e2e` runs it with `--ignored` on a Linux and a Windows runner.
//! Steps: `fs-blueprint new` (check, generate, write), `fs-blueprint check --drift` (no drift), the check caller of the package is there, no crate has sources.
use std::path::{Path, PathBuf};
use std::process::Command;

const NEW: &str = env!("CARGO_BIN_EXE_fs-blueprint");

fn fs_blueprint(args: &[&std::ffi::OsStr]) -> std::process::Output {
    Command::new(NEW)
        .args(args)
        .output()
        .expect("fs-blueprint starts")
}

#[test]
#[ignore = "builds the generated program (network, minutes); run by the workflow blueprint-e2e with --ignored"]
fn one_blueprint_becomes_a_program_that_builds_starts_and_answers() {
    let root =
        Path::new(env!("CARGO_TARGET_TMPDIR")).join(format!("e2e-probe-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&root);
    let blueprint = Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/data/probe/scaffold.toml");
    let os = std::ffi::OsStr::new;
    // the one command: check + generate + write (and the blueprint with its forms folder into the new repo)
    let made = fs_blueprint(&[
        os("new"),
        blueprint.as_os_str(),
        os("--out"),
        root.as_os_str(),
        os("--catalog"),
        probe_catalog().as_os_str(),
    ]);
    assert!(
        made.status.success(),
        "new failed:\n{}\n{}",
        String::from_utf8_lossy(&made.stdout),
        String::from_utf8_lossy(&made.stderr)
    );
    assert!(String::from_utf8_lossy(&made.stdout).contains("made probe"));
    assert!(
        root.join("scaffold.toml").is_file() && root.join("forms").join("simple.toml").is_file()
    );

    // what was written is the output of the blueprint: no drift
    let drift = fs_blueprint(&[
        os("check"),
        os("--repo"),
        root.as_os_str(),
        os("--catalog"),
        probe_catalog().as_os_str(),
        os("--drift"),
    ]);
    assert!(
        drift.status.success(),
        "drift:\n{}",
        String::from_utf8_lossy(&drift.stdout)
    );

    // a data package: no Rust source (nothing to build); the check workflow of the package is generated
    assert!(root
        .join(".github")
        .join("workflows")
        .join("check.yml")
        .is_file());
    assert!(!root
        .join("crates")
        .join("fs-probe-bin")
        .join("src")
        .exists());
}

/// The catalog of the probe: the shipped one plus the capability `probing` the probe blueprint declares.
fn probe_catalog() -> PathBuf {
    let path = Path::new(env!("CARGO_TARGET_TMPDIR")).join("probe-catalog.toml");
    let shipped = fs_blueprint::catalog_file::BUILTIN.replacen(
        "capabilities = [",
        "capabilities = [\"probing\", ",
        1,
    );
    std::fs::write(&path, shipped).unwrap();
    path
}
