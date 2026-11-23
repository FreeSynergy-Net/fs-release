//! Pattern: Guard (checks the arguments of the package `freesynergy-release` before anything is put into a package; the package itself is `package_rpm`'s job).

use pgp::composed::{Deserializable, SignedPublicKey};
use pgp::types::KeyDetails;

use crate::package_rpm::PAGES_URL;

/// The Pages address for the `.repo` file: `https` only, one parsable URL with a host, no user info, no query, no fragment, and no white space or control
/// character anywhere (a line break would put a new line into the `.repo` file). The placeholder is refused. Returned without a trailing slash.
///
/// # Errors
/// A text naming what is wrong.
pub fn checked_pages_url(pages: &str) -> Result<String, String> {
    if pages == PAGES_URL || pages.is_empty() {
        return Err("the Pages address is not set: give --pages-url (it is not known yet)".into());
    }
    if pages.chars().any(|c| c.is_whitespace() || c.is_control()) {
        return Err("the Pages address holds white space or a control character".into());
    }
    let u = url::Url::parse(pages).map_err(|e| format!("the Pages address is not a URL: {e}"))?;
    if u.scheme() != "https" {
        return Err("the Pages address must start with https://".into());
    }
    if u.host_str().is_none_or(str::is_empty)
        || !u.username().is_empty()
        || u.password().is_some()
        || u.query().is_some()
        || u.fragment().is_some()
    {
        return Err(
            "the Pages address must be https://host/path (no user info, query or fragment)".into(),
        );
    }
    Ok(pages.trim_end_matches('/').to_string())
}

/// The key file must hold exactly ONE armored `OpenPGP` PUBLIC key with ONE primary key (crate `pgp`), its primary fingerprint must be the expected one (40 hex digits,
/// given by the caller, never fixed in code) and its bindings must verify. Returns the bytes for the package: the checked key serialized anew, never the raw input
/// (so nothing appended to the file, no second key and no secret key can travel along).
///
/// # Errors
/// A text when the fingerprint option is malformed, the file is not exactly one public key, or the fingerprints differ.
pub fn checked_key(input: &[u8], expected: &str) -> Result<Vec<u8>, String> {
    let want = expected.to_ascii_uppercase();
    if want.len() != 40 || !want.chars().all(|c| c.is_ascii_hexdigit()) {
        return Err("--fingerprint must be 40 hex digits (no spaces)".into());
    }
    let armor = std::str::from_utf8(input).map_err(|_| "the key is not armored text")?;
    if armor.contains("PRIVATE KEY") {
        return Err("the key file holds a private key: refused".into());
    }
    if armor.matches("-----BEGIN PGP").count() != 1 {
        return Err("the key file must hold exactly one armored block".into());
    }
    let bad = |e: String| format!("the key file is not an armored PGP public key: {e}");
    let (mut keys, _) = SignedPublicKey::from_armor_many(input).map_err(|e| bad(e.to_string()))?;
    let key = keys
        .next()
        .ok_or_else(|| bad("no key".into()))?
        .map_err(|e| bad(e.to_string()))?;
    if keys.next().is_some() {
        return Err("the key file holds more than one primary key: refused".into());
    }
    let have = format!("{:X}", key.fingerprint());
    if have != want {
        return Err(format!(
            "the fingerprint of the key is {have}, not the expected one"
        ));
    }
    key.verify_bindings()
        .map_err(|e| format!("the key bindings do not verify: {e}"))?;
    key.to_armored_bytes(None.into())
        .map_err(|e| format!("the key cannot be written again: {e}"))
}

/// Checks a key file (see `checked_key`) and throws the bytes away.
///
/// # Errors
/// As `checked_key`.
pub fn check_fingerprint(armor: &str, expected: &str) -> Result<(), String> {
    checked_key(armor.as_bytes(), expected).map(|_| ())
}
