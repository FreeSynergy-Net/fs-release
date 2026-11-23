//! Pattern: none (test). Chain gate `check-pin-chain`: an action that runs `$GITHUB_ACTION_PATH/../../scripts/...` takes the `scripts/` of ITS pin,
//! so those `scripts/` are compared with the start head too (REVIEW release-0.0.18 M2: a stale `lock-sources.toml` behind `actions/gates@X` stayed green).

use std::fs;
use std::path::Path;
use std::process::Command;

use fs_release::pin_chain::check;

fn git(d: &Path, args: &[&str]) -> String {
    let o = Command::new("git")
        .arg("-C")
        .arg(d)
        .args([
            "-c",
            "commit.gpgsign=false",
            "-c",
            "user.name=t",
            "-c",
            "user.email=t@t.t",
        ])
        .args(args)
        .output()
        .unwrap();
    assert!(o.status.success(), "git {args:?}");
    String::from_utf8_lossy(&o.stdout).trim().to_string()
}

fn write(d: &Path, rel: &str, text: &str) {
    let p = d.join(rel);
    fs::create_dir_all(p.parent().unwrap()).unwrap();
    fs::write(p, text).unwrap();
}

/// Old commit: action (`action_run` is its `run:`) plus `scripts/gates.sh` v1 and `scripts/lock-sources.toml` v1. Start commit: the same action, a
/// workflow pinning it to the old commit, and `change` applied to the repository. Returns the red findings.
fn findings(action_run: &str, change: impl Fn(&Path)) -> Vec<String> {
    let t = tempfile::tempdir().unwrap();
    let d = t.path();
    git(d, &["init", "-q", "-b", "main"]);
    write(
        d,
        "actions/gates/action.yml",
        &format!("runs:\n  steps:\n    - run: {action_run}\n"),
    );
    write(d, "scripts/gates.sh", "gates v1");
    write(d, "scripts/lock-sources.toml", "owner = old");
    git(d, &["add", "-A"]);
    git(d, &["commit", "-q", "-m", "old"]);
    let old = git(d, &["rev-parse", "HEAD"]);
    write(
        d,
        ".github/workflows/x.yml",
        &format!("jobs:\n  a:\n    steps:\n      - uses: FreeSynergy-Net/fs-release/actions/gates@{old}\n"),
    );
    change(d);
    git(d, &["add", "-A"]);
    git(d, &["commit", "-q", "-m", "start"]);
    check(d, "HEAD").unwrap()
}

const RUNS_SCRIPTS: &str = "'\"$GITHUB_ACTION_PATH/../../scripts/gates.sh\" \"$PWD\"'";

fn scripts_findings(found: &[String]) -> Vec<&String> {
    found
        .iter()
        .filter(|f| f.contains("runs the scripts/"))
        .collect()
}

#[test]
fn changed_scripts_behind_an_action_pin_are_red_with_the_file_and_the_fix() {
    let found = findings(RUNS_SCRIPTS, |d| {
        write(d, "scripts/lock-sources.toml", "owner = new");
    });
    let s = scripts_findings(&found);
    assert_eq!(s.len(), 1, "{found:?}");
    assert!(s[0].contains("scripts/lock-sources.toml"), "{s:?}");
    assert!(
        !s[0].contains("scripts/gates.sh,") && !s[0].contains("gates.sh (fix"),
        "only the changed file: {s:?}"
    );
    assert!(s[0].contains("pin-refs.sh"), "{s:?}");
}

#[test]
fn equal_scripts_behind_an_action_pin_are_green() {
    let found = findings(RUNS_SCRIPTS, |_| {});
    assert!(found.is_empty(), "{found:?}");
}

#[test]
fn a_script_that_exists_only_at_the_start_is_red() {
    let found = findings(RUNS_SCRIPTS, |d| write(d, "scripts/new.sh", "new"));
    let s = scripts_findings(&found);
    assert!(s.iter().any(|f| f.contains("scripts/new.sh")), "{found:?}");
}

#[test]
fn a_script_that_exists_only_at_the_pin_is_red() {
    let found = findings(RUNS_SCRIPTS, |d| {
        fs::remove_file(d.join("scripts/gates.sh")).unwrap();
    });
    let s = scripts_findings(&found);
    assert!(
        s.iter().any(|f| f.contains("scripts/gates.sh")),
        "{found:?}"
    );
}

/// An action that never reaches `../../scripts` is not held to the scripts (no over-reach).
#[test]
fn changed_scripts_behind_an_action_that_does_not_use_them_stay_green() {
    let found = findings("echo hello", |d| {
        write(d, "scripts/lock-sources.toml", "owner = new");
    });
    assert!(scripts_findings(&found).is_empty(), "{found:?}");
}

/// The self-pins inside `scripts/` are the one allowed difference (as everywhere in the gate).
#[test]
fn a_changed_self_pin_inside_scripts_is_not_a_difference() {
    let a = "0123456789012345678901234567890123456789";
    let b = "abcdefabcdefabcdefabcdefabcdefabcdefabcd";
    let t = tempfile::tempdir().unwrap();
    let d = t.path();
    git(d, &["init", "-q", "-b", "main"]);
    write(
        d,
        "actions/gates/action.yml",
        &format!("runs:\n  steps:\n    - run: {RUNS_SCRIPTS}\n"),
    );
    write(
        d,
        "scripts/t.yml",
        &format!("- uses: FreeSynergy-Net/fs-release/actions/x@{a}\n"),
    );
    git(d, &["add", "-A"]);
    git(d, &["commit", "-q", "-m", "old"]);
    let old = git(d, &["rev-parse", "HEAD"]);
    write(
        d,
        "scripts/t.yml",
        &format!("- uses: FreeSynergy-Net/fs-release/actions/x@{b}\n"),
    );
    write(
        d,
        ".github/workflows/x.yml",
        &format!("jobs:\n  a:\n    steps:\n      - uses: FreeSynergy-Net/fs-release/actions/gates@{old}\n"),
    );
    git(d, &["add", "-A"]);
    git(d, &["commit", "-q", "-m", "start"]);
    let found = check(d, "HEAD").unwrap();
    assert!(scripts_findings(&found).is_empty(), "{found:?}");
}
