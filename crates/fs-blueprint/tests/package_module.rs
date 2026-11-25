#![cfg(target_os = "linux")]
//! `fs-blueprint package filesystem|module`: the made RPMs are read back with the system `rpm -qp` (INST-4a); links and unsafe names are refused.
mod common;
use common::{cli, data, have_rpm, make_demo, rpm, scratch};
use std::path::{Path, PathBuf};

// ---- INST-4a: `package filesystem` and `package module` ----

const SHARED: [&str; 6] = [
    "/usr/libexec/freesynergy",
    "/usr/libexec/freesynergy/modules",
    "/usr/share/freesynergy",
    "/usr/share/freesynergy/frontends",
    "/usr/share/freesynergy/modules",
    "/usr/share/freesynergy/programs",
];

/// A module folder as the decision describes it, plus a fake binary.
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

#[test]
fn the_filesystem_package_owns_every_shared_folder_and_nothing_else() {
    if !have_rpm() {
        return;
    }
    let out = scratch("fs");
    let (code, o, e) = cli(&[
        "package",
        "filesystem",
        "--pkg-version",
        "0.0.1",
        "--out",
        out.to_str().unwrap(),
    ]);
    assert_eq!(code, 0, "{o}{e}");
    let f = out.join("freesynergy-filesystem-0.0.1-1.noarch.rpm");
    let list = rpm(&["-qpl"], &f);
    assert_eq!(list.lines().collect::<Vec<_>>(), SHARED);
    // every entry is a folder (d), root:root, 0755: a file would show another type
    let modes = rpm(
        &[
            "-qp",
            "--qf",
            "[%{FILEMODES:perms} %{FILEUSERNAME}:%{FILEGROUPNAME}\n]",
        ],
        &f,
    );
    assert!(
        modes.lines().all(|l| l == "drwxr-xr-x root:root"),
        "{modes}"
    );
}

#[test]
fn the_module_package_has_the_folder_layout_modes_and_requires_of_the_decision() {
    if !have_rpm() {
        return;
    }
    let (d, bin) = module_folder("mod-src");
    let out = scratch("mod-out");
    let (code, msg) = make_module(&out, &d, &bin, "planning", "x86_64");
    assert_eq!(code, 0, "{msg}");
    let f = out.join("fs-module-planning-0.0.4-1.x86_64.rpm");
    let want = [
        "/usr/libexec/freesynergy/modules/planning",
        "/usr/libexec/freesynergy/modules/planning/catalog",
        "/usr/libexec/freesynergy/modules/planning/catalog/commands.json",
        "/usr/libexec/freesynergy/modules/planning/catalog/forms.json",
        "/usr/libexec/freesynergy/modules/planning/catalog/settings.json",
        "/usr/libexec/freesynergy/modules/planning/service",
        "/usr/share/freesynergy/modules/planning.metainfo.xml",
        "/usr/share/licenses/fs-module-planning",
        "/usr/share/licenses/fs-module-planning/LICENSE-APACHE",
        "/usr/share/licenses/fs-module-planning/LICENSE-MIT",
    ];
    let list = rpm(&["-qpl"], &f);
    let mut got: Vec<_> = list.lines().collect();
    got.sort_unstable();
    assert_eq!(got, want);
    // the shared folders belong to freesynergy-filesystem, not to the module
    assert!(SHARED.iter().all(|s| !list.lines().any(|l| l == *s)));
    let modes = rpm(
        &[
            "-qp",
            "--qf",
            "[%{FILENAMES} %{FILEMODES:perms} %{FILEUSERNAME}:%{FILEGROUPNAME}\n]",
        ],
        &f,
    );
    let mode_of = |p: &str| {
        modes
            .lines()
            .find_map(|l| l.strip_prefix(&format!("{p} ")))
            .unwrap_or_else(|| panic!("{p} not in {modes}"))
            .to_string()
    };
    assert_eq!(
        mode_of("/usr/libexec/freesynergy/modules/planning/service"),
        "-rwxr-xr-x root:root"
    );
    assert_eq!(
        mode_of("/usr/libexec/freesynergy/modules/planning/catalog/commands.json"),
        "-rw-r--r-- root:root"
    );
    assert_eq!(
        mode_of("/usr/libexec/freesynergy/modules/planning"),
        "drwxr-xr-x root:root"
    );
    assert_eq!(
        mode_of("/usr/share/licenses/fs-module-planning"),
        "drwxr-xr-x root:root"
    );
    let info = rpm(&["-qp", "--qf", "%{NAME} %{VERSION} %{ARCH}"], &f);
    assert_eq!(info, "fs-module-planning 0.0.4 x86_64");
    let req = rpm(&["-qpR"], &f);
    assert!(req.lines().any(|l| l == "freesynergy-filesystem"), "{req}");
    let prov = rpm(&["-qp", "--provides"], &f);
    assert!(
        prov.lines()
            .any(|l| l.starts_with("fs-module-planning = 0.0.4")),
        "{prov}"
    );
}

