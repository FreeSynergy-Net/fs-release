//! `[program]`, `[contract]`, `[[ruleset]]` and `[frontend]`: forms and references (addendum point 4).
mod support;
use support::{edit, run, TASKS};

fn has(src: &str, text: &str) -> bool {
    run(src, true).errors.iter().any(|e| e.to_string() == text)
}

#[test]
fn the_program_name_follows_the_naming_rule() {
    for bad in ["Tasks", "fs-tasks", "1tasks", "ta_sks", ""] {
        assert!(
            has(
                &edit(TASKS, "name = \"tasks\"", &format!("name = \"{bad}\"")),
                &format!("name-rule{{name={bad}}}")
            ),
            "{bad:?}"
        );
    }
    assert!(!run(
        &edit(TASKS, "name = \"tasks\"", "name = \"my-tasks2\""),
        true
    )
    .has("name-rule"));
}

#[test]
fn skeleton_and_capability_must_exist() {
    assert!(has(
        &edit(TASKS, "skeleton = \"0.1.0\"", "skeleton = \"9.9.9\""),
        "skeleton-unknown{found=9.9.9}"
    ));
    assert!(has(
        &edit(TASKS, "skeleton = \"0.1.0\"", "skeleton = \"1\""),
        "semver-invalid{key=program.skeleton, found=1}"
    ));
    assert!(has(
        &edit(TASKS, "capability = \"tasks\"", "capability = \"nope\""),
        "capability-unknown{key=nope}"
    ));
    assert!(has(
        &edit(TASKS, "\"dependency.evaluate\"", "\"dependency.ghost\""),
        "capability-unknown{key=dependency.ghost}"
    ));
}

#[test]
fn a_main_category_is_needed() {
    assert!(has(
        &edit(
            TASKS,
            "categories = [\"Office\", \"ProjectManagement\"]",
            "categories = [\"ProjectManagement\"]"
        ),
        "category-main-missing"
    ));
    assert!(has(
        &edit(
            TASKS,
            "categories = [\"Office\", \"ProjectManagement\"]",
            "categories = []"
        ),
        "field-empty{table=program, field=categories}"
    ));
}

#[test]
fn the_contract_has_a_major_minor_version_and_a_file_id() {
    assert!(has(
        &edit(TASKS, "version = \"0.1\"", "version = \"0.1.0\""),
        "contract-version-invalid{found=0.1.0}"
    ));
    for bad in [
        "0xc04d25897154347",
        "c04d258971543476",
        "0xc04d25897154347g",
        "0xc04d2589715434761",
    ] {
        assert!(
            has(
                &edit(TASKS, "0xc04d258971543476", bad),
                &format!("contract-id-invalid{{found={bad}}}")
            ),
            "{bad}"
        );
    }
}

#[test]
fn a_rule_set_needs_a_default_among_its_presets_and_known_presets() {
    assert!(has(
        &edit(TASKS, "default = \"freesynergy\"", "default = \"other\""),
        "ruleset-default-missing{ruleset=progress}"
    ));
    assert!(has(
        &edit(
            TASKS,
            "presets = [\"rfc8984\", \"freesynergy\"]",
            "presets = [\"rfc8984\", \"freesynergy\", \"x\"]"
        ),
        "preset-unknown{ruleset=progress, preset=x}"
    ));
    assert!(has(
        &edit(
            TASKS,
            "presets = [\"rfc8984\", \"freesynergy\"]",
            "presets = []"
        ),
        "field-empty{table=ruleset, field=presets}"
    ));
}

#[test]
fn a_rule_set_needs_a_type_and_a_source_of_the_right_kind() {
    assert!(has(
        &edit(TASKS, "type = \"Task\"\nfrom", "type = \"Ghost\"\nfrom"),
        "type-unknown{name=Ghost}"
    ));
    assert!(has(
        &edit(TASKS, "from = \"fs-planning\"", "from = \"nobody\""),
        "ruleset-source-unknown{name=nobody}"
    ));
    assert!(has(
        &edit(TASKS, "from = \"fs-planning\"", "from = \"board\""),
        "kind-wrong-role{module=board, expected=standard|toolkit}"
    ));
}

#[test]
fn the_forms_description_lies_in_the_forms_folder_in_the_root() {
    for bad in [
        "layout/board.toml",
        "board.toml",
        "crates/x/forms/board.toml",
        "forms",
        "forms/",
        "Forms/board.toml",
    ] {
        let src = edit(TASKS, "forms/board.toml", bad);
        let r = run(&src, true);
        assert!(
            r.errors
                .iter()
                .any(|e| e.to_string() == format!("layout-outside-forms{{path={bad}}}")),
            "{bad}: {:?}",
            r.errors
        );
    }
    assert!(!run(TASKS, true).has("layout-outside-forms"));
    assert!(!run(
        &edit(TASKS, "forms/board.toml", "forms/sub/board.toml"),
        true
    )
    .has("layout-outside-forms"));
}

#[test]
fn the_frontend_needs_its_layout_file_and_a_valid_cover() {
    assert!(has(
        &edit(TASKS, "forms/board.toml", "forms/missing.toml"),
        "layout-not-found{path=forms/missing.toml}"
    ));
    assert!(has(
        &edit(TASKS, "forms/board.toml", "../board.toml"),
        "path-invalid{path=../board.toml}"
    ));
    assert!(has(&edit(TASKS, "covers = [\"Items.list\", \"Items.query\", \"Items.create\", \"Items.editIf\", \"Items.readiness\"]", "covers = \"some\""), "covers-invalid{found=some}"));
    assert!(has(&edit(TASKS, "covers = [\"Items.list\", \"Items.query\", \"Items.create\", \"Items.editIf\", \"Items.readiness\"]", "covers = []"), "field-empty{table=frontend, field=covers}"));
    assert!(!run(&edit(TASKS, "covers = [\"Items.list\", \"Items.query\", \"Items.create\", \"Items.editIf\", \"Items.readiness\"]", "covers = \"all\""), true).has("covers-invalid"));
}

#[test]
fn a_frontend_never_names_a_crate() {
    // there is no key for a crate: the old `standard = "simple"` style crate switch does not exist in schema 2
    assert!(fs_blueprint::Blueprint::from_toml(&edit(
        TASKS,
        "[frontend]\n",
        "[frontend]\ncrate = \"board\"\n"
    ))
    .is_err());
}
