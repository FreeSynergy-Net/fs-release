//! Review S3/S4: the Pages address and the public key of `freesynergy-release` are checked before anything goes into the package.
use fs_blueprint::package_rpm::{check_fingerprint, checked_pages_url, release_spec, repo_file};
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
    assert!(
        repo_file(&url)
            .lines()
            .filter(|l| l.starts_with("baseurl="))
            .count()
            == 2
    );
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
