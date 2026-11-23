//! Pattern: none (test). Chain gate `check-pin-chain`: the code reached through the self-pins equals the code at the start head.
//! One test per level and per chain, so no check hides behind another (the mutants "level 1 check removed" and "level 2 check removed"
//! each turn exactly one test red).

use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

use fs_release::cli::main_with;
use fs_release::pin_chain::check;

const ZERO: &str = "0000000000000000000000000000000000000000";
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

/// The files of one state; `{P}` in a text is the pin of the round.
struct Repo {
    dir: tempfile::TempDir,
    pub_sh: String,
    pub_yml: String,
    sign_sh: String,
}

impl Repo {
    fn new() -> Self {
        let r = Self {
            dir: tempfile::tempdir().unwrap(),
            pub_sh: "publish v1".into(),
            pub_yml: "name: pub".into(),
            sign_sh: "sign v1".into(),
        };
        git(r.path(), &["init", "-q", "-b", "main"]);
        r
    }

    fn path(&self) -> &Path {
        self.dir.path()
    }

    fn write(&self, rel: &str, text: &str) {
        let p: PathBuf = self.path().join(rel);
        fs::create_dir_all(p.parent().unwrap()).unwrap();
        fs::write(p, text).unwrap();
    }

    /// Commit the current state with every self-pin set to `pin`:
    /// release.yml -> _pub.yml -> actions/pub, fin.yml -> actions/sign.
    fn commit(&self, pin: &str) -> String {
        self.write(
            ".github/workflows/release.yml",
            &format!("jobs:\n  a:\n    uses: {O}/.github/workflows/_pub.yml@{pin}\n"),
        );
        self.write(
            ".github/workflows/_pub.yml",
            &format!(
                "{}\njobs:\n  b:\n    steps:\n      - uses: {O}/actions/pub@{pin}\n",
                self.pub_yml
            ),
        );
        self.write(
            ".github/workflows/fin.yml",
            &format!("jobs:\n  c:\n    steps:\n      - uses: {O}/actions/sign@{pin}\n"),
        );
        self.write("actions/pub/pub.sh", &self.pub_sh);
        self.write("actions/sign/sign.sh", &self.sign_sh);
        git(self.path(), &["add", "-A"]);
        git(
            self.path(),
            &["commit", "-q", "-m", "state", "--allow-empty"],
        );
        git(self.path(), &["rev-parse", "HEAD"])
    }

    /// Bootstrap and two pin rounds: the fixed point. Returns (c0, c1, c2).
    fn fixed_point(&self) -> (String, String, String) {
        let c0 = self.commit(ZERO);
        let c1 = self.commit(&c0);
        let c2 = self.commit(&c1);
        (c0, c1, c2)
    }
}

fn red(r: &Repo, start: &str) -> Vec<String> {
    check(r.path(), start).unwrap()
}

#[test]
fn the_fixed_point_is_green() {
    let r = Repo::new();
    let (_, _, c2) = r.fixed_point();
    assert_eq!(red(&r, &c2), Vec::<String>::new());
}

#[test]
fn a_round_that_only_changes_pins_stays_green() {
    let r = Repo::new();
    let (_, _, c2) = r.fixed_point();
    let c3 = r.commit(&c2);
    assert_eq!(red(&r, &c3), Vec::<String>::new());
}

#[test]
fn the_state_after_one_round_is_red_at_the_second_level() {
    // c1 pins c0, whose _pub.yml still pins the bootstrap placeholder: the action is not reached.
    let r = Repo::new();
    let (_, c1, _) = r.fixed_point();
    let found = red(&r, &c1);
    assert!(
        found
            .iter()
            .any(|f| f.contains("is no commit of this repository")),
        "{found:?}"
    );
}

#[test]
fn only_the_first_level_differs() {
    let mut r = Repo::new();
    let (_, _, _) = r.fixed_point();
    r.pub_yml = "name: pub\n# changed".into();
    let s = r.commit(&git(r.path(), &["rev-parse", "HEAD"]));
    let found = red(&r, &s);
    assert!(
        found.iter().any(|f| f.contains("_pub.yml at pin")),
        "{found:?}"
    );
    assert!(!found.iter().any(|f| f.contains("pub.sh")), "{found:?}");
}

