//! Test support: fixture for `fs-release address aufrufer` (fs-release clone with tags, caller repos with one workflow file).

use std::fmt::Write as _;
use std::fs;
use std::path::Path;
use std::process::Command;

use fs_release::cli::main_with;

pub const NEW: &str = "FreeSynergy-Net";

pub fn git(dir: &Path, args: &[&str]) -> String {
    let o = Command::new("git")
        .arg("-C")
        .arg(dir)
        .args([
            "-c",
            "user.name=t",
            "-c",
            "user.email=t@t",
            "-c",
            "commit.gpgsign=false",
        ])
        .args(args)
        .output()
        .unwrap();
    assert!(o.status.success(), "git {args:?}");
    String::from_utf8_lossy(&o.stdout).trim().to_string()
}

pub fn commit(dir: &Path, msg: &str) -> String {
    fs::write(dir.join("f.txt"), msg).unwrap();
    git(dir, &["add", "-A"]);
    git(dir, &["commit", "-q", "-m", msg]);
    git(dir, &["rev-parse", "HEAD"])
}

/// fs-release clone: old (tag v0.0.17) -> neutral -> l (main) -> later (tag v0.0.19); `side` branches off l, not on main.
pub struct Fix {
    pub dir: tempfile::TempDir,
    pub old: String,
    pub neutral: String,
    pub l: String,
    pub side: String,
}

pub fn fixture() -> Fix {
    let t = tempfile::tempdir().unwrap();
    let d = t.path();
    git(d, &["init", "-q", "-b", "main"]);
    let old = commit(d, "old");
    git(d, &["tag", "v0.0.17"]);
    let neutral = commit(d, "neutral");
    let l = commit(d, "l");
    git(d, &["checkout", "-q", "-b", "side"]);
    let side = commit(d, "side");
    git(d, &["checkout", "-q", "main"]);
    commit(d, "later");
    git(d, &["tag", "v0.0.19"]);
    git(d, &["update-ref", "refs/remotes/origin/main", "main"]);
    // HEAD of the clone is NOT main: a mutant that judges against HEAD instead of origin/main turns red here.
    git(d, &["checkout", "-q", "side"]);
    Fix {
        dir: t,
        old,
        neutral,
        l,
        side,
    }
}

/// Caller repo with one workflow file holding the given `uses:` lines.
/// `      - uses: <u>` per line.
fn uses_body(uses: &[String]) -> String {
    uses.iter().fold(String::new(), |mut s, u| {
        let _ = writeln!(s, "      - uses: {u}");
        s
    })
}

/// Caller repo with one workflow file holding the given `uses:` lines.
pub fn caller(uses: &[String]) -> tempfile::TempDir {
    let t = tempfile::tempdir().unwrap();
    git(t.path(), &["init", "-q", "-b", "main"]);
    fs::create_dir_all(t.path().join(".github/workflows")).unwrap();
    fs::write(t.path().join(".github/workflows/ci.yml"), uses_body(uses)).unwrap();
    git(t.path(), &["add", "-A"]);
    git(t.path(), &["commit", "-q", "-m", "x"]);
    t
}

/// Caller repo whose only workflow has the given file name (for example a non-ASCII one).
pub fn caller_named(name: &str, uses: &[String]) -> tempfile::TempDir {
    let t = caller(&[]);
    fs::remove_file(t.path().join(".github/workflows/ci.yml")).unwrap();
    fs::write(
        t.path().join(".github/workflows").join(name),
        uses_body(uses),
    )
    .unwrap();
    git(t.path(), &["add", "-A"]);
    git(t.path(), &["commit", "-q", "-m", "y"]);
    t
}

pub fn u(owner: &str, r: &str) -> String {
    format!("{owner}/fs-release/.github/workflows/check.yml@{r}")
}

pub fn cli(f: &Fix, uses: &[String], neutral: &str) -> i32 {
    let c = caller(uses);
    cli_with(f, c.path(), neutral)
}

pub fn cli_dir(f: &Fix, dir: &Path) -> i32 {
    cli_with(f, dir, &f.neutral)
}

fn cli_with(f: &Fix, dir: &Path, neutral: &str) -> i32 {
    main_with(
        [
            "fs-release",
            "address",
            "aufrufer",
            "--neutral",
            neutral,
            "--fs-release",
            &f.dir.path().display().to_string(),
            &dir.display().to_string(),
        ]
        .map(String::from)
        .to_vec(),
    )
}
