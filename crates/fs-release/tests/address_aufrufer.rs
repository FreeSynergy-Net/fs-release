//! Pattern: none (test). `fs-release address aufrufer`: green/red/error cases and the mutants (rule 4 off, rule 5 off,
//! case-insensitive owner) that the suite must catch.

use std::fs;
use std::path::Path;
use std::process::Command;

use fs_release::address::aufrufer::{Ctx, Report, Rules};
use fs_release::cli::main_with;

fn git(dir: &Path, args: &[&str]) -> String {
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

fn commit(dir: &Path, msg: &str) -> String {
    fs::write(dir.join("f.txt"), msg).unwrap();
    git(dir, &["add", "-A"]);
    git(dir, &["commit", "-q", "-m", msg]);
    git(dir, &["rev-parse", "HEAD"])
}

/// fs-release clone: old (tag v0.0.17) -> neutral -> l (main) -> later (tag v0.0.19); `side` branches off l, not on main.
struct Fix {
    dir: tempfile::TempDir,
    old: String,
    neutral: String,
    l: String,
    side: String,
}

fn fixture() -> Fix {
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
    Fix {
        dir: t,
        old,
        neutral,
        l,
        side,
    }
}

/// Caller repo with one workflow file holding the given `uses:` lines.
fn caller(uses: &[String]) -> tempfile::TempDir {
    let t = tempfile::tempdir().unwrap();
    git(t.path(), &["init", "-q", "-b", "main"]);
    fs::create_dir_all(t.path().join(".github/workflows")).unwrap();
    let body: String = uses
        .iter()
        .map(|u| format!("      - uses: {u}\n"))
        .collect();
    fs::write(t.path().join(".github/workflows/ci.yml"), body).unwrap();
    git(t.path(), &["add", "-A"]);
    git(t.path(), &["commit", "-q", "-m", "x"]);
    t
}

fn u(owner: &str, r: &str) -> String {
    format!("{owner}/fs-release/.github/workflows/check.yml@{r}")
}

fn red_count(f: &Fix, rules: Rules, uses: &[String]) -> usize {
    let c = caller(uses);
    let ctx = Ctx::new(f.dir.path(), &f.neutral, rules).unwrap();
    let mut rep = Report {
        lines: Vec::new(),
        quiet_repos: Vec::new(),
    };
    ctx.check_repo(c.path(), "HEAD", &mut rep).unwrap();
    rep.red()
}

fn cli(f: &Fix, uses: &[String], neutral: &str) -> i32 {
    let c = caller(uses);
    let a: Vec<String> = [
        "fs-release",
        "address",
        "aufrufer",
        "--neutral",
        neutral,
        "--fs-release",
    ]
    .iter()
    .map(ToString::to_string)
    .chain([
        f.dir.path().display().to_string(),
        c.path().display().to_string(),
    ])
    .collect();
    main_with(a)
}

const NEW: &str = "FreeSynergy-Net";

#[test]
fn aufrufer_green_new_at_l_and_new_at_tag_after_l() {
    let f = fixture();
    assert_eq!(cli(&f, &[u(NEW, &f.l)], &f.neutral), 0, "neu@L");
    assert_eq!(
        cli(&f, &[u(NEW, "v0.0.19")], &f.neutral),
        0,
        "neu@Tag nach L"
    );
    assert_eq!(
        cli(&f, &[u(NEW, &f.l), u(NEW, &f.l)], &f.neutral),
        0,
        "same ref twice"
    );
}

#[test]
fn aufrufer_red_cases() {
    let f = fixture();
    let cases: Vec<(&str, Vec<String>)> = vec![
        ("alt@v0.0.17", vec![u("FreeSynergy", "v0.0.17")]),
        ("neu@v0.0.17", vec![u(NEW, "v0.0.17")]),
        ("neu@main", vec![u(NEW, "main")]),
        ("neu@SHA vor neutral", vec![u(NEW, &f.old)]),
        ("neu@Seitenzweig-SHA", vec![u(NEW, &f.side)]),
        ("freesynergy-net@L", vec![u("freesynergy-net", &f.l)]),
        ("FreeSynergy-Netz@L", vec![u("FreeSynergy-Netz", &f.l)]),
        ("XFreeSynergy-Net@L", vec![u("XFreeSynergy-Net", &f.l)]),
        ("kurze SHA", vec![u(NEW, &f.l[..12])]),
        ("unbekannte SHA", vec![u(NEW, &"1".repeat(40))]),
        ("gemischt", vec![u(NEW, &f.l), u(NEW, "v0.0.19")]),
    ];
    for (name, uses) in cases {
        assert_eq!(cli(&f, &uses, &f.neutral), 1, "{name} must be red");
    }
}

#[test]
fn aufrufer_error_is_two_never_green() {
    let f = fixture();
    let ok = [u(NEW, &f.l)];
    assert_eq!(
        main_with(
            ["fs-release", "address", "aufrufer", "x"]
                .map(String::from)
                .to_vec()
        ),
        2,
        "neutral missing"
    );
    assert_eq!(cli(&f, &ok, "abc"), 2, "neutral not 40 hex");
    assert_eq!(cli(&f, &ok, &"2".repeat(40)), 2, "neutral not in clone");
    let c = caller(&ok);
    let args = |clone: &Path, repo: &Path| {
        [
            "fs-release",
            "address",
            "aufrufer",
            "--neutral",
            &f.neutral,
            "--fs-release",
            &clone.display().to_string(),
            &repo.display().to_string(),
        ]
        .map(String::from)
        .to_vec()
    };
    let plain = tempfile::tempdir().unwrap();
    assert_eq!(
        main_with(args(plain.path(), c.path())),
        2,
        "clone is no git repo"
    );
    assert_eq!(
        main_with(args(f.dir.path(), plain.path())),
        2,
        "caller dir is no repo"
    );
    let mut a = args(f.dir.path(), c.path());
    a.insert(3, "--ref=nope".into());
    assert_eq!(main_with(a), 2, "ref not readable");
}

#[test]
fn aufrufer_repo_without_calls_is_info_not_red() {
    let f = fixture();
    assert_eq!(cli(&f, &["actions/checkout@v4".to_string()], &f.neutral), 0);
}

#[test]
fn aufrufer_mutants_are_caught() {
    let f = fixture();
    let strict = Rules::strict();
    let before = [u(NEW, &f.old)];
    let side = [u(NEW, &f.side)];
    let lower = [u("freesynergy-net", &f.l)];
    assert_eq!(red_count(&f, strict, &before), 1);
    assert_eq!(red_count(&f, strict, &side), 1);
    assert_eq!(red_count(&f, strict, &lower), 1);
    let no4 = Rules {
        contains_neutral: false,
        ..strict
    };
    assert_eq!(
        red_count(&f, no4, &before),
        0,
        "mutant rule 4 off: case `neu@SHA vor neutral` turns green, so the red test catches it"
    );
    let no5 = Rules {
        on_main: false,
        ..strict
    };
    assert_eq!(
        red_count(&f, no5, &side),
        0,
        "mutant rule 5 off: side branch turns green, caught"
    );
    let ci = Rules {
        case_insensitive_owner: true,
        ..strict
    };
    assert_eq!(
        red_count(&f, ci, &lower),
        0,
        "mutant case-insensitive: freesynergy-net turns green, caught"
    );
}
