//! Pattern: none (integration test: the guards; a wrong run writes nothing).

mod support;

use std::fs;

use fs_crate_index::{
    apply, check_dl, publish, scan, split_v2, version_of_ref, wait, PublishArgs, WaitArgs,
};
use support::{ok, run, World};

fn packaged(w: &mut World) -> fs_crate_index::ScanArgs {
    w.release("0.0.7", 42);
    let s = w.scan_args();
    let r = scan(&s).unwrap();
    assert!(r.error.is_none() && r.entries == 3, "{r:?}");
    s
}

fn pargs(w: &World, s: &fs_crate_index::ScanArgs, git_ref: &str, upload: &str) -> PublishArgs {
    PublishArgs {
        git_ref: git_ref.into(),
        crate_file: s.out.join("demo-base-0.0.7.crate"),
        registry_dir: w.reg.clone(),
        index_url: w.url.clone(),
        upload: upload.into(),
        source: String::new(),
    }
}

fn nothing_written(w: &World, before: &str) {
    assert_eq!(w.head(), before, "no commit");
    assert!(
        !w.reg.join("crates").exists() && !w.reg.join("de").exists(),
        "no file"
    );
    assert!(
        ok(&run(&w.reg, "git", &["status", "--porcelain"], &[]))
            .trim()
            .is_empty(),
        "clean"
    );
}

#[test]
fn ref_guard_accepts_only_a_plain_tag() {
    assert_eq!(version_of_ref("refs/tags/v0.0.7").unwrap(), "0.0.7");
    for bad in [
        "refs/heads/main",
        "refs/heads/v0.0.7",
        "refs/pull/3/merge",
        "",
        "v0.0.7",
        "refs/tags/0.0.7",
        "refs/tags/v0.0.7-rc1",
        "refs/tags/v0.0",
        "refs/tags/v0.0.07",
        "refs/tags/v1.2.3+b",
    ] {
        assert!(version_of_ref(bad).is_err(), "{bad} must be refused");
    }
}

#[test]
fn features_with_dep_or_question_slash_go_to_features2_with_v2() {
    let mut w = World::new();
    let s = packaged(&mut w);
    let mut p = reg_index::metadata_from_crate(&w.url, s.out.join("demo-app-0.0.7.crate")).unwrap();
    assert!(
        p.features.contains_key("conf"),
        "cargo reports the hidden/explicit dep: feature: {:?}",
        p.features
    );
    p.features.insert("plain".into(), vec!["conf".into()]);
    p.features.insert("weak".into(), vec!["demo-lib?/x".into()]);
    assert!(split_v2(&mut p));
    assert_eq!(p.v, Some(2));
    let f2 = p.features2.clone().unwrap();
    assert!(f2.contains_key("conf") && f2.contains_key("weak") && !f2.contains_key("plain"));
    assert!(p.features.contains_key("plain") && !p.features.contains_key("conf"));
    // a crate without such a value stays format v1
    let mut q =
        reg_index::metadata_from_crate(&w.url, s.out.join("demo-base-0.0.7.crate")).unwrap();
    assert!(!split_v2(&mut q) && q.v.is_none() && q.features2.is_none());
}

#[test]
fn a_branch_pr_or_wrong_tag_writes_nothing() {
    let mut w = World::new();
    let s = packaged(&mut w);
    let before = w.head();
    for r in [
        "refs/heads/main",
        "refs/pull/1/merge",
        "",
        "refs/tags/v0.0.9",
    ] {
        let e = publish(&pargs(&w, &s, r, "crates/{crate}")).unwrap_err();
        assert!(format!("{e:#}").contains("tag"), "{r}: {e:#}");
        nothing_written(&w, &before);
    }
}

#[test]
fn publish_with_a_dl_that_does_not_lead_to_the_stored_file_writes_nothing() {
    let mut w = World::new();
    let s = packaged(&mut w);
    let before = w.head();
    let e = publish(&pargs(&w, &s, "refs/tags/v0.0.7", "elsewhere/{crate}")).unwrap_err();
    assert!(format!("{e:#}").contains("dl"), "{e:#}");
    nothing_written(&w, &before);
    assert!(!w.reg.join("elsewhere").exists());
    assert!(check_dl(&w.reg, "crates/{crate}", "demo-base", "0.0.7").is_ok());
    let none = w.root.join("nothing");
    fs::create_dir_all(&none).unwrap();
    assert!(
        check_dl(&none, "crates/{crate}", "demo-base", "0.0.7").is_err(),
        "no config.json"
    );
}

