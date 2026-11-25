//! Which `[[module]]` becomes a `Requires` of the program package: `standard` and `toolkit` yes (their own package `fs-module-<name>`), `component` no (display only,
//! lives in the host and in fs-starter). And the licence files are a must, like for a module package.
use fs_blueprint::package_rpm::{needs_package, program_spec};
use fs_blueprint::Blueprint;
use std::path::Path;

const OK: &str = include_str!("data/repos/ok/scaffold.toml");

fn spec_with(kind: &str) -> Vec<String> {
    let text = OK.replace(
        "name = \"board\"\nkind = \"component\"",
        &format!("name = \"board\"\nkind = \"{kind}\""),
    );
    let bp = Blueprint::from_toml(&text).unwrap();
    let folder = Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/data/repos/ok");
    let s = program_spec(&bp, &folder, None, "1").unwrap();
    s.requires.into_iter().map(|r| r.name).collect()
}

#[test]
fn the_kinds_decide_who_is_a_package() {
    assert!(needs_package("standard"));
    assert!(needs_package("toolkit"));
    assert!(!needs_package("component"));
    assert!(!needs_package("nonsense"));
}

#[test]
fn a_standard_and_a_toolkit_are_required_a_component_is_not() {
    for kind in ["standard", "toolkit"] {
        assert!(
            spec_with(kind).contains(&"fs-module-board".to_string()),
            "{kind}"
        );
    }
    let comp = spec_with("component");
    assert!(!comp.iter().any(|n| n == "fs-module-board"), "{comp:?}");
    assert_eq!(comp, ["fs-starter", "freesynergy-filesystem"]);
}

#[test]
fn each_missing_licence_file_alone_stops_the_package() {
    let src = Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/data/repos/ok");
    let bp = Blueprint::from_toml(OK).unwrap();
    for gone in ["LICENSE-MIT", "LICENSE-APACHE"] {
        let d = Path::new(env!("CARGO_TARGET_TMPDIR"))
            .join(format!("lic-{gone}-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&d);
        std::fs::create_dir_all(&d).unwrap();
        std::fs::write(d.join("scaffold.toml"), OK).unwrap();
        for keep in ["LICENSE-MIT", "LICENSE-APACHE"]
            .into_iter()
            .filter(|k| *k != gone)
        {
            std::fs::copy(src.join(keep), d.join(keep)).unwrap();
        }
        let e = program_spec(&bp, &d, None, "1").err().expect("must stop");
        assert!(e.contains(gone), "{gone}: {e}");
        std::fs::remove_dir_all(&d).unwrap();
    }
}
