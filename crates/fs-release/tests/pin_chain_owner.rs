//! Pattern: none (test). Chain gate `check-pin-chain`: an `fs-release` owner that is spelled differently or is foreign is red
//! (GitHub resolves `freesynergy-net/...` too, the chain follows only the exact spellings: before, such a pin stayed old and green).

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

/// A repo whose start head has one correct self-pin (`FreeSynergy-Net`, to an old commit with the SAME action code) and a second `uses:` with `owner`
/// pinned to an OLD action code. Returns the red findings.
fn findings(owner: &str) -> Vec<String> {
    let t = tempfile::tempdir().unwrap();
    let d = t.path();
    git(d, &["init", "-q", "-b", "main"]);
    write(d, "actions/pub/pub.sh", "publish v1");
    git(d, &["add", "-A"]);
    git(d, &["commit", "-q", "-m", "old"]);
    let old = git(d, &["rev-parse", "HEAD"]);
    write(d, "actions/pub/pub.sh", "publish v2");
    let wf = format!(
        "jobs:\n  a:\n    steps:\n      - uses: FreeSynergy-Net/fs-release/actions/pub@{old}\n      - uses: {owner}/fs-release/actions/pub@{old}\n"
    );
    write(d, ".github/workflows/x.yml", &wf);
    git(d, &["add", "-A"]);
    git(d, &["commit", "-q", "-m", "start"]);
    check(d, "HEAD").unwrap()
}

#[test]
fn a_lowercase_owner_pinned_to_old_code_is_red() {
    let found = findings("freesynergy-net");
    assert!(
        found.iter().any(|f| f.contains("unknown owner spelling")),
        "{found:?}"
    );
}

#[test]
fn a_foreign_owner_is_red() {
    let found = findings("Evil");
    assert!(
        found.iter().any(|f| f.contains("unknown owner spelling")),
        "{found:?}"
    );
}

#[test]
fn the_exact_owners_are_not_flagged_as_spelling() {
    for o in ["FreeSynergy-Net", "FreeSynergy"] {
        let found = findings(o);
        assert!(
            !found.iter().any(|f| f.contains("unknown owner spelling")),
            "{found:?}"
        );
        assert!(
            found
                .iter()
                .any(|f| f.contains("differs from the start head")),
            "{found:?}"
        );
    }
}
