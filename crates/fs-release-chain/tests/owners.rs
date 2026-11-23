//! Test of the owner boundary: only the two known owners are ours (old and new, exactly), never "any owner".

mod support;

use fs_release_chain::meta::Reach;
use fs_release_chain::owner::{is_ours, is_release_repo, same_repo};
use fs_release_chain::preapproved::Preapproved;
use fs_release_chain::Outcome;
use support::flow::all_green;
use support::{git, World};

const NEW: &str = "FreeSynergy-Net";
const OLD: &str = "FreeSynergy";

#[test]
fn only_the_two_known_owners_are_ours() {
    for ok in ["FreeSynergy/fs-x", "FreeSynergy-Net/fs-x"] {
        assert!(is_ours(ok), "{ok}");
    }
    for bad in [
        "Evil/fs-x",
        "freesynergy/fs-x",
        "FreeSynergy-Netx/fs-x",
        "FreeSynergyX/fs-x",
        "FreeSynergy-Net-evil/fs-x",
        "FreeSynergy/",
        "FreeSynergy",
        "",
    ] {
        assert!(!is_ours(bad), "{bad:?}");
    }
}

#[test]
fn the_release_repo_is_fs_release_of_a_known_owner_only() {
    assert!(is_release_repo("FreeSynergy/fs-release"));
    assert!(is_release_repo("FreeSynergy-Net/fs-release"));
    for bad in [
        "Evil/fs-release",
        "FreeSynergy-Net/fs-release-evil",
        "FreeSynergy/fs-tasks",
    ] {
        assert!(!is_release_repo(bad), "{bad}");
    }
}

#[test]
fn the_same_repo_under_either_known_owner_is_the_same_repo() {
    assert!(same_repo("FreeSynergy/fs-x", "FreeSynergy-Net/fs-x"));
    assert!(same_repo("FreeSynergy-Net/fs-x", "FreeSynergy/fs-x"));
    assert!(same_repo("Org/fs-demo", "Org/fs-demo"));
    assert!(!same_repo("Evil/fs-x", "FreeSynergy-Net/fs-x"));
    assert!(!same_repo("FreeSynergy/fs-x", "Evil/fs-x"));
    assert!(!same_repo("FreeSynergy/fs-x", "FreeSynergy-Net/fs-y"));
}

fn list(repos: &[&str]) -> Preapproved {
    Preapproved {
        repos: repos.iter().map(ToString::to_string).collect(),
        catalog_targets: vec![format!("{NEW}/Catalog")],
        public_ok: "ref".into(),
    }
}

#[test]
fn the_list_holds_for_the_repo_under_either_known_owner_but_not_for_a_third_owner() {
    let p = list(&[&format!("{NEW}/fs-tasks")]);
    for slug in [format!("{NEW}/fs-tasks"), format!("{OLD}/fs-tasks")] {
        assert!(p.allows(&slug, &format!("{OLD}/Catalog"), Reach::Public).is_ok(), "{slug}");
    }
    for slug in ["Evil/fs-tasks", "FreeSynergy-Netx/fs-tasks"] {
        assert!(p.allows(slug, &format!("{NEW}/Catalog"), Reach::Shared).is_err(), "{slug}");
    }
    assert!(p.allows(&format!("{NEW}/fs-tasks"), "Evil/Catalog", Reach::Public).is_err());
}

#[test]
fn a_release_dir_whose_origin_is_fs_release_of_the_new_owner_is_accepted() {
    let w = World::new();
    let sha = w.head();
    w.approve(&sha, "JA");
    all_green(&w, &sha);
    let rel = w.root().join("rel");
    let bare = w.root().join("rel-origin.git");
    let url = format!("https://github.com/{NEW}/fs-release.git");
    git(&rel, &["config", "remote.origin.url", &url]);
    git(&rel, &["config", &format!("url.{}.insteadOf", bare.display()), &url]);
    let o = w.run_names(Reach::Shared, &["review".to_string()]);
    assert!(
        !matches!(&o, Outcome::Red { line, .. } if line.contains("not fs-release")),
        "the new owner's fs-release must be accepted: {}",
        o.line()
    );
}
