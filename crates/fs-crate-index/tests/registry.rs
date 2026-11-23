//! Pattern: none (integration test). A throw-away registry (git index + `.crate` files, `file://`, no GitHub, no network): index line, checksum,
//! fresh resolution of a consumer with `registry = "freesynergy"`, counter-test "a changed .crate is rejected by cargo", counter-test "a branch run writes nothing".

use std::fs;
use std::path::{Path, PathBuf};
use std::process::{Command, Output};

use fs_crate_index::{check_dl, check_features, publish, version_of_ref, PublishArgs};

fn run(dir: &Path, cmd: &str, args: &[&str], envs: &[(&str, &Path)]) -> Output {
    let mut c = Command::new(cmd);
    c.args(args).current_dir(dir).env("GIT_AUTHOR_NAME", "t").env("GIT_AUTHOR_EMAIL", "t@example.org").env("GIT_COMMITTER_NAME", "t").env("GIT_COMMITTER_EMAIL", "t@example.org");
    for (k, v) in envs {
        c.env(k, v);
    }
    c.output().unwrap()
}

fn ok(o: &Output) -> String {
    assert!(o.status.success(), "{}{}", String::from_utf8_lossy(&o.stdout), String::from_utf8_lossy(&o.stderr));
    String::from_utf8_lossy(&o.stdout).into_owned()
}

struct World {
    _tmp: tempfile::TempDir,
    root: PathBuf,
    lib: PathBuf,
    reg: PathBuf,
    url: String,
    n: u32,
}

impl World {
    fn new() -> Self {
        for (k, v) in [("GIT_AUTHOR_NAME", "t"), ("GIT_AUTHOR_EMAIL", "t@example.org"), ("GIT_COMMITTER_NAME", "t"), ("GIT_COMMITTER_EMAIL", "t@example.org")] {
            std::env::set_var(k, v);
        }
        // TEST_TMPDIR: a folder OUTSIDE every cargo workspace (a nested fixture would believe it is a workspace member)
        let tmp = match std::env::var("TEST_TMPDIR") {
            Ok(d) => tempfile::Builder::new().prefix("k2-").tempdir_in(d).unwrap(),
            Err(_) => tempfile::tempdir().unwrap(),
        };
        let root = tmp.path().to_path_buf();
        let reg = root.join("fs-crates");
        let url = format!("file://{}", reg.display());
        reg_index::init(&reg, &format!("{url}/crates/{{crate}}/{{crate}}-{{version}}.crate"), None).unwrap();
        let lib = root.join("demo-lib");
        fs::create_dir_all(lib.join("src")).unwrap();
        fs::write(lib.join("src/lib.rs"), "pub fn answer() -> u32 { 42 }\n").unwrap();
        let w = Self { _tmp: tmp, root, lib, reg, url, n: 0 };
        w.set_version("0.0.7");
        w
    }
    fn set_version(&self, v: &str) {
        fs::write(
            self.lib.join("Cargo.toml"),
            format!("[package]\nname = \"demo-lib\"\nversion = \"{v}\"\nedition = \"2021\"\nlicense = \"MIT\"\ndescription = \"demo\"\n"),
        )
        .unwrap();
        ok(&run(&self.lib, "cargo", &["generate-lockfile"], &[]));
    }
    fn args(&mut self, git_ref: &str) -> PublishArgs {
        self.n += 1;
        PublishArgs {
            git_ref: git_ref.into(),
            package: "demo-lib".into(),
            workspace: self.lib.clone(),
            registry_dir: self.reg.clone(),
            index_url: self.url.clone(),
            upload: "crates/{crate}".into(),
            target_dir: self.root.join(format!("target-{}", self.n)),
        }
    }
    fn head(&self) -> String {
        ok(&run(&self.reg, "git", &["rev-parse", "HEAD"], &[])).trim().into()
    }
    fn consumer(&self, req: &str) -> PathBuf {
        let c = self.root.join(format!("consumer-{}", req.replace(['~', '.', '='], "_")));
        fs::create_dir_all(c.join("src")).unwrap();
        fs::create_dir_all(c.join(".cargo")).unwrap();
        fs::write(c.join("src/main.rs"), "fn main() { println!(\"{}\", demo_lib::answer()); }\n").unwrap();
        fs::write(c.join(".cargo/config.toml"), format!("[registries.freesynergy]\nindex = \"{}\"\n", self.url)).unwrap();
        fs::write(
            c.join("Cargo.toml"),
            format!("[package]\nname = \"consumer\"\nversion = \"0.1.0\"\nedition = \"2021\"\n\n[dependencies]\ndemo-lib = {{ version = \"{req}\", registry = \"freesynergy\" }}\n"),
        )
        .unwrap();
        c
    }
    fn cargo_home(&self) -> PathBuf {
        let h = self.root.join("cargo-home");
        fs::create_dir_all(&h).unwrap();
        h
    }
}

