//! End to end: ONE blueprint becomes an empty program that builds, starts and answers, on Linux and on Windows.
//! The test builds the generated workspace (it needs the network for the git pins and takes minutes), so it is `#[ignore]` in the normal run; the workflow
//! `blueprint-e2e` runs it with `--ignored` on a Linux and a Windows runner. Steps: generate, write, check (no drift), `cargo test` of the generated
//! workspace (its own start test: version, help, wrong word, `serve` + handshake + stop), then the binary itself: `--version`, `--help`, `serve` + handshake.
mod support;
use fs_blueprint::emit::check::check;
use fs_blueprint::emit::generate;
use fs_blueprint::emit::write::{write, DirStore};
use fs_blueprint::Blueprint;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use support::{FakeCatalog, FakeView};

const PROBE: &str = include_str!("data/probe.toml");

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

#[test]
#[ignore = "builds the generated program (network, minutes); run by the workflow blueprint-e2e with --ignored"]
fn one_blueprint_becomes_a_program_that_builds_starts_and_answers() {
    let root =
        Path::new(env!("CARGO_TARGET_TMPDIR")).join(format!("e2e-probe-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&root);
    std::fs::create_dir_all(&root).unwrap();
    let bp = Blueprint::from_toml(PROBE).unwrap();
    let arts = generate(&bp, &FakeCatalog).unwrap();
    let mut store = DirStore::new(&root);
    let out = write(&arts, &mut store).unwrap();
    assert!(out.written.len() >= 10, "{out:?}");
    // the blueprint itself is part of the program repo: the generated entry point includes it
    std::fs::write(root.join("scaffold.toml"), PROBE).unwrap();

    // what was written is the output of the blueprint: no drift
    let mut view = FakeView::default();
    for a in &arts {
        view.contents.insert(
            a.path.clone(),
            std::fs::read_to_string(a.path.split('/').fold(root.clone(), |p, c| p.join(c)))
                .unwrap(),
        );
    }
    assert_eq!(check(&bp, &FakeCatalog, &view).errors, []);

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

    // serve: the help answers and the server stays up until it is ended (the handshake itself is the generated start test above)
    let status = Command::new(exe(&target))
        .args(["serve", "--help"])
        .stdout(Stdio::null())
        .status()
        .unwrap();
    assert!(status.success());
}
