//! Pattern: none (test). `fs-release address aufrufer`, review round 2: repo-root rule, non-ASCII names, look-alike repo name,
//! composite actions, comment part, and the clone HEAD that is not main (kills the origin/main -> HEAD mutant).

mod support;
use std::fs;
use support::aufrufer::{caller, caller_named, cli, cli_dir, fixture, git, u, NEW};

#[test]
fn aufrufer_subfolder_of_a_repo_is_an_error_not_green() {
    let f = fixture();
    let c = caller(&[u("FreeSynergy", "main")]);
    assert_eq!(cli_dir(&f, c.path()), 1, "root: red");
    let sub = c.path().join(".github");
    assert_eq!(cli_dir(&f, &sub), 2, "subfolder: rc 2");
}

#[test]
fn aufrufer_non_ascii_workflow_name_is_read() {
    let f = fixture();
    let c = caller_named("ä.yml", &[u("FreeSynergy", &f.l)]);
    assert_eq!(cli_dir(&f, c.path()), 1, "ä.yml with the old owner is red");
    let ok = caller_named("ä.yml", &[u(NEW, &f.l)]);
    assert_eq!(cli_dir(&f, ok.path()), 0);
}

#[test]
fn aufrufer_head_of_clone_is_not_the_main_ref() {
    let f = fixture();
    assert_ne!(
        git(f.dir.path(), &["rev-parse", "HEAD"]),
        git(f.dir.path(), &["rev-parse", "origin/main"])
    );
    assert_eq!(
        cli(&f, &[u(NEW, &f.l)], &f.neutral),
        0,
        "L is on origin/main"
    );
    assert_eq!(
        cli(&f, &[u(NEW, &f.side)], &f.neutral),
        1,
        "side is HEAD but not on origin/main"
    );
}

#[test]
fn aufrufer_look_alike_repo_name_is_recognised_and_red() {
    let f = fixture();
    let n = format!("{NEW}/FS-Release/.github/workflows/check.yml@{}", f.l);
    assert_eq!(
        cli(&f, &[n], &f.neutral),
        1,
        "FS-Release is a look-alike: red, not skipped"
    );
    assert_eq!(
        cli(&f, &[format!("{NEW}/xfs-release/a.yml@main")], &f.neutral),
        0,
        "different name: not ours"
    );
}

#[test]
fn aufrufer_comment_part_and_composite_actions() {
    let f = fixture();
    assert_eq!(
        cli(&f, &[format!("{} # v0.0.19", u(NEW, &f.l))], &f.neutral),
        0,
        "`@<L> # v0.0.x` green"
    );
    assert_eq!(
        cli(&f, &[format!("{} # {}", u(NEW, "main"), f.l)], &f.neutral),
        1,
        "`@main # <L>` red"
    );
    for dir in [".github/actions/x", "actions/x"] {
        let c = caller(&[]);
        fs::create_dir_all(c.path().join(dir)).unwrap();
        fs::write(
            c.path().join(dir).join("action.yaml"),
            format!("      - uses: {}\n", u("FreeSynergy", &f.l)),
        )
        .unwrap();
        git(c.path(), &["add", "-A"]);
        git(c.path(), &["commit", "-q", "-m", "a"]);
        assert_eq!(cli_dir(&f, c.path()), 1, "{dir}/action.yaml is read");
    }
}
