//! Pattern: none (test). Chain gate `check-pin-chain`: behind an action pin the WHOLE tree of the pin is compared with the start head (REVIEW
//! pin-chain-gate r4 M1; before: only `scripts/` and only the text `../../scripts`). One fixture per way an action reaches the tree.

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

/// Old commit: action (`action_run` is its `run:`), `scripts/`, `RUST_VERSION`, `configs/deny.toml`, `release-signing-key.asc`, a neighbour action.
/// Start commit: the same plus a workflow pinning `actions/gates` to the old commit, and `change` applied. Returns the findings.
fn findings(action_run: &str, change: impl Fn(&Path)) -> Vec<String> {
    let t = tempfile::tempdir().unwrap();
    let d = t.path();
    git(d, &["init", "-q", "-b", "main"]);
    write(
        d,
        "actions/gates/action.yml",
        &format!("runs:\n  steps:\n    - run: {action_run}\n"),
    );
    write(d, "actions/sib/install.sh", "install v1");
    write(d, "scripts/gates.sh", "gates v1");
    write(d, "scripts/lock-sources.toml", "owner = old");
    write(d, "RUST_VERSION", "1.90.0");
    write(d, "configs/deny.toml", "deny v1");
    write(d, "release-signing-key.asc", "key v1");
    write(
        d,
        ".github/workflows/x.yml",
        &format!(
            "jobs:\n  a:\n    steps:\n      - uses: FreeSynergy-Net/fs-release/actions/gates@{}\n",
            "0".repeat(40)
        ),
    );
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

fn tree_findings(found: &[String]) -> Vec<&String> {
    found
        .iter()
        .filter(|f| f.contains("whole repository"))
        .collect()
}

/// The spellings of the old text check (and others): none of them mentions `../../scripts` as that exact text, the gate must not care.
const FORMS: [(&str, &str); 4] = [
    ("exact", "'\"$GITHUB_ACTION_PATH/../../scripts/gates.sh\"'"),
    (
        "dot-slash",
        "'\"$GITHUB_ACTION_PATH/../.././scripts/gates.sh\"'",
    ),
    (
        "double-slash",
        "'\"$GITHUB_ACTION_PATH/..//../scripts/gates.sh\"'",
    ),
    (
        "cd",
        "'cd \"$GITHUB_ACTION_PATH/../..\" && scripts/gates.sh'",
    ),
];

#[test]
fn changed_scripts_are_red_in_every_spelling_with_the_file_and_the_fix() {
    for (name, run) in FORMS {
        let found = findings(run, |d| {
            write(d, "scripts/lock-sources.toml", "owner = new");
        });
        let s = tree_findings(&found);
        assert_eq!(s.len(), 1, "{name}: {found:?}");
        assert!(s[0].contains("scripts/lock-sources.toml"), "{name}: {s:?}");
        assert!(
            !s[0].contains("scripts/gates.sh,"),
            "{name}: only the changed file: {s:?}"
        );
        assert!(s[0].contains("pin-refs.sh"), "{name}: {s:?}");
    }
}

#[test]
fn equal_tree_behind_an_action_pin_is_green() {
    for (name, run) in FORMS {
        let found = findings(run, |_| {});
        assert!(found.is_empty(), "{name}: {found:?}");
    }
}

fn red_for(run: &str, file: &str, text: &str) {
    let found = findings(run, |d| write(d, file, text));
    let s = tree_findings(&found);
    assert!(s.iter().any(|f| f.contains(file)), "{file}: {found:?}");
}

#[test]
fn rust_version_read_through_dotdot_is_red() {
    red_for(
        "'cat \"$GITHUB_ACTION_PATH/../../RUST_VERSION\"'",
        "RUST_VERSION",
        "1.91.0",
    );
}

#[test]
fn deny_config_read_through_dotdot_is_red() {
    red_for(
        "'cargo deny --config \"$GITHUB_ACTION_PATH/../../configs/deny.toml\"'",
        "configs/deny.toml",
        "deny v2",
    );
}

#[test]
fn signing_key_read_through_dotdot_is_red() {
    red_for(
        "'gpg --import \"$GITHUB_ACTION_PATH/../../release-signing-key.asc\"'",
        "release-signing-key.asc",
        "key v2",
    );
}

#[test]
fn a_neighbour_action_is_red_when_it_changed() {
    red_for(
        "'\"$GITHUB_ACTION_PATH/../sib/install.sh\"'",
        "actions/sib/install.sh",
        "install v2",
    );
}

#[test]
fn a_script_that_exists_only_at_the_start_is_red() {
    red_for(FORMS[0].1, "scripts/new.sh", "new");
}

#[test]
fn a_script_that_exists_only_at_the_pin_is_red() {
    let found = findings(FORMS[0].1, |d| {
        fs::remove_file(d.join("scripts/gates.sh")).unwrap();
    });
    assert!(
        tree_findings(&found)
            .iter()
            .any(|f| f.contains("scripts/gates.sh")),
        "{found:?}"
    );
}

/// A binary file with other bytes is red even when its lossy text would be equal (fail closed).
#[test]
fn a_changed_binary_file_is_red() {
    let found = findings("echo hi", |d| {
        fs::write(d.join("release-signing-key.asc"), [0xff_u8, 0xfe, 0x01]).unwrap();
    });
    assert!(
        tree_findings(&found)
            .iter()
            .any(|f| f.contains("release-signing-key.asc")),
        "{found:?}"
    );
}

/// The red is reported once per pin, not once per action at that pin.
#[test]
fn one_finding_per_pin() {
    let found = findings(FORMS[0].1, |d| {
        write(d, "RUST_VERSION", "1.91.0");
        write(d, "configs/deny.toml", "deny v2");
    });
    assert_eq!(tree_findings(&found).len(), 1, "{found:?}");
}

/// The self-pins (in `scripts/` or a workflow) are the one allowed difference (as everywhere in the gate).
#[test]
fn a_changed_self_pin_anywhere_in_the_tree_is_not_a_difference() {
    let a = "0123456789012345678901234567890123456789";
    let b = "abcdefabcdefabcdefabcdefabcdefabcdefabcd";
    let t = tempfile::tempdir().unwrap();
    let d = t.path();
    git(d, &["init", "-q", "-b", "main"]);
    write(
        d,
        "actions/gates/action.yml",
        &format!("runs:\n  steps:\n    - run: {}\n", FORMS[0].1),
    );
    write(
        d,
        "scripts/t.yml",
        &format!("- uses: FreeSynergy-Net/fs-release/actions/x@{a}\n"),
    );
    write(
        d,
        ".github/workflows/y.yml",
        &format!("- uses: FreeSynergy-Net/fs-release/actions/x@{a}\n"),
    );
    write(
        d,
        ".github/workflows/x.yml",
        &format!(
            "jobs:\n  a:\n    steps:\n      - uses: FreeSynergy-Net/fs-release/actions/gates@{}\n",
            "0".repeat(40)
        ),
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
    assert!(tree_findings(&found).is_empty(), "{found:?}");
}
