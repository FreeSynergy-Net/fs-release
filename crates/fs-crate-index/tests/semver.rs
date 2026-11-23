//! Pattern: none (integration test: the ONE semver gate, three cases: no version = green and said so, a version = compared, registry not reachable = RED with its own text).

mod support;

use std::fs;
use std::path::PathBuf;

use fs_crate_index::{apply, gate, scan, Outcome, SemverArgs};
use support::{ok, run, World};

fn fake(w: &World, exit: u8) -> (PathBuf, PathBuf) {
    let bin = w.root.join(format!("fake-semver-{exit}.sh"));
    let log = w.root.join(format!("semver-args-{exit}.txt"));
    fs::write(
        &bin,
        format!("#!/bin/sh\necho \"$@\" >> {}\nexit {exit}\n", log.display()),
    )
    .unwrap();
    ok(&run(&w.root, "chmod", &["+x", bin.to_str().unwrap()], &[]));
    (bin, log)
}

fn args(w: &mut World, registry: &str, bin: Option<PathBuf>) -> SemverArgs {
    w.n += 1;
    SemverArgs {
        registry: registry.into(),
        package: "demo-base".into(),
        source: w.src.clone(),
        version: None,
        semver_checks: bin,
        work_dir: w.root.join(format!("semver-work-{}", w.n)),
    }
}

fn entered(w: &mut World) {
    w.release("0.0.7", 42);
    let s = w.scan_args();
    assert_eq!(scan(&s).unwrap().entries, 3);
    apply(&w.apply_args(&s)).unwrap();
}

#[test]
fn case_1_registry_without_the_crate_is_green_and_says_no_baseline() {
    let mut w = World::new();
    let (bin, log) = fake(&w, 0);
    let reg = w.reg.to_string_lossy().into_owned();
    let o = gate(&args(&mut w, &reg, Some(bin))).unwrap();
    assert_eq!(o, Outcome::NoBaseline);
    assert!(o
        .line("demo-base")
        .contains("no baseline: first registry version"));
    assert!(
        !log.exists(),
        "cargo-semver-checks is not called without a baseline"
    );
}

#[test]
fn case_2_registry_with_a_version_compares_with_the_newest_one_by_baseline_root_not_a_tag() {
    let mut w = World::new();
    entered(&mut w);
    let (bin, log) = fake(&w, 0);
    // by directory and by clone URL (the early signal of a publisher repo has only the URL)
    for reg in [w.reg.to_string_lossy().into_owned(), w.url.clone()] {
        let o = gate(&args(&mut w, &reg, Some(bin.clone()))).unwrap();
        assert_eq!(o, Outcome::Compared("0.0.7".parse().unwrap()), "{reg}");
    }
    let text = fs::read_to_string(&log).unwrap();
    assert_eq!(text.lines().count(), 2);
    assert!(
        text.contains("--release-type minor")
            && text.contains("--baseline-root")
            && text.contains("-p demo-base"),
        "{text}"
    );
    assert!(
        text.contains("baseline-demo-base-0.0.7/demo-base-0.0.7"),
        "{text}"
    );
    assert!(!text.contains("--baseline-rev"), "never a git tag: {text}");
    // MUTANT-like counter-test: the same call with a red semver-checks is RED
    let (red, _) = fake(&w, 1);
    let dir = w.reg.to_string_lossy().into_owned();
    let e = gate(&args(&mut w, &dir, Some(red))).unwrap_err();
    assert!(format!("{e:#}").contains("semver gate RED"), "{e:#}");
    // a baseline without a binary is NOT RUN, never green
    let o = gate(&args(&mut w, &dir, None)).unwrap();
    assert!(matches!(o, Outcome::NotRun(_)) && o.line("demo-base").contains("NOT RUN"));
}

#[test]
fn case_3_registry_not_reachable_is_red_with_its_own_text() {
    let mut w = World::new();
    let (bin, log) = fake(&w, 0);
    for reg in [
        format!("file://{}/missing-registry", w.root.display()),
        w.root.join("not-a-registry").to_string_lossy().into_owned(),
    ] {
        fs::create_dir_all(w.root.join("not-a-registry")).unwrap();
        let e = format!(
            "{:#}",
            gate(&args(&mut w, &reg, Some(bin.clone()))).unwrap_err()
        );
        assert!(
            e.contains("NOT REACHABLE") || e.contains("NOT READABLE"),
            "{e}"
        );
        assert!(
            !e.contains("first registry version") && !e.contains("baseline:"),
            "never confused with case 1: {e}"
        );
    }
    assert!(!log.exists());
}

#[test]
fn a_checked_branch_cannot_switch_a_lint_off_in_its_manifest() {
    let mut w = World::new();
    entered(&mut w);
    let (bin, log) = fake(&w, 0);
    let reg = w.reg.to_string_lossy().into_owned();
    let manifest = w.src.join("demo-base/Cargo.toml");
    let orig = fs::read_to_string(&manifest).unwrap();
    // allowed: nothing, or exactly `enum_variant_added = "warn"`
    assert!(gate(&args(&mut w, &reg, Some(bin.clone()))).is_ok());
    let with =
        |lints: &str| format!("{orig}\n[package.metadata.cargo-semver-checks.lints]\n{lints}\n");
    fs::write(&manifest, with("enum_variant_added = \"warn\"")).unwrap();
    assert!(
        gate(&args(&mut w, &reg, Some(bin.clone()))).is_ok(),
        "the one allowed deviation"
    );
    // MUTANT: a lint set to allow (any lint, also a table form), a warn for another lint, a workspace-level table: RED, cargo-semver-checks is not even called
    let n = fs::read_to_string(&log).unwrap().lines().count();
    for bad in [
        with("enum_variant_missing = \"allow\""),
        with("enum_variant_added = \"allow\""),
        with("enum_missing = \"warn\""),
        with("function_missing = { level = \"allow\" }"),
    ] {
        fs::write(&manifest, bad).unwrap();
        let e = format!(
            "{:#}",
            gate(&args(&mut w, &reg, Some(bin.clone()))).unwrap_err()
        );
        assert!(e.contains("changes the gate"), "{e}");
    }
    fs::write(&manifest, &orig).unwrap();
    fs::write(
        w.src.join("Cargo.toml"),
        format!(
            "{}\n[workspace.metadata.cargo-semver-checks.lints]\nenum_missing = \"allow\"\n",
            fs::read_to_string(w.src.join("Cargo.toml")).unwrap()
        ),
    )
    .unwrap();
    assert!(
        gate(&args(&mut w, &reg, Some(bin))).is_err(),
        "workspace table too"
    );
    assert_eq!(
        fs::read_to_string(&log).unwrap().lines().count(),
        n,
        "no call after a red manifest"
    );
}
