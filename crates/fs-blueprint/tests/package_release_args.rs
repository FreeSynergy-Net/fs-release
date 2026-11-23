//! Review S3/S4: the Pages address and the public key of `freesynergy-release` are checked before anything goes into the package.
use fs_blueprint::package_rpm::{
    check_fingerprint, checked_key, checked_pages_url, release_spec, repo_file, KEY_PATH,
};
use std::path::Path;

const FP: &str = "16A4A3EE563AEF51FCF0C55DED7B57C4A1904342";

fn key() -> String {
    let p = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../release-signing-key.asc");
    std::fs::read_to_string(p).unwrap()
}

#[test]
fn a_good_https_address_is_taken_and_loses_its_trailing_slash() {
    assert_eq!(
        checked_pages_url("https://example.org/Repository/").unwrap(),
        "https://example.org/Repository"
    );
}

#[test]
fn bad_addresses_are_refused() {
    for bad in [
        "",
        "PAGES-URL-NOT-SET",
        "http://example.org/x",
        "ftp://example.org/x",
        "example.org/x",
        "https://",
        "https://example.org/x\nenabled=0",
        "https://example.org/x\r\ngpgcheck=0",
        "https://example.org/ x",
        "https://example.org/x\ty",
        "https://user:pw@example.org/x",
        "https://user@example.org/x",
        "https://example.org/x?a=b",
        "https://example.org/x#frag",
        "https://exa\u{0}mple.org/x",
    ] {
        assert!(checked_pages_url(bad).is_err(), "{bad:?} must be refused");
    }
}

#[test]
fn no_line_break_can_get_into_the_repo_file() {
    let spec = release_spec(
        "https://example.org/x\nenabled=0",
        key().as_bytes(),
        FP,
        "1",
        "1",
    );
    assert!(spec.is_err());
    let url = checked_pages_url("https://example.org/x").unwrap();
    let n = repo_file(&url)
        .lines()
        .filter(|l| l.starts_with("baseurl="))
        .count();
    assert_eq!(n, 1);
}

#[test]
fn the_repo_file_has_exactly_one_section_and_nothing_switched_off() {
    let repo = repo_file("https://pages.invalid/Repository");
    let sections: Vec<&str> = repo.lines().filter(|l| l.starts_with('[')).collect();
    assert_eq!(sections, ["[freesynergy]"], "{repo}");
    assert!(
        !repo.contains("enabled=0") && !repo.contains("nightly"),
        "{repo}"
    );
    for line in ["enabled=1", "gpgcheck=1", "repo_gpgcheck=1"] {
        assert_eq!(
            repo.lines().filter(|l| *l == line).count(),
            1,
            "{line}: {repo}"
        );
    }
    assert!(!repo.contains("skip_if_unavailable"), "{repo}");
}

#[test]
fn the_expected_fingerprint_is_compared_with_the_key() {
    assert!(check_fingerprint(&key(), FP).is_ok());
    assert!(check_fingerprint(&key(), &FP.to_lowercase()).is_ok());
    let other = "0000000000000000000000000000000000000000";
    assert!(check_fingerprint(&key(), other)
        .unwrap_err()
        .contains("not the expected"));
}

#[test]
fn a_malformed_fingerprint_or_a_non_key_is_refused() {
    for bad in [
        "",
        "16A4",
        "16A4 A3EE 563A EF51 FCF0 C55D ED7B 57C4 A190 4342",
        "ZZ",
    ] {
        assert!(check_fingerprint(&key(), bad).is_err(), "{bad:?}");
    }
    assert!(check_fingerprint("not a key", FP).is_err());
    assert!(check_fingerprint(
        "-----BEGIN PGP PUBLIC KEY BLOCK-----\n\nAAAA\n-----END PGP PUBLIC KEY BLOCK-----\n",
        FP
    )
    .is_err());
}

#[test]
fn a_release_package_needs_every_check() {
    let ok = release_spec("https://example.org/x", key().as_bytes(), FP, "1.0.0", "1");
    assert!(ok.is_ok());
    let wrong_fp = release_spec(
        "https://example.org/x",
        key().as_bytes(),
        &"A".repeat(40),
        "1.0.0",
        "1",
    );
    assert!(wrong_fp.is_err());
    let secret = release_spec(
        "https://example.org/x",
        b"-----BEGIN PGP PRIVATE KEY BLOCK-----\nx\n",
        FP,
        "1.0.0",
        "1",
    );
    assert!(secret.is_err());
}

// Review round 2 (M1, M2): only ONE checked public key may travel into the package, and it is written anew.
const PROBE_FP: &str = "93221421976AA9D02CC46D42A2F6E14508F844D7";

fn probe(name: &str) -> String {
    let p = Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/data/release-key");
    std::fs::read_to_string(p.join(name)).unwrap()
}

/// The throw-away secret key of the test is stored without its armor lines (so no scanner takes it for a leak).
fn probe_secret() -> String {
    probe("probe-two-secret.txt")
        .replace("@@BEGIN@@", "-----BEGIN PGP PRIVATE KEY BLOCK-----")
        .replace("@@END@@", "-----END PGP PRIVATE KEY BLOCK-----")
}

#[test]
fn the_probe_key_alone_is_taken() {
    let out = checked_key(probe("probe-one.asc").as_bytes(), PROBE_FP).unwrap();
    let text = String::from_utf8(out).unwrap();
    assert_eq!(text.matches("-----BEGIN PGP").count(), 1);
}

#[test]
fn a_second_key_block_appended_is_refused() {
    let both = format!("{}\n{}", probe("probe-one.asc"), probe("probe-two.asc"));
    let e = checked_key(both.as_bytes(), PROBE_FP).unwrap_err();
    assert!(e.contains("exactly one armored block"), "{e}");
    assert!(release_spec("https://example.org/x", both.as_bytes(), PROBE_FP, "1", "1").is_err());
}

#[test]
fn two_primary_keys_in_one_block_are_refused() {
    let both = probe("probe-both-in-one-block.asc");
    assert_eq!(both.matches("-----BEGIN PGP").count(), 1);
    let e = checked_key(both.as_bytes(), PROBE_FP).unwrap_err();
    assert!(e.contains("more than one primary key"), "{e}");
    assert!(release_spec("https://example.org/x", both.as_bytes(), PROBE_FP, "1", "1").is_err());
}

#[test]
fn a_secret_key_appended_to_a_public_key_is_refused() {
    let both = format!("{}\n{}", probe("probe-one.asc"), probe_secret());
    let e = checked_key(both.as_bytes(), PROBE_FP).unwrap_err();
    assert!(e.contains("private key"), "{e}");
    assert!(release_spec("https://example.org/x", both.as_bytes(), PROBE_FP, "1", "1").is_err());
}

#[test]
fn the_package_holds_the_key_written_anew_without_trailing_text() {
    let with_tail = format!("{}\nsome text after the key\n", probe("probe-one.asc"));
    let spec = release_spec(
        "https://example.org/x",
        with_tail.as_bytes(),
        PROBE_FP,
        "1",
        "1",
    )
    .unwrap();
    let file = spec.files.iter().find(|f| f.dest == KEY_PATH).unwrap();
    let text = String::from_utf8(file.bytes.clone().unwrap()).unwrap();
    assert!(!text.contains("some text after"));
    assert_eq!(text.matches("-----BEGIN PGP").count(), 1);
    assert!(check_fingerprint(&text, PROBE_FP).is_ok());
}
