//! Tests of `fs-release address` (owner swap): errors are red, after-check finds what replace leaves, Cargo sources stay, stale allow is red.

use std::fs;
use std::path::Path;
use std::process::Command;

use fs_release::address::mention::{mentions, swap_safe, Kind};
use fs_release::address::scan::scan;
use fs_release::cli::main_with;

fn git(dir: &Path, args: &[&str]) {
    let s = Command::new("git")
        .arg("-C")
        .arg(dir)
        .args(["-c", "user.name=t", "-c", "user.email=t@t", "-c", "commit.gpgsign=false"])
        .args(args)
        .status()
        .unwrap();
    assert!(s.success(), "git {args:?}");
}

fn repo(files: &[(&str, &str)]) -> tempfile::TempDir {
    let t = tempfile::tempdir().unwrap();
    git(t.path(), &["init", "-q", "-b", "main"]);
    for (p, c) in files {
        let f = t.path().join(p);
        fs::create_dir_all(f.parent().unwrap()).unwrap();
        fs::write(f, c).unwrap();
    }
    git(t.path(), &["add", "-A"]);
    git(t.path(), &["commit", "-q", "-m", "x"]);
    t
}

fn addr(args: &[&str], dir: &Path) -> i32 {
    let mut v: Vec<String> = ["fs-release", "address"].iter().map(ToString::to_string).collect();
    v.extend(args.iter().map(ToString::to_string));
    v.push(dir.display().to_string());
    main_with(v)
}

const MIX: &[(&str, &str)] = &[
    ("README.md", "see https://github.com/FreeSynergy/fs-x for more\n"),
    (".github/workflows/ci.yml", "      - uses: FreeSynergy/fs-release/actions/a@abc\n"),
    ("list.toml", "slugs = [\"FreeSynergy/fs-y\"]\n"),
    ("Cargo.toml", "dep = { git = \"https://github.com/FreeSynergy/fs-x\" }\n"),
    ("Cargo.lock", "source = \"git+https://github.com/FreeSynergy/fs-x#abc\"\n"),
    ("scripts/lock-sources.toml", "fs-types = \"https://github.com/FreeSynergy/fs-types\"\n"),
    (".cargo/config.toml", "[patch.\"https://github.com/FreeSynergy/fs-x\"]\n"),
];

#[test]
fn address_error_is_red_not_zero_hits() {
    let r = repo(MIX);
    assert_eq!(addr(&["nachher", "--ref", "origin/doesnotexist"], r.path()), 2, "missing ref");
    assert_eq!(addr(&["bestand", "--ref", "origin/main"], r.path()), 2, "repo without origin/main");
    let plain = tempfile::tempdir().unwrap();
    assert_eq!(addr(&["bestand", "--ref", "HEAD"], plain.path()), 2, "not a repo");
    let bad = r.path().join("allow.json");
    fs::write(&bad, "[{ not json").unwrap();
    assert_eq!(addr(&["bestand", "--ref", "HEAD", "--allow", bad.to_str().unwrap()], r.path()), 2, "broken allow file");
    assert_eq!(addr(&["bestand", "--ref", "HEAD", "--allow", "/nonexistent/allow.json"], r.path()), 2, "missing allow file");
}

#[test]
fn address_nachher_finds_what_ersetzen_left() {
    let r = repo(MIX);
    assert_eq!(addr(&["ersetzen"], r.path()), 0);
    git(r.path(), &["commit", "-q", "-am", "swap"]);
    assert_eq!(fs::read_to_string(r.path().join("README.md")).unwrap(), "see https://github.com/FreeSynergy-Net/fs-x for more\n");
    // the slug in list.toml is no safe form: replace leaves it, the broad after-check must find it.
    let hits = scan(r.path(), "HEAD", "FreeSynergy").unwrap();
    assert!(hits.iter().any(|h| h.path == "list.toml"), "list.toml must be found");
    assert_eq!(addr(&["nachher", "--ref", "HEAD"], r.path()), 1, "RED while list.toml still names the old owner");
    let allow = r.path().join("allow.json");
    fs::write(&allow, r#"[{"pattern":"fs-y","reason":"hand work later","scope":"list.toml"}]"#).unwrap();
    assert_eq!(addr(&["nachher", "--ref", "HEAD", "--allow", allow.to_str().unwrap()], r.path()), 0, "green: only Cargo classes and allow are left");
    assert_eq!(addr(&["nachher", "--all-classes", "--ref", "HEAD", "--allow", allow.to_str().unwrap()], r.path()), 1, "Cargo classes count after part 2");
}

#[test]
fn address_cargo_sources_and_lock_sources_stay() {
    let r = repo(MIX);
    assert_eq!(addr(&["ersetzen"], r.path()), 0);
    for (p, c) in MIX.iter().filter(|(p, _)| p.contains("Cargo") || p.contains("lock-sources") || p.contains(".cargo")) {
        assert_eq!(&fs::read_to_string(r.path().join(p)).unwrap(), c, "{p} must stay untouched");
    }
    assert!(fs::read_to_string(r.path().join(".github/workflows/ci.yml")).unwrap().contains("uses: FreeSynergy-Net/fs-release/"));
}

#[test]
fn address_stale_allow_entry_is_red() {
    let r = repo(&[("README.md", "see https://github.com/FreeSynergy/fs-x\n")]);
    let allow = r.path().join("allow.json");
    fs::write(&allow, r#"[{"pattern":"nothing like this","reason":"old","scope":"docs/"}]"#).unwrap();
    assert_eq!(addr(&["bestand", "--ref", "HEAD", "--allow", allow.to_str().unwrap()], r.path()), 1);
    fs::write(&allow, r#"[{"pattern":"fs-x","reason":"","scope":""}]"#).unwrap();
    assert_eq!(addr(&["bestand", "--ref", "HEAD", "--allow", allow.to_str().unwrap()], r.path()), 2, "entry without reason");
}

#[test]
fn address_mention_scanner_forms() {
    let o = "FreeSynergy";
    let kinds = |l: &str| mentions(l, o).iter().map(|m| m.kind).collect::<Vec<_>>();
    assert_eq!(kinds("git@github.com:FreeSynergy/fs-x.git"), [Kind::Safe]);
    assert_eq!(kinds("repository: FreeSynergy/fs-documentation"), [Kind::Safe]);
    assert_eq!(kinds("gh api -R FreeSynergy/Catalog"), [Kind::Safe]);
    assert_eq!(kinds("ORG = \"FreeSynergy\""), [Kind::Safe]);
    assert_eq!(kinds("x = \"FreeSynergy/fs-y\""), [Kind::Other]);
    assert_eq!(kinds("docker pull ghcr.io/freesynergy/img"), [Kind::Ghcr]);
    assert_eq!(kinds("github\\.com[/:]FreeSynergy/x"), [Kind::Safe]);
    // no hits: XDG paths, npm scope, new owner, project name in prose, domain.
    for l in ["~/.config/freesynergy/tasks", "/srv/FreeSynergy/.scratch", "@freesynergy/ui", "github.com/FreeSynergy-Net/x", "FreeSynergy is a project", "support@freesynergy.net"] {
        assert!(kinds(l).is_empty(), "{l}");
    }
    // a half-swapped line still counts, and the swap leaves the new owner alone.
    let mixed = "a github.com/FreeSynergy-Net/x b github.com/FreeSynergy/y";
    assert_eq!(kinds(mixed), [Kind::Safe]);
    assert_eq!(swap_safe(mixed, o, "FreeSynergy-Net").unwrap(), "a github.com/FreeSynergy-Net/x b github.com/FreeSynergy-Net/y");
}
