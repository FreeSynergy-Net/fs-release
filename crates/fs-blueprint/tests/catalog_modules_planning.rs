//! The shipped catalog knows the standard `planning` (repo fs-modules-planning, tags 0.0.1 to 0.0.6 and 0.0.8, no 0.0.7; the host name, not the repo name) and the check takes it as a known module and version.
mod support;
use fs_blueprint::{check_source, FileCatalog, Kind, KitCatalog, Report, Rules};
use support::{FakeView, TASKS};

fn blueprint(module: &str, kind: &str, version: &str) -> String {
    TASKS.replace("fs-todo", module).replace(
        &format!("name = \"{module}\"\nkind = \"standard\"\nversion = \"0.0.1\""),
        &format!("name = \"{module}\"\nkind = \"{kind}\"\nversion = \"{version}\""),
    )
}

fn check(src: &str) -> Report {
    check_source(
        src,
        &FileCatalog::builtin(),
        &FakeView::standard().with_talk(),
        &Rules::projekt(),
    )
}

fn has(r: &Report, code: &str) -> bool {
    r.errors.iter().any(|e| e.to_string() == code)
}

#[test]
fn the_shipped_catalog_lists_the_planning_tags_as_a_standard_without_presets() {
    let c = FileCatalog::builtin();
    let m = c.module("planning").expect("listed");
    assert_eq!(m.kind, Kind::Standard);
    assert_eq!(
        m.tags,
        ["0.0.1", "0.0.2", "0.0.3", "0.0.4", "0.0.5", "0.0.6", "0.0.8"]
    );
    assert!(m.presets.is_empty() && m.words.is_empty());
    assert!(c.bundle("planning").is_none());
    assert!(
        c.module("fs-modules-planning").is_none(),
        "the repo name is not a module name"
    );
    assert!(c.module("fs-todo").is_none() && c.module("fs-planning").is_none());
}

#[test]
fn the_check_knows_the_module_and_its_tag() {
    let r = check(&blueprint("planning", "standard", "0.0.5"));
    assert!(!has(&r, "module-unknown{name=planning}"), "{:?}", r.errors);
    assert!(!r.errors.iter().any(|e| {
        let t = e.to_string();
        t.starts_with("version-unknown") || t.starts_with("kind-mismatch")
    }));
}

#[test]
fn another_version_or_kind_is_still_refused() {
    let r = check(&blueprint("planning", "standard", "0.0.7"));
    assert!(has(&r, "version-unknown{module=planning, version=0.0.7}"));
    let r = check(&blueprint("planning", "toolkit", "0.0.1"));
    assert!(has(
        &r,
        "kind-mismatch{module=planning, declared=toolkit, catalog=standard}"
    ));
    let r = check(&blueprint("fs-todo", "standard", "0.0.1"));
    assert!(has(&r, "module-unknown{name=fs-todo}"));
}
