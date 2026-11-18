//! `[[module]]` with its mandatory `kind`, `[[bundle]]` as its own table (ADR 0101 E10, addendum points 4 and 5).
mod support;
use fs_blueprint::Blueprint;
use support::{codes, edit, only, run, TASKS};

const BOARD: &str = "name = \"board\"\nkind = \"component\"\nversion = \"0.0.29\"";

#[test]
fn a_module_without_kind_is_a_missing_field() {
    let e = Blueprint::from_toml(&edit(
        TASKS,
        "name = \"fs-todo\"\nkind = \"standard\"\n",
        "name = \"fs-todo\"\n",
    ))
    .expect_err("refused");
    assert_eq!(e.to_string(), "field-missing{table=module, field=kind}");
}

#[test]
fn an_unknown_kind_is_named() {
    let r = run(
        &edit(TASKS, BOARD, &BOARD.replace("component", "library")),
        true,
    );
    assert!(r
        .errors
        .iter()
        .any(|e| e.to_string() == "kind-unknown{module=board, found=library}"));
}

#[test]
fn a_kind_different_from_the_catalog_is_a_mismatch() {
    let r = run(
        &edit(TASKS, BOARD, &BOARD.replace("component", "toolkit")),
        true,
    );
    assert!(r.errors.iter().any(
        |e| e.to_string() == "kind-mismatch{module=board, declared=toolkit, catalog=component}"
    ));
}

#[test]
fn a_module_that_is_not_in_the_catalog_is_unknown() {
    let r = run(&edit(TASKS, BOARD, &BOARD.replace("board", "ghost")), true);
    assert!(r
        .errors
        .iter()
        .any(|e| e.to_string() == "module-unknown{name=ghost}"));
}

#[test]
fn a_version_without_a_tag_is_unknown_and_a_bad_form_is_no_version() {
    let r = run(
        &edit(TASKS, BOARD, &BOARD.replace("0.0.29", "0.0.28")),
        true,
    );
    assert!(r
        .errors
        .iter()
        .any(|e| e.to_string() == "version-unknown{module=board, version=0.0.28}"));
    let r = run(&edit(TASKS, BOARD, &BOARD.replace("0.0.29", "v1")), true);
    assert!(r
        .errors
        .iter()
        .any(|e| e.to_string() == "semver-invalid{key=board.version, found=v1}"));
}

#[test]
fn a_module_twice_is_a_duplicate() {
    let src = edit(
        TASKS,
        "[[bundle]]",
        &format!("[[module]]\n{BOARD}\n\n[[bundle]]"),
    );
    assert!(codes(&src).contains(&"duplicate"));
}

#[test]
fn a_part_of_a_bundle_may_not_be_a_module_too() {
    let src = edit(
        TASKS,
        "[[bundle]]",
        "[[module]]\nname = \"progress\"\nkind = \"toolkit\"\nversion = \"0.0.1\"\n\n[[bundle]]",
    );
    assert!(run(&src, true)
        .errors
        .iter()
        .any(|e| e.to_string() == "module-in-bundle{module=progress, bundle=fs-planning}"));
}

#[test]
fn an_unknown_part_of_a_bundle_is_named() {
    let src = edit(TASKS, "\"claim\"]", "\"claim\", \"ghost\"]");
    assert!(run(&src, true)
        .errors
        .iter()
        .any(|e| e.to_string() == "bundle-part-unknown{bundle=fs-planning, part=ghost}"));
}

#[test]
fn a_bundle_item_needs_its_type() {
    let src = edit(TASKS, "item = \"Task\"", "item = \"Ticket\"");
    assert!(run(&src, true)
        .errors
        .iter()
        .any(|e| e.to_string() == "type-unknown{name=Ticket}"));
}

#[test]
fn a_bundle_that_is_not_in_the_catalog_is_unknown_and_empty_parts_are_refused() {
    let src = edit(
        TASKS,
        "name = \"fs-planning\"\nversion",
        "name = \"fs-ghost\"\nversion",
    );
    assert!(run(&src, true).has("bundle-unknown"));
    let src = edit(
        TASKS,
        "parts = [\"readiness\", \"schedule\", \"progress\", \"rank\", \"checklist\", \"claim\"]",
        "parts = []",
    );
    assert!(run(&src, true)
        .errors
        .iter()
        .any(|e| e.to_string() == "field-empty{table=bundle, field=parts}"));
}

#[test]
fn a_type_of_a_module_needs_a_standard_module() {
    for module in ["board", "fs-notes"] {
        let src = edit(
            TASKS,
            "module = \"fs-todo\"",
            &format!("module = \"{module}\""),
        );
        let src = if module == "fs-notes" {
            edit(&src, "[[module]]\nname = \"board\"", "[[module]]\nname = \"fs-notes\"\nkind = \"toolkit\"\nversion = \"0.0.1\"\n\n[[module]]\nname = \"board\"")
        } else {
            src
        };
        let r = run(&src, true);
        assert!(
            r.errors
                .iter()
                .any(|e| e.code == "kind-wrong-role" && e.param("module") == Some(module)),
            "{module}: {:?}",
            r.errors
        );
    }
}

#[test]
fn a_type_of_a_module_that_is_not_declared_is_unknown_and_a_tag_error_does_not_hide_it() {
    let src = edit(TASKS, "module = \"fs-todo\"", "module = \"fs-progress\"");
    assert!(run(&src, true)
        .errors
        .iter()
        .any(|e| e.to_string() == "module-unknown{name=fs-progress}"));
}

#[test]
fn only_the_version_codes_remain_in_the_example_so_each_edit_above_is_the_only_cause() {
    assert_eq!(only(&run(TASKS, true), "version-unknown"), 2);
}
