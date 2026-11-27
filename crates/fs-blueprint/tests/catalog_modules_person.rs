//! The shipped catalog knows the standard `person` (repo fs-modules-person, tags 0.0.1 and 0.0.3; the host name, not the repo name) and the check takes it as a known module and version.
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
fn the_shipped_catalog_lists_the_person_tags_as_a_standard() {
    let c = FileCatalog::builtin();
    let m = c.module("person").expect("listed");
    assert_eq!(m.kind, Kind::Standard);
    assert_eq!(m.tags, ["0.0.1", "0.0.3"]);
    assert!(
        c.module("fs-modules-person").is_none(),
        "the repo name is not a module name"
    );
}

#[test]
fn the_check_knows_the_module_and_its_tag() {
    let r = check(&blueprint("person", "standard", "0.0.3"));
    assert!(!has(&r, "module-unknown{name=person}"), "{:?}", r.errors);
    assert!(!r.errors.iter().any(|e| {
        let t = e.to_string();
        t.starts_with("version-unknown") || t.starts_with("kind-mismatch")
    }));
}

#[test]
fn another_version_or_kind_is_refused() {
    let r = check(&blueprint("person", "standard", "0.0.2"));
    assert!(has(&r, "version-unknown{module=person, version=0.0.2}"));
    let r = check(&blueprint("person", "toolkit", "0.0.1"));
    assert!(has(
        &r,
        "kind-mismatch{module=person, declared=toolkit, catalog=standard}"
    ));
}
