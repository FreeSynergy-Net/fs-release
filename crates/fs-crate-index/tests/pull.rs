//! Pattern: none (integration test, r3 review of K-2: `scan` without the semver binary is RED (never NOT RUN = green), a tag moved between the two jobs is RED,
//! ADR 0093 "three cases": 1 = changed `.crate` (tests/registry.rs), 2 = index and `.crate` changed together with an old lock, 3 = fresh consumer without lock).

mod support;

use std::fs;
use std::path::Path;

use fs_crate_index::{apply, main_with, scan};
use support::{ok, run, World};

fn entered(w: &mut World, version: &str, answer: u32) {
    w.release(version, answer);
    let s = w.scan_args();
    assert_eq!(scan(&s).unwrap().entries, 3);
    apply(&w.apply_args(&s)).unwrap();
}

#[test]
fn scan_without_the_semver_binary_is_red_when_a_baseline_exists_and_plans_nothing() {
    let mut w = World::new();
    entered(&mut w, "0.0.7", 42);
    w.release("0.0.8", 43);
    let head = w.head();
    let mut s = w.scan_args();
    s.semver_checks = None; // the workflow lost `--semver-checks`
    let rep = scan(&s).unwrap();
    assert!(
        rep.error.as_deref().is_some_and(|e| e.contains("NOT RUN")),
        "NOT RUN is never green: {rep:?}"
    );
    assert_eq!(rep.entries, 0);
    assert_eq!(fs::read_to_string(s.out.join("plan.txt")).unwrap(), "");
    assert_eq!(w.head(), head);
}

#[test]
fn the_command_line_of_scan_does_not_start_without_semver_checks() {
    let w = World::new();
    let a = |extra: &[&str]| {
        let mut v: Vec<String> = ["fs-crate-index", "scan", "--allow"]
            .iter()
            .map(ToString::to_string)
            .collect();
        v.push(w.allow.to_string_lossy().into_owned());
        for (k, d) in [
            ("--registry-dir", &w.reg),
            ("--work-dir", &w.root.join("cw")),
            ("--out", &w.root.join("co")),
        ] {
            v.push(k.into());
            v.push(d.to_string_lossy().into_owned());
        }
        v.push("--source-base".into());
        v.push(format!("file://{}", w.root.display()));
        v.extend(extra.iter().map(ToString::to_string));
        v
    };
    assert_eq!(
        main_with(a(&[])),
        2,
        "missing --semver-checks is a usage error"
    );
    assert!(!w.root.join("cw").exists(), "nothing was started");
    assert_eq!(
        main_with(a(&["--semver-checks", w.ok_semver().to_str().unwrap()])),
        0
    );
}

#[test]
fn a_tag_that_moves_between_the_two_jobs_is_red_and_nothing_is_entered() {
    let mut w = World::new();
    w.release("0.0.7", 42);
    let s = w.scan_args();
    assert_eq!(scan(&s).unwrap().entries, 3);
    // between job 1 and job 2 somebody sets the tag to another commit
    w.write_crates("0.0.7", 99);
    w.commit("other");
    ok(&run(
        &w.src,
        "git",
        &["tag", "-f", "-a", "-m", "moved", "v0.0.7"],
        &[],
    ));
    let head = w.head();
    let e = format!("{:#}", apply(&w.apply_args(&s)).unwrap_err());
    assert!(e.contains("moved between the jobs"), "{e}");
    assert_eq!(w.head(), head, "nothing entered");
}

/// Swaps `demo-base` 0.0.7 in the registry: another valid `.crate` and the matching checksum in the index line, committed (index and file together).
fn swap_crate_and_index(w: &World) {
    w.write_crates("0.0.7", 13);
    let t = w.root.join("t-swap");
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
            t.to_str().unwrap(),
        ],
        &[],
    ));
    let krate = w.reg.join("crates/demo-base/demo-base-0.0.7.crate");
    fs::copy(t.join("package/demo-base-0.0.7.crate"), &krate).unwrap();
    let sum = |p: &Path| {
        ok(&run(&w.root, "sha256sum", &[p.to_str().unwrap()], &[]))
            .split_whitespace()
            .next()
            .unwrap()
            .to_string()
    };
    let new = sum(&krate);
    let idx = w.reg.join("de/mo/demo-base");
    let text = fs::read_to_string(&idx).unwrap();
    let old = text
        .split("\"cksum\":\"")
        .nth(1)
        .unwrap()
        .split('"')
        .next()
        .unwrap()
        .to_string();
    assert_ne!(old, new, "the swap changed the file");
    fs::write(&idx, text.replace(&old, &new)).unwrap();
    ok(&run(&w.reg, "git", &["add", "-A"], &[]));
    ok(&run(&w.reg, "git", &["commit", "-q", "-m", "swapped"], &[]));
}

#[test]
fn case_2_index_and_crate_swapped_together_with_an_old_lock_is_red_and_case_3_fresh_consumer_accepts_it(
) {
    let mut w = World::new();
    entered(&mut w, "0.0.7", 42);
    let c = w.consumer("~0.0.7");
    let home1 = w.cargo_home();
    ok(&run(&c, "cargo", &["fetch"], &[("CARGO_HOME", &home1)])); // writes Cargo.lock with the checksum
    assert!(fs::read_to_string(c.join("Cargo.lock"))
        .unwrap()
        .contains("checksum"));
    swap_crate_and_index(&w);
    // case 2: old lock, a fresh cargo home (no local cache that would hide the swap)
    let home2 = w.root.join("cargo-home-2");
    let o = run(
        &c,
        "cargo",
        &["fetch", "--locked"],
        &[("CARGO_HOME", &home2)],
    );
    assert!(
        !o.status.success(),
        "the old lock must refuse the swapped crate"
    );
    assert!(
        String::from_utf8_lossy(&o.stderr)
            .to_lowercase()
            .contains("checksum"),
        "{}",
        String::from_utf8_lossy(&o.stderr)
    );
    // case 3: a fresh consumer without a lock takes the swapped version (this is the core of the model: the lock protects, the registry repo is the trust anchor)
    let c3 = w.consumer("^0.0.7");
    let home3 = w.root.join("cargo-home-3");
    assert!(!c3.join("Cargo.lock").exists());
    ok(&run(&c3, "cargo", &["fetch"], &[("CARGO_HOME", &home3)]));
}
