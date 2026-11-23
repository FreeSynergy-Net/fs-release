//! Pattern: none (test). Chain gate `check-pin-chain`, whole tree behind a pin: which pins are held to it (outside `actions/`) and that it is reported once per pin.

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

fn tree_findings(found: &[String]) -> Vec<&String> {
    found
        .iter()
        .filter(|f| f.contains("whole repository"))
        .collect()
}

/// A pin to a directory outside `actions/` (or to the repository root) is held to the whole tree too (no `actions/` prefix needed).
#[test]
fn an_action_outside_actions_is_held_to_the_tree() {
    let t = tempfile::tempdir().unwrap();
    let d = t.path();
    git(d, &["init", "-q", "-b", "main"]);
    write(
        d,
        "tools/gates/action.yml",
        "runs:\n  steps:\n    - run: echo hi\n",
    );
    write(d, "RUST_VERSION", "1.90.0");
    git(d, &["add", "-A"]);
    git(d, &["commit", "-q", "-m", "old"]);
    let old = git(d, &["rev-parse", "HEAD"]);
    write(d, "RUST_VERSION", "1.91.0");
    write(
        d,
        ".github/workflows/x.yml",
        &format!(
            "jobs:\n  a:\n    steps:\n      - uses: FreeSynergy-Net/fs-release/tools/gates@{old}\n"
        ),
    );
    git(d, &["add", "-A"]);
    git(d, &["commit", "-q", "-m", "start"]);
    let found = check(d, "HEAD").unwrap();
    assert!(
        tree_findings(&found)
            .iter()
            .any(|f| f.contains("RUST_VERSION")),
        "{found:?}"
    );
}

/// Two actions at the same pin give one finding, not two (the message says it holds for every action at that pin).
#[test]
fn two_actions_at_one_pin_give_one_finding() {
    let t = tempfile::tempdir().unwrap();
    let d = t.path();
    git(d, &["init", "-q", "-b", "main"]);
    write(
        d,
        "actions/a/action.yml",
        "runs:\n  steps:\n    - run: echo a\n",
    );
    write(
        d,
        "actions/b/action.yml",
        "runs:\n  steps:\n    - run: echo b\n",
    );
    write(d, "RUST_VERSION", "1.90.0");
    git(d, &["add", "-A"]);
    git(d, &["commit", "-q", "-m", "old"]);
    let old = git(d, &["rev-parse", "HEAD"]);
    write(d, "RUST_VERSION", "1.91.0");
    let u = |n: &str| format!("      - uses: FreeSynergy-Net/fs-release/actions/{n}@{old}\n");
    write(
        d,
        ".github/workflows/x.yml",
        &format!("jobs:\n  a:\n    steps:\n{}{}", u("a"), u("b")),
    );
    git(d, &["add", "-A"]);
    git(d, &["commit", "-q", "-m", "start"]);
    let found = check(d, "HEAD").unwrap();
    assert_eq!(tree_findings(&found).len(), 1, "{found:?}");
}

/// Two binary files with different bytes whose lossy text is equal are still different (no lossy compare).
#[test]
fn binary_files_that_only_look_equal_as_lossy_text_are_red() {
    let t = tempfile::tempdir().unwrap();
    let d = t.path();
    git(d, &["init", "-q", "-b", "main"]);
    write(
        d,
        "actions/a/action.yml",
        "runs:\n  steps:\n    - run: echo a\n",
    );
    fs::write(d.join("blob.bin"), [0xff_u8, 0x01]).unwrap();
    git(d, &["add", "-A"]);
    git(d, &["commit", "-q", "-m", "old"]);
    let old = git(d, &["rev-parse", "HEAD"]);
    fs::write(d.join("blob.bin"), [0xfe_u8, 0x01]).unwrap();
    write(
        d,
        ".github/workflows/x.yml",
        &format!(
            "jobs:\n  a:\n    steps:\n      - uses: FreeSynergy-Net/fs-release/actions/a@{old}\n"
        ),
    );
    git(d, &["add", "-A"]);
    git(d, &["commit", "-q", "-m", "start"]);
    let found = check(d, "HEAD").unwrap();
    assert!(
        tree_findings(&found).iter().any(|f| f.contains("blob.bin")),
        "{found:?}"
    );
}
