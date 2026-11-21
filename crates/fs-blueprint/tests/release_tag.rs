//! B2 (report of 21 Nov.): the release tag of the generated CI is the tag of THIS crate version, so the generator that is released is the one the CI installs.
use fs_blueprint::emit::skeleton::{of, RELEASE_TAG};

#[test]
fn the_release_tag_is_v_plus_the_crate_version() {
    // fails the moment the crate version is raised for a release and the constant is forgotten (and the other way round); gold files follow the constant
    assert_eq!(
        RELEASE_TAG,
        format!("v{}", env!("CARGO_PKG_VERSION")),
        "skeleton::RELEASE_TAG must be set to v<crate version> at the release (and the gold files regenerated)"
    );
}

#[test]
fn the_skeleton_table_hands_out_that_one_tag() {
    assert_eq!(of("0.1.0").map(|s| s.release_tag), Some(RELEASE_TAG));
}

#[test]
fn there_is_no_pin_for_a_building_block_nobody_uses() {
    // B5: the pin of fs-rpc was dead (no template used it): the slot is gone, so a template that names it is refused
    assert!(fs_blueprint::emit::slot::Slot::parse("pin_rpc").is_none());
}
