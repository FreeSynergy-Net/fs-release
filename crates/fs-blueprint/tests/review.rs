//! Conditions of the S1 review (B1, B2, B3) and of decision 6e (B6) on the check of the blueprint.
mod support;
use support::{edit, run, FakeView, SHIFT, TASKS};

fn has(src: &str, text: &str) -> bool {
    run(src, true).errors.iter().any(|e| e.to_string() == text)
}
fn none(src: &str, code: &str) -> bool {
    !run(src, true).has(code)
}

const FIELD_STD: &str = "standard = \"https://www.rfc-editor.org/rfc/rfc8984#section-5.1\"";

#[test]
fn b1_the_standard_of_a_field_needs_the_uri_form_with_a_section() {
    for bad in [
        "foo",
        "https://x.org/a",
        "ftp://x.org/a#b",
        "https://#b",
        "https://x.org/a#",
    ] {
        let src = edit(SHIFT, FIELD_STD, &format!("standard = \"{bad}\""));
        assert!(
            has(
                &src,
                &format!("standard-invalid{{type=Shift, found={bad}}}")
            ),
            "{bad}"
        );
    }
    assert!(none(SHIFT, "standard-invalid"));
}

#[test]
fn b1_the_field_standard_rule_follows_the_rule_set() {
    let src = edit(SHIFT, FIELD_STD, "standard = \"foo\"");
    let rule = |m| fs_blueprint::Rules::projekt().with(fs_blueprint::Rule::StandardEvidence, m);
    let off = support::run_with(&src, FakeView::standard(), &rule(fs_blueprint::Mode::Off));
    assert!(
        !off.has("standard-invalid") && off.warnings.iter().all(|w| w.code != "standard-invalid")
    );
    let warn = support::run_with(&src, FakeView::standard(), &rule(fs_blueprint::Mode::Warn));
    assert!(
        !warn.has("standard-invalid") && warn.warnings.iter().any(|w| w.code == "standard-invalid")
    );
}

#[test]
fn b2_empty_or_shapeless_values_are_refused() {
    assert!(has(
        &edit(SHIFT, "envelope = \"Event\"", "envelope = \"\""),
        "envelope-invalid{type=Shift, found=}"
    ));
    assert!(has(
        &edit(SHIFT, "envelope = \"Event\"", "envelope = \"not a type\""),
        "envelope-invalid{type=Shift, found=not a type}"
    ));
    assert!(none(
        &edit(
            SHIFT,
            "envelope = \"Event\"",
            "envelope = \"https://x.org/ns#Shift\""
        ),
        "envelope-invalid"
    ));
    assert!(has(
        &edit(SHIFT, "standard = \"list\"", "standard = \"\""),
        "field-empty{table=frontend, field=standard}"
    ));
    assert!(has(
        &edit(SHIFT, "standard = \"list\"", "standard = \"Not Kebab\""),
        "name-case{key=frontend.standard, found=Not Kebab}"
    ));
    assert!(has(
        &edit(TASKS, "what = \"Sperrt", "what = \"\"\nwhat2 = \"Sperrt").replace("what2", "#"),
        "field-empty{table=exception, field=what}"
    ));
    assert!(has(
        &edit(TASKS, "why = \"kein Modul", "why = \"  \"\n# \"kein Modul"),
        "field-empty{table=exception, field=why}"
    ));
}

#[test]
fn b2_every_covers_entry_has_the_form_interface_dot_method_or_type_dot_field() {
    let list = |l: &str| {
        edit(TASKS, "covers = [\"Items.list\", \"Items.query\", \"Items.create\", \"Items.editIf\", \"Items.readiness\"]", &format!("covers = [{l}]"))
    };
    assert!(has(
        &list("\"Items.list\", \"x\""),
        "covers-invalid{found=x}"
    ));
    for bad in [
        "",
        "Items.",
        ".list",
        "items.list",
        "Items.List",
        "Items list",
        "A.b.c",
    ] {
        assert!(
            has(
                &list(&format!("\"{bad}\"")),
                &format!("covers-invalid{{found={bad}}}")
            ),
            "{bad:?}"
        );
    }
    assert!(none(
        &list("\"Items.list\", \"Task.dueDate\""),
        "covers-invalid"
    ));
}

