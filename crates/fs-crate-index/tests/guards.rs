//! Pattern: none (integration test: the guards; a branch or wrong-tag run writes nothing).

mod support;

use std::fs;

use fs_crate_index::{check_dl, check_features, publish, version_of_ref};
use support::{ok, run, World};

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
fn a_branch_or_pr_run_or_a_wrong_tag_writes_nothing() {
    let mut w = World::new();
    let before = w.head();
    for r in [
        "refs/heads/main",
        "refs/pull/1/merge",
        "",
        "refs/tags/v0.0.9",
    ] {
        let e = publish(&w.args(r)).unwrap_err();
        assert!(!format!("{e:#}").is_empty());
        assert_eq!(w.head(), before, "{r}: no commit");
        assert!(
            !w.reg.join("crates").exists() && !w.reg.join("de").exists(),
            "{r}: no file"
        );
        assert!(
            ok(&run(&w.reg, "git", &["status", "--porcelain"], &[]))
                .trim()
                .is_empty(),
            "{r}: clean"
        );
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
    assert_eq!(
        fs::read(w.reg.join("crates/demo-lib/demo-lib-0.0.7.crate")).unwrap(),
        before
    );
}

#[test]
fn dl_that_does_not_lead_to_the_stored_file_is_refused() {
    let w = World::new();
    assert!(check_dl(&w.reg, "crates/{crate}", "demo-lib", "0.0.7").is_ok());
    assert!(check_dl(&w.reg, "elsewhere/{crate}", "demo-lib", "0.0.7").is_err());
    let none = w.root.join("nothing");
    fs::create_dir_all(&none).unwrap();
    assert!(
        check_dl(&none, "crates/{crate}", "demo-lib", "0.0.7").is_err(),
        "no config.json"
    );
}
