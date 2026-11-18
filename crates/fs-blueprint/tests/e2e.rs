//! End to end: ONE blueprint becomes an empty program through the real command `fs-blueprint new`, and the program builds, starts and answers, on Linux and on Windows.
//! The test builds the generated workspace (it needs the network for the git pins and takes minutes), so it is `#[ignore]` in the normal run; the workflow
//! `blueprint-e2e` runs it with `--ignored` on a Linux and a Windows runner. Steps: `fs-blueprint new` (check, generate, write), `fs-blueprint check --drift` (no drift),
//! `cargo build` and `cargo test` of the generated workspace (its own start test: version, help, wrong word, `serve` + handshake + stop), then the binary itself.
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};

const NEW: &str = env!("CARGO_BIN_EXE_fs-blueprint");

fn cargo(dir: &Path, target: &Path, args: &[&str]) {
    let cargo = std::env::var("CARGO").unwrap_or_else(|_| "cargo".to_string());
    let out = Command::new(cargo)
        .args(args)
        .current_dir(dir)
        .env("CARGO_TARGET_DIR", target)
        .env("CARGO_TERM_COLOR", "never")
        .output()
        .expect("cargo starts");
    assert!(
        out.status.success(),
        "cargo {args:?} failed:\n{}\n{}",
        String::from_utf8_lossy(&out.stdout),
        String::from_utf8_lossy(&out.stderr)
    );
}

fn exe(target: &Path) -> PathBuf {
    target
        .join("debug")
        .join(format!("fs-probe{}", std::env::consts::EXE_SUFFIX))
}

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

    let target = root.join("target");
    cargo(&root, &target, &["build", "--workspace"]);
    cargo(&root, &target, &["test", "--workspace"]);

    let version = Command::new(exe(&target))
        .arg("--version")
        .output()
        .unwrap();
    assert!(version.status.success());
    assert_eq!(
        String::from_utf8_lossy(&version.stdout).trim(),
        "fs-probe 0.0.1"
    );
    let help = Command::new(exe(&target)).arg("--help").output().unwrap();
    assert!(help.status.success());
    assert!(String::from_utf8_lossy(&help.stdout).contains("fs-probe serve"));
    let status = Command::new(exe(&target))
        .args(["serve", "--help"])
        .stdout(Stdio::null())
        .status()
        .unwrap();
    assert!(status.success());
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