#[test]
fn only_the_second_level_differs() {
    let mut r = Repo::new();
    let (_, _, _) = r.fixed_point();
    r.pub_sh = "publish v2".into();
    let s = r.commit(&git(r.path(), &["rev-parse", "HEAD"]));
    let found = red(&r, &s);
    assert!(
        found
            .iter()
            .any(|f| f.contains("actions/pub/pub.sh at pin")),
        "{found:?}"
    );
    assert!(
        !found.iter().any(|f| f.contains("_pub.yml at pin")),
        "{found:?}"
    );
}

#[test]
fn only_another_chain_differs() {
    let mut r = Repo::new();
    let (_, _, _) = r.fixed_point();
    r.sign_sh = "sign v2".into();
    let s = r.commit(&git(r.path(), &["rev-parse", "HEAD"]));
    let found = red(&r, &s);
    assert!(
        found.iter().any(|f| f.contains("actions/sign/sign.sh")),
        "{found:?}"
    );
    assert!(!found.iter().any(|f| f.contains("pub")), "{found:?}");
}

#[test]
fn two_rounds_close_a_change_of_the_action() {
    let mut r = Repo::new();
    let (_, _, _) = r.fixed_point();
    r.pub_sh = "publish v2".into();
    let s = r.commit(&git(r.path(), &["rev-parse", "HEAD"]));
    assert_ne!(red(&r, &s), Vec::<String>::new());
    let s1 = r.commit(&s);
    assert_ne!(
        red(&r, &s1),
        Vec::<String>::new(),
        "one round is not enough (depth 2)"
    );
    let s2 = r.commit(&s1);
    assert_eq!(red(&r, &s2), Vec::<String>::new());
}

#[test]
fn a_pin_that_is_no_full_sha_is_red() {
    let r = Repo::new();
    let _ = r.fixed_point();
    let s = r.commit("v0.0.17");
    let found = red(&r, &s);
    assert!(
        found.iter().any(|f| f.contains("not a full commit SHA")),
        "{found:?}"
    );
}

#[test]
fn a_pin_to_a_commit_without_the_file_is_red() {
    let r = Repo::new();
    let _ = r.fixed_point();
    git(r.path(), &["commit", "-q", "--allow-empty", "-m", "x"]);
    let empty = {
        git(r.path(), &["checkout", "-q", "--orphan", "bare"]);
        git(r.path(), &["rm", "-rf", "-q", "."]);
        git(r.path(), &["commit", "-q", "--allow-empty", "-m", "empty"]);
        let e = git(r.path(), &["rev-parse", "HEAD"]);
        git(r.path(), &["checkout", "-q", "main"]);
        e
    };
    let s = r.commit(&empty);
    let found = red(&r, &s);
    assert!(
        found.iter().any(|f| f.contains("does not exist at pin")),
        "{found:?}"
    );
}

#[test]
fn bad_input_is_an_error_not_a_green() {
    let r = Repo::new();
    let _ = r.fixed_point();
    assert!(check(r.path(), "no-such-ref").is_err());
    let lone = tempfile::tempdir().unwrap();
    git(lone.path(), &["init", "-q", "-b", "main"]);
    git(lone.path(), &["commit", "-q", "--allow-empty", "-m", "x"]);
    assert!(
        check(lone.path(), "HEAD").is_err(),
        "no pin at all is no green"
    );
}

#[test]
fn the_command_is_hard_by_default_and_only_reports_with_melden() {
    let mut r = Repo::new();
    let _ = r.fixed_point();
    r.pub_sh = "publish v2".into();
    let s = r.commit(&git(r.path(), &["rev-parse", "HEAD"]));
    let base = |extra: &[&str]| {
        let mut a: Vec<String> = [
            "fs-release",
            "check-pin-chain",
            "--repo",
            r.path().to_str().unwrap(),
        ]
        .map(String::from)
        .to_vec();
        a.extend(extra.iter().map(|x| (*x).to_string()));
        a.push(s.clone());
        main_with(a)
    };
    assert_eq!(base(&[]), 1);
    assert_eq!(base(&["--melden"]), 0);
    assert_eq!(
        main_with(
            [
                "fs-release",
                "check-pin-chain",
                "--repo",
                r.path().to_str().unwrap(),
                "nope"
            ]
            .map(String::from)
            .to_vec()
        ),
        2
    );
}