#[test]
fn ref_guard_accepts_only_a_plain_tag() {
    assert_eq!(version_of_ref("refs/tags/v0.0.7").unwrap(), "0.0.7");
    for bad in ["refs/heads/main", "refs/heads/v0.0.7", "refs/pull/3/merge", "", "v0.0.7", "refs/tags/0.0.7", "refs/tags/v0.0.7-rc1", "refs/tags/v0.0", "refs/tags/v0.0.07", "refs/tags/v1.2.3+b"] {
        assert!(version_of_ref(bad).is_err(), "{bad} must be refused");
    }
}

#[test]
fn v2_features_are_refused() {
    let mut f = std::collections::BTreeMap::new();
    f.insert("a".to_string(), vec!["b".to_string()]);
    assert!(check_features(&f).is_ok());
    f.insert("c".to_string(), vec!["dep:serde".to_string()]);
    assert!(check_features(&f).is_err());
    f.remove("c");
    f.insert("d".to_string(), vec!["serde?/derive".to_string()]);
    assert!(check_features(&f).is_err());
}

#[test]
fn tag_run_writes_index_line_checksum_and_crate_in_one_commit() {
    let mut w = World::new();
    let before = w.head();
    let line = publish(&w.args("refs/tags/v0.0.7")).unwrap();
    assert!(line.contains("demo-lib 0.0.7"), "{line}");
    assert_ne!(before, w.head(), "one new commit");
    let count = ok(&run(&w.reg, "git", &["rev-list", "--count", &format!("{before}..HEAD")], &[]));
    assert_eq!(count.trim(), "1", "exactly one commit");
    let files = ok(&run(&w.reg, "git", &["show", "--name-only", "--format=%s", "HEAD"], &[]));
    assert!(files.contains("Updating crate `demo-lib#0.0.7`"), "{files}");
    assert!(files.contains("de/mo/demo-lib\n") || files.contains("de/mo/demo-lib"), "{files}");
    assert!(files.contains("crates/demo-lib/demo-lib-0.0.7.crate"), "the .crate is in the same commit: {files}");
    let index = fs::read_to_string(w.reg.join("de/mo/demo-lib")).unwrap();
    assert_eq!(index.lines().count(), 1);
    let crate_file = w.reg.join("crates/demo-lib/demo-lib-0.0.7.crate");
    let sum = ok(&run(&w.reg, "sha256sum", &[crate_file.to_str().unwrap()], &[]));
    let sum = sum.split_whitespace().next().unwrap().to_string();
    assert!(index.contains(&format!("\"cksum\":\"{sum}\"")), "checksum of the index = sha256 of the file: {index} / {sum}");
    assert!(index.contains("\"name\":\"demo-lib\"") && index.contains("\"vers\":\"0.0.7\"") && index.contains("\"yanked\":false"), "{index}");
    assert_eq!(ok(&run(&w.reg, "git", &["status", "--porcelain"], &[])).trim(), "", "nothing left uncommitted");
}

