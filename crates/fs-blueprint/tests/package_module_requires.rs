#![cfg(target_os = "linux")]
//! The `requires` of `catalog/commands.json` become `Requires` and `Recommends` of the module RPM (read back with the system `rpm`).
mod common;
use common::{cli, have_rpm, rpm, scratch};
use std::path::{Path, PathBuf};

fn module_folder(name: &str) -> (PathBuf, PathBuf) {
    let d = scratch(name);
    std::fs::create_dir_all(d.join("catalog")).unwrap();
    for (f, t) in [
        ("catalog/commands.json", "[]"),
        ("catalog/forms.json", "{}"),
        ("catalog/settings.json", "{}"),
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

fn make_module(out: &Path, d: &Path, bin: &Path, name: &str, arch: &str) -> (i32, String) {
    let (code, o, e) = cli(&[
        "package",
        "module",
        d.to_str().unwrap(),
        "--name",
        name,
        "--pkg-version",
        "0.0.4",
        "--binary",
        bin.to_str().unwrap(),
        "--arch",
        arch,
        "--out",
        out.to_str().unwrap(),
    ]);
    (code, format!("{o}{e}"))
}

fn deps_of(tag: &str, name: &str, commands: &str) -> Result<String, String> {
    let (d, bin) = module_folder(&format!("req-src-{tag}"));
    std::fs::write(d.join("catalog/commands.json"), commands).unwrap();
    let out = scratch(&format!("req-out-{tag}"));
    let (code, msg) = make_module(&out, &d, &bin, name, "x86_64");
    if code != 0 {
        assert!(
            std::fs::read_dir(&out).map_or(true, |mut r| r.next().is_none()),
            "nothing may be written"
        );
        return Err(msg);
    }
    let f = out.join(format!("fs-module-{name}-0.0.4-1.x86_64.rpm"));
    Ok(format!(
        "{}\n--\n{}",
        rpm(&["-qpR"], &f),
        rpm(&["-qp", "--recommends"], &f)
    ))
}

#[test]
fn requires_of_commands_json_become_requires_and_recommends() {
    if !have_rpm() {
        return;
    }
    let both = r#"{"module":"planning","requires":[{"module":"person","required":true},{"module":"texts","required":false}]}"#;
    let got = deps_of("both", "planning", both).unwrap();
    let (req, rec) = got.split_once("\n--\n").unwrap();
    assert!(req.lines().any(|l| l == "freesynergy-filesystem"), "{got}");
    assert!(req.lines().any(|l| l == "fs-module-person"), "{got}");
    assert!(!req.contains("fs-module-texts"), "{got}");
    assert!(rec.lines().any(|l| l == "fs-module-texts"), "{got}");
    assert!(!rec.contains("fs-module-person"), "{got}");
}

#[test]
fn no_requires_field_keeps_only_the_filesystem() {
    if !have_rpm() {
        return;
    }
    for c in ["[]", r#"{"module":"planning","commands":[]}"#] {
        let got = deps_of("none", "planning", c).unwrap();
        let (req, rec) = got.split_once("\n--\n").unwrap();
        assert!(!req.contains("fs-module-"), "{got}");
        assert!(rec.trim().is_empty() || rec.trim() == "(none)", "{got}");
    }
}

#[test]
fn a_bad_requires_is_a_clear_error_and_writes_nothing() {
    for (c, word) in [
        (r#"{"requires":"person"}"#, "requires"),
        (
            r#"{"requires":[{"module":"Per son","required":true}]}"#,
            "not usable",
        ),
        (
            r#"{"requires":[{"module":"planning","required":true}]}"#,
            "itself",
        ),
        (r#"{"requires":[{"module":"person"}]}"#, "required"),
        (
            r#"{"requires":[{"module":"fs-person","required":true}]}"#,
            "fs-",
        ),
        ("not json", "commands.json"),
    ] {
        let e = deps_of("bad", "planning", c).unwrap_err();
        assert!(e.contains(word), "{c}: {e}");
    }
}
