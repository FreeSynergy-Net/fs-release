//! Pattern: none (integration test: index line, checksum, fresh resolution of a consumer, a changed .crate is rejected by cargo).

mod support;

use std::fs;

use fs_crate_index::publish;
use support::{ok, run, World};

#[test]
fn tag_run_writes_index_line_checksum_and_crate_in_one_commit() {
    let mut w = World::new();
    let before = w.head();
    let line = publish(&w.args("refs/tags/v0.0.7")).unwrap();
    assert!(line.contains("demo-lib 0.0.7"), "{line}");
    assert_ne!(before, w.head(), "one new commit");
    let count = ok(&run(
        &w.reg,
        "git",
        &["rev-list", "--count", &format!("{before}..HEAD")],
        &[],
    ));
    assert_eq!(count.trim(), "1", "exactly one commit");
    let files = ok(&run(
        &w.reg,
        "git",
        &["show", "--name-only", "--format=%s", "HEAD"],
        &[],
    ));
    assert!(files.contains("Updating crate `demo-lib#0.0.7`"), "{files}");
    assert!(
        files.contains("de/mo/demo-lib\n") || files.contains("de/mo/demo-lib"),
        "{files}"
    );
    assert!(
        files.contains("crates/demo-lib/demo-lib-0.0.7.crate"),
        "the .crate is in the same commit: {files}"
    );
    let index = fs::read_to_string(w.reg.join("de/mo/demo-lib")).unwrap();
    assert_eq!(index.lines().count(), 1);
    let crate_file = w.reg.join("crates/demo-lib/demo-lib-0.0.7.crate");
    let sum = ok(&run(
        &w.reg,
        "sha256sum",
        &[crate_file.to_str().unwrap()],
        &[],
    ));
    let sum = sum.split_whitespace().next().unwrap().to_string();
    assert!(
        index.contains(&format!("\"cksum\":\"{sum}\"")),
        "checksum of the index = sha256 of the file: {index} / {sum}"
    );
    assert!(
        index.contains("\"name\":\"demo-lib\"")
            && index.contains("\"vers\":\"0.0.7\"")
            && index.contains("\"yanked\":false"),
        "{index}"
    );
    assert_eq!(
        ok(&run(&w.reg, "git", &["status", "--porcelain"], &[])).trim(),
        "",
        "nothing left uncommitted"
    );
}

#[test]
fn consumer_resolves_the_newest_version_of_the_range_fresh() {
    let mut w = World::new();
    publish(&w.args("refs/tags/v0.0.7")).unwrap();
    w.set_version("0.0.8");
    publish(&w.args("refs/tags/v0.0.8")).unwrap();
    let c = w.consumer("~0.0.7");
    let home = w.cargo_home();
    let o = run(
        &c,
        "cargo",
        &["run", "-q"],
        &[
            ("CARGO_HOME", &home),
            ("CARGO_TARGET_DIR", &c.join("target")),
        ],
    );
    assert_eq!(ok(&o).trim(), "42");
    let lock = fs::read_to_string(c.join("Cargo.lock")).unwrap();
    assert_eq!(
        lock.matches("name = \"demo-lib\"").count(),
        1,
        "one source: {lock}"
    );
    assert!(
        lock.contains("version = \"0.0.8\""),
        "newest of ~0.0.7: {lock}"
    );
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
    ok(&run(
        &other,
        "cargo",
        &[
            "package",
            "--locked",
            "--target-dir",
            w.root.join("t-other").to_str().unwrap(),
        ],
        &[],
    ));
    fs::copy(
        w.root.join("t-other/package/demo-lib-0.0.7.crate"),
        w.reg.join("crates/demo-lib/demo-lib-0.0.7.crate"),
    )
    .unwrap();
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