#[test]
fn a_version_already_in_the_index_is_never_replaced() {
    let mut w = World::new();
    let s = packaged(&mut w);
    publish(&pargs(&w, &s, "refs/tags/v0.0.7", "crates/{crate}")).unwrap();
    let head = w.head();
    let before = fs::read(w.reg.join("crates/demo-base/demo-base-0.0.7.crate")).unwrap();
    assert!(publish(&pargs(&w, &s, "refs/tags/v0.0.7", "crates/{crate}")).is_err());
    assert_eq!(w.head(), head);
    assert_eq!(
        fs::read(w.reg.join("crates/demo-base/demo-base-0.0.7.crate")).unwrap(),
        before
    );
}

#[test]
fn a_repo_that_is_not_on_the_allow_list_is_not_scanned_and_not_applied() {
    let mut w = World::new();
    let s = packaged(&mut w);
    // apply: a plan line of a repo that is not allowed
    let plan = fs::read_to_string(s.out.join("plan.txt"))
        .unwrap()
        .replace("owner/demo", "evil/demo");
    fs::write(s.out.join("plan.txt"), plan).unwrap();
    let e = apply(&w.apply_args(&s)).unwrap_err();
    assert!(format!("{e:#}").contains("allow list"), "{e:#}");
    // scan: an empty allow list finds nothing
    fs::write(&w.allow, "# nobody\n").unwrap();
    let s2 = w.scan_args();
    assert_eq!(scan(&s2).unwrap().entries, 0);
}

#[test]
fn a_lightweight_tag_and_a_tag_off_main_are_red() {
    let mut w = World::new();
    ok(&run(&w.src, "git", &["tag", "v0.0.7"], &[])); // lightweight
    let s = w.scan_args();
    let r = scan(&s).unwrap();
    assert!(
        r.error
            .as_deref()
            .is_some_and(|e| e.contains("not annotated")),
        "{r:?}"
    );
    assert_eq!(r.entries, 0);
    // annotated, but on a side branch that is not on main
    let mut w = World::new();
    ok(&run(&w.src, "git", &["checkout", "-q", "-b", "side"], &[]));
    w.release("0.0.7", 42);
    ok(&run(&w.src, "git", &["checkout", "-q", "main"], &[]));
    let s = w.scan_args();
    let r = scan(&s).unwrap();
    assert!(
        r.error
            .as_deref()
            .is_some_and(|e| e.contains("not on main")),
        "{r:?}"
    );
    assert_eq!(r.entries, 0);
}

#[test]
fn a_tag_that_does_not_match_the_package_version_is_red() {
    let mut w = World::new();
    w.release("0.0.7", 42);
    ok(&run(
        &w.src,
        "git",
        &["tag", "-a", "-m", "wrong", "v0.0.9"],
        &[],
    ));
    // v0.0.7 would be fine, v0.0.9 holds 0.0.7 crates: red
    let s = w.scan_args();
    let r = scan(&s).unwrap();
    assert_eq!(r.entries, 3, "0.0.7 is planned first");
    let e = r.error.unwrap();
    assert!(e.contains("v0.0.9") && e.contains("does not match"), "{e}");
}

#[test]
fn a_tag_that_was_moved_after_its_entry_is_red_and_nothing_is_entered() {
    let mut w = World::new();
    let s = packaged(&mut w);
    apply(&w.apply_args(&s)).unwrap();
    w.write_crates("0.0.7", 99);
    w.commit("after");
    ok(&run(
        &w.src,
        "git",
        &["tag", "-f", "-a", "-m", "moved", "v0.0.7"],
        &[],
    ));
    let head = w.head();
    let r = scan(&w.scan_args()).unwrap();
    assert!(
        r.error.as_deref().is_some_and(|e| e.contains("MOVED")),
        "{r:?}"
    );
    assert_eq!(w.head(), head);
}

#[test]
fn a_tag_older_than_the_first_version_of_the_allow_list_is_ignored() {
    let mut w = World::new();
    w.release("0.0.6", 1);
    fs::write(&w.allow, "owner/demo 0.0.7 demo-base demo-lib demo-app\n").unwrap();
    let r = scan(&w.scan_args()).unwrap();
    assert!(r.error.is_none() && r.entries == 0, "{r:?}");
}

#[test]
fn wait_gives_a_clear_blocker_with_the_one_click() {
    let mut w = World::new();
    let s = packaged(&mut w);
    let args = |pkg: &str| WaitArgs {
        registry_url: w.url.clone(),
        package: pkg.into(),
        version: "0.0.7".into(),
        minutes: 0,
        interval_s: 1,
        workflow_page: "https://example.org/actions/workflows/index.yml".into(),
        work_dir: w.root.join(format!("wait-{pkg}")),
    };
    let a = args("demo-base");
    fs::create_dir_all(&a.work_dir).unwrap();
    let e = format!("{:#}", wait(&a).unwrap_err());
    assert!(
        e.contains("BLOCKED")
            && e.contains("Enable workflow")
            && e.contains("Run workflow")
            && e.contains("example.org"),
        "{e}"
    );
    apply(&w.apply_args(&s)).unwrap();
    let a = args("demo-app");
    fs::create_dir_all(&a.work_dir).unwrap();
    wait(&a).unwrap();
}

