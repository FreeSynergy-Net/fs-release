//! Pattern: none (integration test: the pull path end to end: scan -> apply, index lines, v2 features, fresh resolution of a consumer,
//! a changed .crate is rejected by cargo, replay, semver gate wired).

mod support;

use std::fs;

use fs_crate_index::{apply, scan};
use support::{ok, run, World};

#[test]
fn scan_and_apply_enter_three_crates_in_order_with_v2_features() {
    let mut w = World::new();
    w.release("0.0.7", 42);
    let before = w.head();
    let s = w.scan_args();
    let rep = scan(&s).unwrap();
    assert!(rep.error.is_none(), "{:?}", rep.error);
    assert_eq!(rep.entries, 3);
    assert_eq!(
        w.head(),
        before,
        "scan (job 1) writes nothing to the registry"
    );
    let done = apply(&w.apply_args(&s)).unwrap();
    assert_eq!(done.len(), 3);
    let count = ok(&run(
        &w.reg,
        "git",
        &["rev-list", "--count", &format!("{before}..HEAD")],
        &[],
    ));
    assert_eq!(count.trim(), "3", "one commit per crate");
    let log = ok(&run(&w.reg, "git", &["log", "--format=%B", "-3"], &[]));
    let sha = ok(&run(&w.src, "git", &["rev-parse", "v0.0.7^{commit}"], &[]));
    assert!(
        log.contains(&format!("source: owner/demo v0.0.7 commit {}", sha.trim())),
        "tag commit in the message: {log}"
    );
    let first = ok(&run(
        &w.reg,
        "git",
        &["show", "--name-only", "--format=%s", "HEAD~2"],
        &[],
    ));
    assert!(
        first.contains("demo-base#0.0.7")
            && first.contains("crates/demo-base/demo-base-0.0.7.crate"),
        "{first}"
    );
    // the optional dependency feature: `dep:` goes to features2 with v 2 (M1: such a crate is NOT refused)
    let app = fs::read_to_string(w.reg.join("de/mo/demo-app")).unwrap();
    assert!(
        app.contains("\"features2\":{\"conf\":[\"dep:demo-lib\"]}") && app.contains("\"v\":2"),
        "{app}"
    );
    assert!(
        app.contains("\"features\":{}"),
        "the v2 feature is not in `features`: {app}"
    );
    let sum = ok(&run(
        &w.reg,
        "sha256sum",
        &[w.reg
            .join("crates/demo-app/demo-app-0.0.7.crate")
            .to_str()
            .unwrap()],
        &[],
    ));
    assert!(
        app.contains(&format!(
            "\"cksum\":\"{}\"",
            sum.split_whitespace().next().unwrap()
        )),
        "{app}"
    );
    assert!(
        !app.contains("\"registry\":\""),
        "same-registry dependencies are stored as such: {app}"
    );
    assert_eq!(
        ok(&run(&w.reg, "git", &["status", "--porcelain"], &[])).trim(),
        ""
    );
    // replay: nothing new, nothing written
    let head = w.head();
    let s2 = w.scan_args();
    let r2 = scan(&s2).unwrap();
    assert!(r2.error.is_none() && r2.entries == 0, "{r2:?}");
    assert_eq!(w.head(), head);
}

#[test]
fn consumer_resolves_fresh_with_the_v2_feature() {
    let mut w = World::new();
    w.release("0.0.7", 42);
    let s = w.scan_args();
    scan(&s).unwrap();
    apply(&w.apply_args(&s)).unwrap();
    w.release("0.0.8", 50);
    let s = w.scan_args();
    assert_eq!(scan(&s).unwrap().entries, 3, "only the new tag");
    apply(&w.apply_args(&s)).unwrap();
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
    assert_eq!(ok(&o).trim(), "50 51");
    let lock = fs::read_to_string(c.join("Cargo.lock")).unwrap();
    for n in ["demo-base", "demo-lib", "demo-app"] {
        assert_eq!(
            lock.matches(&format!("name = \"{n}\"")).count(),
            1,
            "one source of {n}: {lock}"
        );
    }
    assert!(
        lock.contains("version = \"0.0.8\"") && lock.contains(&format!("registry+{}", w.url)),
        "{lock}"
    );
}

