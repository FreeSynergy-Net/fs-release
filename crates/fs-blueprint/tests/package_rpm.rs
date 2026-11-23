//! `fs-blueprint package rpm|release`: the made RPM is read back with the system `rpm -qp` (files, requires, arch, desktop entry, no signature).
use fs_blueprint::cli::run;
use std::path::{Path, PathBuf};
use std::process::Command;

fn data(name: &str) -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("tests/data/repos")
        .join(name)
}

fn scratch(name: &str) -> PathBuf {
    let d =
        Path::new(env!("CARGO_TARGET_TMPDIR")).join(format!("pkg-{name}-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&d);
    d
}

fn cli(words: &[&str]) -> (i32, String, String) {
    let words: Vec<String> = words.iter().map(|w| (*w).to_string()).collect();
    let (mut out, mut err) = (Vec::new(), Vec::new());
    let code = run(&words, &mut out, &mut err);
    (
        code,
        String::from_utf8(out).unwrap(),
        String::from_utf8(err).unwrap(),
    )
}

/// `rpm` must be there: a missing tool is a RED test, never a silent pass (review S5). The one way out is the explicit `FS_SKIP_RPM_TESTS=1`, and never when `CI` is set; it is said loudly.
fn have_rpm() -> bool {
    if Command::new("rpm").arg("--version").output().is_ok() {
        return true;
    }
    let skip = std::env::var("FS_SKIP_RPM_TESTS").is_ok_and(|v| v == "1");
    assert!(
        skip && std::env::var_os("CI").is_none(),
        "rpm is not installed: install it (CI installs it) or set FS_SKIP_RPM_TESTS=1 outside CI"
    );
    eprintln!("SKIPPED: rpm is not installed (FS_SKIP_RPM_TESTS=1)");
    false
}

const FP: &str = "16A4A3EE563AEF51FCF0C55DED7B57C4A1904342";

fn rpm(args: &[&str], file: &Path) -> String {
    let o = Command::new("rpm").args(args).arg(file).output().unwrap();
    assert!(
        o.status.success(),
        "rpm {args:?}: {}",
        String::from_utf8_lossy(&o.stderr)
    );
    String::from_utf8(o.stdout).unwrap()
}

fn make_demo(out: &Path, more: &[&str]) -> PathBuf {
    let bp = data("ok").join("scaffold.toml");
    let mut w = vec![
        "package",
        "rpm",
        bp.to_str().unwrap(),
        "--out",
        out.to_str().unwrap(),
    ];
    w.extend_from_slice(more);
    let (code, o, e) = cli(&w);
    assert_eq!(code, 0, "{o}{e}");
    out.join("fs-demo-0.0.1-1.noarch.rpm")
}

#[test]
fn the_program_package_has_exactly_the_files_of_the_decision() {
    if !have_rpm() {
        return;
    }
    let f = make_demo(&scratch("files"), &["--starter-min", "0.0.4"]);
    let list = rpm(&["-qpl"], &f);
    let want = [
        "/usr/share/applications/net.freesynergy.Demo.desktop",
        "/usr/share/freesynergy/programs/demo",
        "/usr/share/freesynergy/programs/demo/forms",
        "/usr/share/freesynergy/programs/demo/forms/demo.toml",
        "/usr/share/freesynergy/programs/demo/scaffold.toml",
    ];
    assert_eq!(list.lines().collect::<Vec<_>>(), want);
    let info = rpm(
        &["-qp", "--qf", "%{NAME} %{VERSION} %{ARCH} %{LICENSE}\n"],
        &f,
    );
    assert_eq!(info.trim(), "fs-demo 0.0.1 noarch MIT OR Apache-2.0");
}

#[test]
fn requires_come_from_the_blueprint_and_the_option() {
    if !have_rpm() {
        return;
    }
    let f = make_demo(&scratch("req"), &["--starter-min", "0.0.4"]);
    let req = rpm(&["-qpR"], &f);
    assert!(req.lines().any(|l| l == "fs-starter >= 0.0.4"), "{req}");
    assert!(
        req.lines().any(|l| l == "fs-module-board >= 0.0.29"),
        "{req}"
    );
    let plain = make_demo(&scratch("req2"), &[]);
    assert!(rpm(&["-qpR"], &plain).lines().any(|l| l == "fs-starter"));
}

#[test]
fn the_desktop_entry_starts_the_program_through_the_starter() {
    if !have_rpm() {
        return;
    }
    let f = make_demo(&scratch("desk"), &[]);
    let out = Command::new("sh")
        .arg("-c")
        .arg(format!(
            "rpm2cpio '{}' | cpio -i --to-stdout '*.desktop' 2>/dev/null",
            f.display()
        ))
        .output()
        .unwrap();
    let text = String::from_utf8(out.stdout).unwrap();
    assert!(text.contains("Exec=fs-starter demo\n"), "{text}");
    assert!(text.contains("Categories=Office;\n"), "{text}");
}

#[test]
fn nothing_is_signed_locally() {
    if !have_rpm() {
        return;
    }
    let f = make_demo(&scratch("sig"), &[]);
    assert!(rpm(&["-qp", "--qf", "%{SIGPGP:pgpsig}|%{RSAHEADER:pgpsig}"], &f).contains("(none)"));
}

#[test]
fn a_blueprint_with_findings_makes_no_package() {
    let out = scratch("bad");
    let bp = data("ok").join("scaffold.toml");
    let text = std::fs::read_to_string(&bp)
        .unwrap()
        .replace("capability = \"demo\"", "capability = \"nonsense\"");
    let dir = scratch("bad-src");
    std::fs::create_dir_all(&dir).unwrap();
    std::fs::write(dir.join("scaffold.toml"), text).unwrap();
    let (code, ..) = cli(&[
        "package",
        "rpm",
        dir.join("scaffold.toml").to_str().unwrap(),
        "--out",
        out.to_str().unwrap(),
    ]);
    assert_ne!(code, 0);
    assert!(!out.exists());
}

fn key() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../../release-signing-key.asc")
}

#[test]
fn the_release_package_has_repo_file_and_key_with_stable_on_and_nightly_off() {
    if !have_rpm() {
        return;
    }
    let out = scratch("rel");
    let (code, o, e) = cli(&[
        "package",
        "release",
        "--key",
        key().to_str().unwrap(),
        "--fingerprint",
        FP,
        "--pages-url",
        "https://pages.invalid/Repository",
        "--out",
        out.to_str().unwrap(),
    ]);
    assert_eq!(code, 0, "{o}{e}");
    let f = out.join(format!(
        "freesynergy-release-{}-1.noarch.rpm",
        env!("CARGO_PKG_VERSION")
    ));
    let list = rpm(&["-qpl"], &f);
    assert_eq!(
        list.lines().collect::<Vec<_>>(),
        [
            "/etc/pki/rpm-gpg/RPM-GPG-KEY-freesynergy",
            "/etc/yum.repos.d/freesynergy.repo"
        ]
    );
    let cfg = rpm(&["-qpc"], &f);
    assert!(cfg.contains("freesynergy.repo"));
    let repo = fs_blueprint::package_rpm::repo_file("https://pages.invalid/Repository");
    let (stable, nightly) = repo.split_once("\n[freesynergy-nightly]").unwrap();
    assert!(stable.contains("enabled=1") && nightly.contains("enabled=0"));
    for part in [stable, nightly] {
        assert!(part.contains("gpgcheck=1") && part.contains("repo_gpgcheck=1"));
    }
}

#[test]
fn the_release_package_is_refused_without_a_pages_address_or_with_a_secret_key() {
    let out = scratch("relbad");
    let k = key();
    let (code, ..) = cli(&[
        "package",
        "release",
        "--key",
        k.to_str().unwrap(),
        "--out",
        out.to_str().unwrap(),
    ]);
    assert_eq!(code, 2);
    let secret = scratch("secret");
    std::fs::create_dir_all(&secret).unwrap();
    let sk = secret.join("k.asc");
    std::fs::write(&sk, "-----BEGIN PGP PRIVATE KEY BLOCK-----\nx\n").unwrap();
    let (code, ..) = cli(&[
        "package",
        "release",
        "--key",
        sk.to_str().unwrap(),
        "--fingerprint",
        FP,
        "--pages-url",
        "https://x.invalid",
        "--out",
        out.to_str().unwrap(),
    ]);
    assert_eq!(code, 2);
    assert!(!out.exists());
}

#[test]
fn changelog_and_build_host_tags_exist_and_the_package_is_reproducible_with_a_source_date() {
    if !have_rpm() {
        return;
    }
    let (a, b) = (scratch("rep-a"), scratch("rep-b"));
    let fa = make_demo(&a, &["--source-date", "1700000000"]);
    let fb = make_demo(&b, &["--source-date", "1700000000"]);
    assert_eq!(std::fs::read(&fa).unwrap(), std::fs::read(&fb).unwrap());
    let tags = rpm(&["-qp", "--qf", "%{BUILDHOST}|%{CHANGELOGNAME}"], &fa);
    assert_eq!(tags, "fs-blueprint|FreeSynergy");
    let desc = rpm(&["-qp", "--qf", "%{DESCRIPTION}"], &fa);
    assert!(desc.lines().all(|l| l.len() <= 79), "{desc}");
}