/// Repacks `demo-base-0.0.7.crate` of the scan output: the file name stays, the name in the inner `Cargo.toml` becomes `inner`.
fn relabel_inner_name(w: &World, s: &fs_crate_index::ScanArgs, inner: &str) {
    let file = s.out.join("demo-base-0.0.7.crate");
    let t = w.root.join("t-relabel");
    let _ = fs::remove_dir_all(&t);
    fs::create_dir_all(&t).unwrap();
    ok(&run(&t, "tar", &["xzf", file.to_str().unwrap()], &[]));
    let manifest = t.join("demo-base-0.0.7/Cargo.toml");
    let text = fs::read_to_string(&manifest).unwrap();
    assert!(text.contains("name = \"demo-base\""), "{text}");
    fs::write(
        &manifest,
        text.replace("name = \"demo-base\"", &format!("name = \"{inner}\"")),
    )
    .unwrap();
    ok(&run(
        &t,
        "tar",
        &["czf", file.to_str().unwrap(), "demo-base-0.0.7"],
        &[],
    ));
}

#[test]
fn a_crate_whose_inner_name_differs_from_the_plan_line_is_red_and_nothing_is_entered() {
    // job `package` is foreign code: the file name and the plan say `demo-base`, the `.crate` says another name
    for inner in ["not-on-any-list", "demo-lib"] {
        let mut w = World::new();
        let s = packaged(&mut w);
        relabel_inner_name(&w, &s, inner);
        let before = w.head();
        let e = format!("{:#}", apply(&w.apply_args(&s)).unwrap_err());
        assert!(
            e.contains(&format!("contains the crate `{inner}`")),
            "{inner}: {e}"
        );
        assert_eq!(w.head(), before, "{inner}: no commit");
        assert!(
            !w.reg.join("crates").exists() && !w.reg.join("no").exists(),
            "{inner}: no file"
        );
    }
}

#[test]
fn a_plan_line_whose_version_is_not_the_version_of_its_tag_is_red_and_nothing_is_entered() {
    let mut w = World::new();
    let s = packaged(&mut w);
    // the plan (a file of job `package`) says 0.0.8 for the tag v0.0.7; file name and plan are consistent with each other
    fs::copy(
        s.out.join("demo-base-0.0.7.crate"),
        s.out.join("demo-base-0.0.8.crate"),
    )
    .unwrap();
    let plan = fs::read_to_string(s.out.join("plan.txt")).unwrap();
    let mut f: Vec<String> = plan
        .lines()
        .next()
        .unwrap()
        .split('\t')
        .map(Into::into)
        .collect();
    f[5] = "0.0.8".into(); // version
    f[6] = "demo-base-0.0.8.crate".into(); // file
    let first = f.join("\t");
    let rest: Vec<&str> = plan.lines().skip(1).collect();
    fs::write(
        s.out.join("plan.txt"),
        format!("{first}\n{}\n", rest.join("\n")),
    )
    .unwrap();
    let before = w.head();
    let e = format!("{:#}", apply(&w.apply_args(&s)).unwrap_err());
    assert!(e.contains("is not version 0.0.8"), "{e}");
    assert_eq!(w.head(), before, "no commit");
}

#[test]
fn a_plan_line_with_another_file_name_than_package_and_version_is_red() {
    let mut w = World::new();
    let s = packaged(&mut w);
    // a valid crate under a name the plan line must not use (also a path that leaves the hand-over directory)
    fs::copy(
        s.out.join("demo-base-0.0.7.crate"),
        w.root.join("escape.crate"),
    )
    .unwrap();
    let plan = fs::read_to_string(s.out.join("plan.txt")).unwrap();
    let mut f: Vec<String> = plan
        .lines()
        .next()
        .unwrap()
        .split('\t')
        .map(Into::into)
        .collect();
    f[6] = "../escape.crate".into();
    let rest: Vec<&str> = plan.lines().skip(1).collect();
    fs::write(
        s.out.join("plan.txt"),
        format!("{}\n{}\n", f.join("\t"), rest.join("\n")),
    )
    .unwrap();
    let before = w.head();
    let e = format!("{:#}", apply(&w.apply_args(&s)).unwrap_err());
    assert!(e.contains("unexpected file name"), "{e}");
    assert_eq!(w.head(), before, "no commit");
}