#[test]
fn a_changed_crate_file_is_rejected_by_cargo() {
    let mut w = World::new();
    w.release("0.0.7", 42);
    let s = w.scan_args();
    scan(&s).unwrap();
    apply(&w.apply_args(&s)).unwrap();
    // mutant: another valid crate under the same name and version, the checksum in the index stays
    w.write_crates("0.0.7", 13);
    ok(&run(
        &w.src,
        "cargo",
        &[
            "package",
            "--locked",
            "--no-verify",
            "--allow-dirty",
            "-p",
            "demo-base",
            "--target-dir",
            w.root.join("t-other").to_str().unwrap(),
        ],
        &[],
    ));
    fs::copy(
        w.root.join("t-other/package/demo-base-0.0.7.crate"),
        w.reg.join("crates/demo-base/demo-base-0.0.7.crate"),
    )
    .unwrap();
    let c = w.consumer("~0.0.7");
    let home = w.cargo_home();
    let o = run(&c, "cargo", &["fetch"], &[("CARGO_HOME", &home)]);
    assert!(!o.status.success(), "cargo must refuse the changed .crate");
    assert!(String::from_utf8_lossy(&o.stderr)
        .to_lowercase()
        .contains("checksum"));
    // control: the untouched world is accepted (otherwise the test would be green when the fetch never worked)
    let mut w2 = World::new();
    w2.release("0.0.7", 42);
    let s2 = w2.scan_args();
    scan(&s2).unwrap();
    apply(&w2.apply_args(&s2)).unwrap();
    let c2 = w2.consumer("~0.0.7");
    let home2 = w2.cargo_home();
    ok(&run(&c2, "cargo", &["fetch"], &[("CARGO_HOME", &home2)]));
}

#[test]
fn the_semver_gate_runs_before_the_entry_with_baseline_root_and_red_means_no_entry() {
    let mut w = World::new();
    w.release("0.0.7", 42);
    let s = w.scan_args();
    scan(&s).unwrap();
    apply(&w.apply_args(&s)).unwrap();
    w.release("0.0.8", 43);
    let fake = w.root.join("fake-semver.sh");
    let argsfile = w.root.join("semver-args.txt");
    fs::write(
        &fake,
        format!("#!/bin/sh\necho \"$@\" >> {}\nexit 0\n", argsfile.display()),
    )
    .unwrap();
    ok(&run(&w.root, "chmod", &["+x", fake.to_str().unwrap()], &[]));
    let mut s = w.scan_args();
    s.semver_checks = Some(fake.clone());
    let rep = scan(&s).unwrap();
    assert!(rep.error.is_none() && rep.entries == 3, "{rep:?}");
    let args = fs::read_to_string(&argsfile).unwrap();
    assert_eq!(args.lines().count(), 3, "once per crate: {args}");
    assert!(
        args.contains("--baseline-root")
            && args.contains("--release-type minor")
            && args.contains("-p demo-app"),
        "{args}"
    );
    assert!(
        args.contains("baseline-demo-app-0.0.7/demo-app-0.0.7"),
        "baseline = the .crate of the own registry checkout: {args}"
    );
    // MUTANT-like counter-test: the same gate but red -> nothing is planned, nothing entered
    fs::write(&fake, "#!/bin/sh\necho breaking change\nexit 1\n").unwrap();
    let head = w.head();
    let mut s = w.scan_args();
    s.semver_checks = Some(fake);
    let rep = scan(&s).unwrap();
    assert!(
        rep.error
            .as_deref()
            .is_some_and(|e| e.contains("semver gate RED")),
        "{rep:?}"
    );
    assert_eq!(rep.entries, 0);
    assert_eq!(fs::read_to_string(s.out.join("plan.txt")).unwrap(), "");
    assert_eq!(w.head(), head);
}
