//! The `Coded` of this crate mirrors the form of fs-types `coded`; equality is enforced against the checked-in reference file (no dependency).
use fs_blueprint::Coded;

const REF: &str = include_str!("data/fs-types-coded.ref");

fn code_of(name: &str) -> &'static str {
    fs_blueprint::CODES
        .iter()
        .map(|(c, _)| *c)
        .find(|c| *c == name)
        .unwrap_or_else(|| panic!("the reference names a code that does not exist: {name}"))
}

fn leak(s: &str) -> &'static str {
    Box::leak(s.to_string().into_boxed_str())
}

#[test]
fn the_form_of_every_reference_case_is_byte_equal() {
    let mut cases = 0;
    for line in REF.lines().filter(|l| !l.starts_with('#') && !l.is_empty()) {
        let f: Vec<&str> = line.split('|').map(str::trim).collect();
        assert_eq!(f.len(), 4, "{line}");
        let mut c = Coded::new(code_of(f[0]));
        for p in f[1].split(';').filter(|p| !p.is_empty()) {
            let (k, v) = p.split_once('=').unwrap();
            c = c.with(leak(k), v);
        }
        assert_eq!(c.message_key(), f[2], "{line}");
        assert_eq!(c.log_text(), f[3], "{line}");
        cases += 1;
    }
    assert!(cases >= 6);
}

#[test]
fn the_reference_names_its_origin() {
    assert!(REF.contains("v0.0.4") && REF.contains("0dbc9b685746ea55ceb789fe1043a162e34ac387"));
    assert!(REF.contains("0d2cf9b486e82ed3356a62635327737354e15445854eee101f1ec3dea42c39a9"));
}