#[test]
fn a_module_package_is_refused_for_a_bad_name_cpu_or_missing_file() {
    let (d, bin) = module_folder("mod-bad-src");
    let out = scratch("mod-bad-out");
    // repo name instead of host module name
    assert_eq!(
        make_module(&out, &d, &bin, "fs-modules-planning", "x86_64").0,
        2
    );
    // not a supported CPU
    assert_eq!(make_module(&out, &d, &bin, "planning", "noarch").0, 2);
    // required file missing
    std::fs::remove_file(d.join("catalog/forms.json")).unwrap();
    assert_eq!(make_module(&out, &d, &bin, "planning", "x86_64").0, 2);
    assert!(!out.exists());
}

#[test]
fn the_requires_use_the_host_module_name_and_a_repo_name_is_refused() {
    if !have_rpm() {
        return;
    }
    let f = make_demo(&scratch("reqname"), &[]);
    let req = rpm(&["-qpR"], &f);
    assert!(req.lines().any(|l| l == "freesynergy-filesystem"), "{req}");
    // `board` is a component: no Requires (tests/package_requires_kind.rs: a standard `board` gives fs-module-board, nothing is cut off)
    assert!(!req.contains("fs-module-board"), "{req}");
    // a blueprint that names the repo (`fs-modules-planning`) must not become fs-module-modules-planning
    let text = std::fs::read_to_string(data("ok").join("scaffold.toml"))
        .unwrap()
        .replace(
            "name = \"board\"\nkind = \"component\"",
            "name = \"fs-modules-planning\"\nkind = \"standard\"",
        );
    let bp =
        fs_blueprint::model::Blueprint::from_toml(&text).unwrap_or_else(|_| panic!("blueprint"));
    let Err(err) = fs_blueprint::package_rpm::program_spec(&bp, &data("ok"), None, "1") else {
        panic!("a repo name must be refused")
    };
    assert!(err.contains("starts with fs-"), "{err}");
    assert_eq!(
        fs_blueprint::package_rpm::module_package("planning").unwrap(),
        "fs-module-planning"
    );
}

#[test]
fn a_module_folder_with_a_link_for_metainfo_licence_or_catalog_is_refused() {
    // The file the link points at must never end up in a package.
    let secret = scratch("link-secret");
    std::fs::create_dir_all(&secret).unwrap();
    let target = secret.join("hostname");
    std::fs::write(&target, "build-machine-secret").unwrap();
    for (case, victim) in [
        ("metainfo", "metainfo.xml"),
        ("lic-mit", "LICENSE-MIT"),
        ("lic-apache", "LICENSE-APACHE"),
    ] {
        let (d, bin) = module_folder(&format!("mod-link-{case}"));
        std::fs::remove_file(d.join(victim)).unwrap();
        std::os::unix::fs::symlink(&target, d.join(victim)).unwrap();
        let out = scratch(&format!("mod-link-{case}-out"));
        let (code, text) = make_module(&out, &d, &bin, "planning", "x86_64");
        assert_eq!(code, 2, "{victim}: {text}");
        assert!(text.contains("a link is not packaged"), "{victim}: {text}");
        assert!(!out.exists(), "{victim}: a package was made");
    }
    // the folder `catalog` itself as a link
    let (d, bin) = module_folder("mod-link-catalog");
    let real = secret.join("catalog");
    std::fs::rename(d.join("catalog"), &real).unwrap();
    std::os::unix::fs::symlink(&real, d.join("catalog")).unwrap();
    let out = scratch("mod-link-catalog-out");
    let (code, text) = make_module(&out, &d, &bin, "planning", "x86_64");
    assert_eq!(code, 2, "{text}");
    assert!(text.contains("a link is not packaged"), "{text}");
    assert!(!out.exists());
}

#[test]
fn unsafe_module_names_are_refused_by_the_name_check() {
    // `valid_name` is the only guard against a path in the name (the rpm crate would panic on `..`).
    let (d, bin) = module_folder("mod-unsafe-src");
    let out = scratch("mod-unsafe-out");
    for name in [
        "../x",
        "planning/../../etc",
        "Planning",
        "a_b",
        "-x",
        "x-",
        "a--b",
        "",
    ] {
        let (code, text) = make_module(&out, &d, &bin, name, "x86_64");
        assert_eq!(code, 2, "{name:?}: {text}");
        assert!(
            text.contains("not usable as a package name"),
            "{name:?}: {text}"
        );
        assert!(
            fs_blueprint::package_rpm::module_package(name).is_err(),
            "{name:?}"
        );
    }
    assert!(!out.exists());
}
