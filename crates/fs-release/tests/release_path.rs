//! Pattern: none (test). `check-pin-chain`, release-path fix gate: every internal pin of the release path must have all fixes of
//! `scripts/release-path-fixes.txt` as ancestors (the case of 13b680a: a pin before the fix builds fs-blueprint without it).

use std::fs;
use std::path::Path;
use std::process::Command;

use fs_release::pin_chain::check;

const O: &str = "FreeSynergy-Net/fs-release";

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
    assert!(
        o.status.success(),
        "git {args:?}: {}",
        String::from_utf8_lossy(&o.stderr)
    );
    String::from_utf8_lossy(&o.stdout).trim().to_string()
}

fn write(d: &Path, rel: &str, text: &str) {
    let p = d.join(rel);
    fs::create_dir_all(p.parent().unwrap()).unwrap();
    fs::write(p, text).unwrap();
}

fn commit(d: &Path, msg: &str) -> String {
    git(d, &["add", "-A"]);
    git(d, &["commit", "-q", "-m", msg, "--allow-empty"]);
    git(d, &["rev-parse", "HEAD"])
}

/// before (a commit without the fix), fix (the fix), side (a commit next to the fix, no descendant of it).
struct Fx {
    dir: tempfile::TempDir,
    before: String,
    fix: String,
    side: String,
}

fn fixture() -> Fx {
    let dir = tempfile::tempdir().unwrap();
    let d = dir.path();
    git(d, &["init", "-q", "-b", "main"]);
    write(d, "tool.txt", "old");
    let before = commit(d, "before");
    git(d, &["checkout", "-q", "-b", "side"]);
    write(d, "side.txt", "side");
    let side = commit(d, "side");
    git(d, &["checkout", "-q", "main"]);
    write(d, "tool.txt", "fixed");
    let fix = commit(d, "fix");
    Fx {
        dir,
        before,
        fix,
        side,
    }
}

/// The start head: release.yml and a `_release-linux.yml` pin `pin`; the list names `fixes`.
fn start(f: &Fx, pin: &str, fixes: &str) -> Vec<String> {
    let d = f.dir.path();
    let uses = |p: &str| format!("jobs:\n  a:\n    uses: {O}/.github/workflows/{p}@{pin}\n");
    write(
        d,
        ".github/workflows/release.yml",
        &uses("_release-linux.yml"),
    );
    write(d, ".github/workflows/_release-linux.yml", "on: x\n");
    write(d, "scripts/release-path-fixes.txt", fixes);
    let head = commit(d, "start");
    check(d, &head)
        .unwrap()
        .into_iter()
        .filter(|r| r.contains("release-path fix") || r.contains("fixes.txt"))
        .collect()
}

#[test]
fn a_pin_that_contains_the_fix_is_green() {
    let f = fixture();
    let list = format!("# fixes\n{} # requires\n", f.fix);
    assert_eq!(start(&f, &f.fix.clone(), &list), Vec::<String>::new());
}

#[test]
fn a_pin_before_the_fix_is_red() {
    let f = fixture();
    let found = start(&f, &f.before.clone(), &format!("{}\n", f.fix));
    assert_eq!(found.len(), 1, "{found:?}");
    assert!(
        found[0].contains("_release-linux.yml@") && found[0].contains(&f.fix[..8]),
        "{found:?}"
    );
}

#[test]
fn a_pin_that_is_no_descendant_of_the_fix_is_red() {
    let f = fixture();
    let found = start(&f, &f.side.clone(), &format!("{}\n", f.fix));
    assert_eq!(found.len(), 1, "{found:?}");
}

#[test]
fn every_fix_of_the_list_is_required() {
    let f = fixture();
    let list = format!("{}\n{}\n", f.before, f.side);
    let found = start(&f, &f.fix.clone(), &list);
    assert_eq!(found.len(), 1, "{found:?}");
    assert!(found[0].contains(&f.side[..8]), "{found:?}");
}

#[test]
fn a_list_entry_that_is_no_full_sha_is_red() {
    let f = fixture();
    let found = start(&f, &f.fix.clone(), "13b680a\n");
    assert!(
        found.iter().any(|r| r.contains("is not a full commit SHA")),
        "{found:?}"
    );
}

#[test]
fn without_a_list_nothing_is_required() {
    let f = fixture();
    let d = f.dir.path();
    write(
        d,
        ".github/workflows/release.yml",
        &format!(
            "jobs:\n  a:\n    uses: {O}/.github/workflows/_release-linux.yml@{}\n",
            f.before
        ),
    );
    write(d, ".github/workflows/_release-linux.yml", "on: x\n");
    let head = commit(d, "start");
    let found = check(d, &head).unwrap();
    assert!(
        !found.iter().any(|r| r.contains("release-path fix")),
        "{found:?}"
    );
}
