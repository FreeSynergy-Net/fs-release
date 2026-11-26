#![cfg(target_os = "linux")]
//! Pattern: none (test). Versions in `Requires` and the form source level: a module may say `at_least` per required module and `form_source_level`
//! (`Requires: freesynergy-form-source >= <level>`); a host program says `contract.form_source_level` (`Provides: freesynergy-form-source = <level>`).
//! Both are read back with the system `rpm -qp`. Missing fields change nothing (additive).
mod common;
use common::{cli, have_rpm, rpm, scratch};
use fs_blueprint::package_rpm::program_spec;
use fs_blueprint::Blueprint;
use std::path::{Path, PathBuf};

fn module(tag: &str, commands: &str) -> (PathBuf, PathBuf) {
    let d = scratch(tag);
    std::fs::create_dir_all(d.join("catalog")).unwrap();
    for (f, t) in [
        ("catalog/commands.json", commands),
        ("catalog/forms.json", "{}"),
        ("metainfo.xml", "<component/>"),
        ("LICENSE-MIT", "MIT"),
        ("LICENSE-APACHE", "Apache"),
        ("service", "\u{7f}ELF"),
    ] {
        std::fs::write(d.join(f), t).unwrap();
    }
    let bin = d.join("service");
    (d, bin)
}

fn make(out: &Path, d: &Path, bin: &Path) -> (i32, String) {
    let (code, o, e) = cli(&[
        "package",
        "module",
        d.to_str().unwrap(),
        "--name",
        "planning",
        "--pkg-version",
        "0.0.4",
        "--binary",
        bin.to_str().unwrap(),
        "--arch",
        "x86_64",
        "--out",
        out.to_str().unwrap(),
    ]);
    (code, format!("{o}{e}"))
}

const RPM: &str = "fs-module-planning-0.0.4-1.x86_64.rpm";

#[test]
fn at_least_and_the_form_source_level_become_versioned_requires() {
    if !have_rpm() {
        return;
    }
    let (d, bin) = module(
        "rv-ok",
        r#"{"form_source_level":2,"requires":[{"module":"person","required":true,"at_least":"0.0.8"},{"module":"files","required":false}]}"#,
    );
    let out = scratch("rv-ok-out");
    let (code, msg) = make(&out, &d, &bin);
    assert_eq!(code, 0, "{msg}");
    let req = rpm(&["-qpR"], &out.join(RPM));
    for want in ["freesynergy-form-source >= 2", "fs-module-person >= 0.0.8"] {
        assert!(req.lines().any(|l| l == want), "{want}: {req}");
    }
    let rec = rpm(&["-qp", "--recommends"], &out.join(RPM));
    assert!(rec.lines().any(|l| l == "fs-module-files"), "{rec}");
}

#[test]
fn without_the_new_fields_nothing_changes() {
    if !have_rpm() {
        return;
    }
    let (d, bin) = module(
        "rv-none",
        r#"{"requires":[{"module":"person","required":true}]}"#,
    );
    let out = scratch("rv-none-out");
    assert_eq!(make(&out, &d, &bin).0, 0);
    let req = rpm(&["-qpR"], &out.join(RPM));
    assert!(req.lines().any(|l| l == "fs-module-person"), "{req}");
    assert!(!req.contains("form-source") && !req.contains(">="), "{req}");
}

#[test]
fn a_bad_level_or_version_is_an_error_and_writes_no_rpm() {
    for (tag, commands, why) in [
        ("rv-zero", r#"{"form_source_level":0}"#, "form_source_level"),
        (
            "rv-text",
            r#"{"form_source_level":"2"}"#,
            "form_source_level",
        ),
        ("rv-neg", r#"{"form_source_level":-1}"#, "form_source_level"),
        (
            "rv-ver",
            r#"{"requires":[{"module":"person","required":true,"at_least":"1.x"}]}"#,
            "is not x.y.z",
        ),
    ] {
        let (d, bin) = module(tag, commands);
        let out = scratch(&format!("{tag}-out"));
        let (code, msg) = make(&out, &d, &bin);
        assert_ne!(code, 0, "{tag}: {msg}");
        assert!(msg.contains(why), "{tag}: {msg}");
        assert!(!out.join(RPM).exists(), "{tag}");
    }
}

#[test]
fn a_host_program_provides_the_level_and_a_zero_level_is_refused() {
    if !have_rpm() {
        return;
    }
    let ok = include_str!("data/repos/ok/scaffold.toml");
    let folder = Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/data/repos/ok");
    let with = |level: &str| {
        ok.replace(
            "[contract]\n",
            &format!("[contract]\nform_source_level = {level}\n"),
        )
    };
    let bp = Blueprint::from_toml(&with("2")).unwrap();
    let spec = program_spec(&bp, &folder, None, "1").unwrap();
    let out = scratch("rv-host");
    std::fs::create_dir_all(&out).unwrap();
    let file = spec.write_into(&out).unwrap();
    let prov = rpm(&["-qp", "--provides"], &file);
    assert!(
        prov.lines().any(|l| l == "freesynergy-form-source = 2"),
        "{prov}"
    );
    let plain = program_spec(&Blueprint::from_toml(ok).unwrap(), &folder, None, "1").unwrap();
    assert_eq!(plain.provides.len(), 0);
    let bad = program_spec(
        &Blueprint::from_toml(&with("0")).unwrap(),
        &folder,
        None,
        "1",
    );
    assert!(bad.err().is_some_and(|e| e.contains("form_source_level")));
}