#[test]
fn b3_a_version_with_two_parts_is_no_version() {
    for bad in ["1.2", "1", "1.2.3.4", "01.2.3", "1.2.x", ""] {
        let src = edit(
            TASKS,
            "name = \"fs-todo\"\nkind = \"standard\"\nversion = \"0.0.1\"",
            &format!("name = \"fs-todo\"\nkind = \"standard\"\nversion = \"{bad}\""),
        );
        assert!(
            has(
                &src,
                &format!("semver-invalid{{key=fs-todo.version, found={bad}}}")
            ),
            "{bad:?}"
        );
    }
}

#[test]
fn b3_a_folder_and_a_file_inside_it_in_one_exception_do_not_overlap_but_in_two_they_do() {
    let one = edit(
        TASKS,
        "paths = [\"crates/fs-tasks-core/src/secret_guard.rs\"]",
        "paths = [\"crates/fs-tasks-core/src/\", \"crates/fs-tasks-core/src/secret_guard.rs\"]",
    );
    assert!(none(&one, "path-overlap"));
    let two = format!("{one}\n[[exception]]\nwhat = \"b\"\nwhy = \"c\"\nchecked = [\"x\"]\nalternatives = []\ntalk = \"conventions/talks/2026-11-xx-tasks-secret-guard.md\"\npaths = [\"crates/fs-tasks-core/src/extra.rs\"]\n");
    assert!(has(
        &two,
        "path-overlap{path=crates/fs-tasks-core/src/extra.rs, first=1, second=2}"
    ));
}

const TWO_TYPES_EXC: &str = "[[exception]]\nwhat = \"Type Extra has no standard\"\nwhy = \"own thing\"\nchecked = [\"x\"]\nalternatives = []\ntalk = \"conventions/talks/2026-11-xx-tasks-secret-guard.md\"\npaths = [\"crates/fs-tasks-core/src/extra.rs\"]\n";

fn extra_type() -> String {
    edit(SHIFT, "[frontend]", "[[type]]\nname = \"Extra\"\nenvelope = \"Note\"\nmedia_type = \"a/b\"\nlabel = { en = \"E\", de = \"E\" }\n\n[frontend]")
}

#[test]
fn b6_a_type_without_a_standard_needs_an_exception_with_a_talk_record_that_names_it() {
    let bare = extra_type();
    assert!(has(&bare, "field-missing{table=type, field=standard}"));
    let with_exc = format!("{bare}\n{TWO_TYPES_EXC}");
    assert!(none(&with_exc, "field-missing"));
    assert!(run(&with_exc, false).has("talk-not-found"));
    assert!(
        run(&with_exc, false).has("field-missing"),
        "no talk record: no exception"
    );
    let other = with_exc.replace("Type Extra has", "Type Other has");
    assert!(
        run(&other, true).has("field-missing"),
        "the exception must name the type"
    );
    let substring = with_exc.replace("Type Extra has", "Type Extras has");
    assert!(
        run(&substring, true).has("field-missing"),
        "a longer word is not the name"
    );
}

#[test]
fn b6_words_may_only_name_modules_that_stand_in_the_blueprint() {
    let word = |extra: &str| {
        edit(
            SHIFT,
            "kit = \"personRef\"",
            "basic = \"choice\"\nwords = \"fs-progress:state\"",
        )
        .replace("[[type]]", &format!("{extra}[[type]]"))
    };
    let module = "[[module]]\nname = \"fs-progress\"\nkind = \"standard\"\nversion = \"0.0.1\"\n\n";
    assert!(has(&word(""), "words-unknown{ref=fs-progress:state}"));
    assert!(none(&word(module), "words-unknown"));
    let part = "[[bundle]]\nname = \"fs-planning\"\nversion = \"0.0.1\"\nparts = [\"fs-progress\"]\nitem = \"Shift\"\n\n";
    assert!(
        has(&word(part), "words-unknown{ref=fs-progress:state}"),
        "a bundle part is no module of the blueprint"
    );
}