#[test]
fn consumer_resolves_the_newest_version_of_the_range_fresh() {
    let mut w = World::new();
    publish(&w.args("refs/tags/v0.0.7")).unwrap();
    w.set_version("0.0.8");
    publish(&w.args("refs/tags/v0.0.8")).unwrap();
    let c = w.consumer("~0.0.7");
    let home = w.cargo_home();
    let o = run(&c, "cargo", &["run", "-q"], &[("CARGO_HOME", &home), ("CARGO_TARGET_DIR", &c.join("target"))]);
    assert_eq!(ok(&o).trim(), "42");
    let lock = fs::read_to_string(c.join("Cargo.lock")).unwrap();
    assert_eq!(lock.matches("name = \"demo-lib\"").count(), 1, "one source: {lock}");
    assert!(lock.contains("version = \"0.0.8\""), "newest of ~0.0.7: {lock}");
    assert!(lock.contains(&format!("registry+{}", w.url)), "{lock}");
}

#[test]
fn a_changed_crate_file_is_rejected_by_cargo() {
    let mut w = World::new();
    publish(&w.args("refs/tags/v0.0.7")).unwrap();
    // mutant: a different (valid) crate under the same name and version, the checksum in the index stays
    let other = w.root.join("other");
    fs::create_dir_all(other.join("src")).unwrap();
    fs::write(other.join("src/lib.rs"), "pub fn answer() -> u32 { 13 }\n").unwrap();
    fs::copy(w.lib.join("Cargo.toml"), other.join("Cargo.toml")).unwrap();
    ok(&run(&other, "cargo", &["generate-lockfile"], &[]));
    ok(&run(&other, "cargo", &["package", "--locked", "--target-dir", w.root.join("t-other").to_str().unwrap()], &[]));
    fs::copy(w.root.join("t-other/package/demo-lib-0.0.7.crate"), w.reg.join("crates/demo-lib/demo-lib-0.0.7.crate")).unwrap();
    let c = w.consumer("~0.0.7");
    let home = w.cargo_home();
    let o = run(&c, "cargo", &["fetch"], &[("CARGO_HOME", &home)]);
    assert!(!o.status.success(), "cargo must refuse the changed .crate");
    let err = String::from_utf8_lossy(&o.stderr).to_lowercase();
    assert!(err.contains("checksum"), "{err}");
    // control: the untouched crate is accepted (the test above would be green without the mutant too, if the fetch never worked)
    let mut w2 = World::new();
    publish(&w2.args("refs/tags/v0.0.7")).unwrap();
    let c2 = w2.consumer("~0.0.7");
    let home2 = w2.cargo_home();
    ok(&run(&c2, "cargo", &["fetch"], &[("CARGO_HOME", &home2)]));
}

#[test]
fn a_branch_or_pr_run_or_a_wrong_tag_writes_nothing() {
    let mut w = World::new();
    let before = w.head();
    for r in ["refs/heads/main", "refs/pull/1/merge", "", "refs/tags/v0.0.9"] {
        let e = publish(&w.args(r)).unwrap_err();
        assert!(!format!("{e:#}").is_empty());
        assert_eq!(w.head(), before, "{r}: no commit");
        assert!(!w.reg.join("crates").exists() && !w.reg.join("de").exists(), "{r}: no file");
        assert_eq!(ok(&run(&w.reg, "git", &["status", "--porcelain"], &[])).trim(), "", "{r}: clean");
    }
}

#[test]
fn a_version_already_in_the_index_is_never_replaced() {
    let mut w = World::new();
    publish(&w.args("refs/tags/v0.0.7")).unwrap();
    let head = w.head();
    let before = fs::read(w.reg.join("crates/demo-lib/demo-lib-0.0.7.crate")).unwrap();
    assert!(publish(&w.args("refs/tags/v0.0.7")).is_err());
    assert_eq!(w.head(), head);
    assert_eq!(fs::read(w.reg.join("crates/demo-lib/demo-lib-0.0.7.crate")).unwrap(), before);
}

#[test]
fn dl_that_does_not_lead_to_the_stored_file_is_refused() {
    let w = World::new();
    assert!(check_dl(&w.reg, "crates/{crate}", "demo-lib", "0.0.7").is_ok());
    assert!(check_dl(&w.reg, "elsewhere/{crate}", "demo-lib", "0.0.7").is_err());
    let none = w.root.join("nothing");
    fs::create_dir_all(&none).unwrap();
    assert!(check_dl(&none, "crates/{crate}", "demo-lib", "0.0.7").is_err(), "no config.json");
}
