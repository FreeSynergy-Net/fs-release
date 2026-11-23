//! Pattern: none (test). Rules of `fs-release address`: global allow entries in a run per repo, too broad allow entries,
//! shell default values, lock fixtures as Cargo sources.

use std::fs;
use std::path::Path;
use std::process::Command;

use fs_release::address::mention::{mentions, Kind};
use fs_release::cli::main_with;

fn repo(files: &[(&str, &str)]) -> tempfile::TempDir {
    let t = tempfile::tempdir().unwrap();
    let git = |args: &[&str]| {
        let s = Command::new("git")
            .arg("-C")
            .arg(t.path())
            .args([
                "-c",
                "user.name=t",
                "-c",
                "user.email=t@t",
                "-c",
                "commit.gpgsign=false",
            ])
            .args(args)
            .status()
            .unwrap();
        assert!(s.success());
    };
    git(&["init", "-q", "-b", "main"]);
    for (p, c) in files {
        let f = t.path().join(p);
        fs::create_dir_all(f.parent().unwrap()).unwrap();
        fs::write(f, c).unwrap();
    }
    git(&["add", "-A"]);
    git(&["commit", "-q", "-m", "x"]);
    t
}

fn addr(args: &[&str], dirs: &[&Path]) -> i32 {
    let mut v: Vec<String> = ["fs-release", "address"]
        .iter()
        .map(ToString::to_string)
        .collect();
    v.extend(args.iter().map(ToString::to_string));
    v.extend(dirs.iter().map(|d| d.display().to_string()));
    main_with(v)
}

#[test]
fn address_global_allow_entry_does_not_make_a_run_for_one_repo_red() {
    let a = repo(&[("notes.txt", "see FreeSynergy/fs-a here\n")]);
    let b = repo(&[("README.md", "plain\n")]);
    let allow = a.path().join("../allow-global.json");
    fs::write(
        &allow,
        r#"[{"pattern":"fs-a here","reason":"history","scope":"notes.txt"}]"#,
    )
    .unwrap();
    let al = allow.to_str().unwrap();
    // only repo b: the global entry matches nothing there, which is no proof that it is stale.
    assert_eq!(
        addr(&["nachher", "--ref", "HEAD", "--allow", al], &[b.path()]),
        0
    );
    // both repos: the entry is used in a, green.
    assert_eq!(
        addr(
            &["nachher", "--ref", "HEAD", "--allow", al],
            &[a.path(), b.path()]
        ),
        0
    );
    // both repos and nothing matches: stale is red.
    fs::write(
        &allow,
        r#"[{"pattern":"no such text","reason":"old","scope":"notes.txt"}]"#,
    )
    .unwrap();
    assert_eq!(
        addr(
            &["nachher", "--ref", "HEAD", "--allow", al],
            &[a.path(), b.path()]
        ),
        1
    );
    // an entry for repo b judged in a run for repo a only is not judged; for its own repo it is.
    let name_b = b.path().file_name().unwrap().to_str().unwrap();
    fs::write(
        &allow,
        format!(r#"[{{"pattern":"no such text","reason":"old","scope":"x/","repo":"{name_b}"}}]"#),
    )
    .unwrap();
    assert_eq!(
        addr(&["bestand", "--ref", "HEAD", "--allow", al], &[a.path()]),
        0
    );
    assert_eq!(
        addr(&["bestand", "--ref", "HEAD", "--allow", al], &[b.path()]),
        1
    );
}

#[test]
fn address_too_broad_unscoped_allow_entry_is_refused() {
    let r = repo(&[("README.md", "see https://github.com/FreeSynergy-Net/fs-x\n")]);
    let allow = r.path().join("../allow-broad.json");
    let run = |json: &str| {
        fs::write(&allow, json).unwrap();
        addr(
            &[
                "nachher",
                "--ref",
                "HEAD",
                "--allow",
                allow.to_str().unwrap(),
            ],
            &[r.path()],
        )
    };
    assert_eq!(
        run(r#"[{"pattern":"freesynergy","reason":"all"}]"#),
        2,
        "owner name without scope"
    );
    assert_eq!(
        run(r#"[{"pattern":"synergy","reason":"all"}]"#),
        2,
        "part of the owner name"
    );
    assert_eq!(
        run(r#"[{"pattern":"/","reason":"all"}]"#),
        2,
        "one character"
    );
    assert_eq!(
        run(r#"[{"pattern":"freesynergy","reason":"history","scope":"docs/adr/"}]"#),
        1,
        "with a scope it is accepted (here stale)"
    );
    assert_eq!(
        run(r#"[{"pattern":"fs-x","reason":"one line"}]"#),
        0,
        "a narrow pattern without scope is fine"
    );
}

#[test]
fn address_shell_default_value_is_found_but_xdg_and_prose_are_not() {
    let kinds = |l: &str| {
        mentions(l, "FreeSynergy")
            .iter()
            .map(|m| m.kind)
            .collect::<Vec<_>>()
    };
    assert_eq!(
        kinds("REPO=\"${GITHUB_REPOSITORY:-FreeSynergy-Net/Catalog}\""),
        [Kind::Safe]
    );
    assert_eq!(kinds("x=${R:=FreeSynergy-Net/fs-x}"), [Kind::Safe]);
    for l in [
        "~/.config/freesynergy/tasks",
        "dir=${XDG:-/home/x/.config/freesynergy/tasks}",
        "@freesynergy/ui",
        "the FreeSynergy-Net project",
        "a-FreeSynergy/x",
    ] {
        assert!(kinds(l).is_empty(), "{l}");
    }
}

#[test]
fn address_lock_fixtures_are_cargo_sources_and_stay() {
    let fx = "source = \"git+https://github.com/FreeSynergy-Net/fs-x#abc\"\n";
    let r = repo(&[
        ("tests/fixtures/lock-sources/green.lock", fx),
        (
            "tests/packaging/rules.sh",
            "[patch.\"https://github.com/FreeSynergy-Net/fs-x\"]\n",
        ),
        ("README.md", "https://github.com/FreeSynergy-Net/fs-x\n"),
    ]);
    assert_eq!(addr(&["ersetzen"], &[r.path()]), 0);
    assert_eq!(
        fs::read_to_string(r.path().join("tests/fixtures/lock-sources/green.lock")).unwrap(),
        fx
    );
    assert!(
        fs::read_to_string(r.path().join("tests/packaging/rules.sh"))
            .unwrap()
            .contains("FreeSynergy/fs-x")
    );
    assert!(fs::read_to_string(r.path().join("README.md"))
        .unwrap()
        .contains("FreeSynergy-Net"));
}
