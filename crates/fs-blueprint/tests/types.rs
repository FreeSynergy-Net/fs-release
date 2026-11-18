//! `[[type]]` and `[[type.field]]`: types of modules carry nothing of their own, own types carry a standard reference per field (points 4 and 6).
mod support;
use support::{edit, run, SHIFT, TASKS};

fn has(src: &str, text: &str) -> bool {
    let r = run(src, true);
    r.errors.iter().any(|e| e.to_string() == text)
}

#[test]
fn a_module_type_with_its_own_standard_or_field_belongs_to_the_module() {
    let with = |extra: &str| {
        edit(
            TASKS,
            "module = \"fs-todo\"\n",
            &format!("module = \"fs-todo\"\n{extra}\n"),
        )
    };
    assert!(has(
        &with("standard = \"https://x.org/a#b\""),
        "owned-by-module{type=Task, key=standard}"
    ));
    assert!(has(
        &with("envelope = \"Note\""),
        "owned-by-module{type=Task, key=envelope}"
    ));
    assert!(has(
        &with("media_type = \"a/b\""),
        "owned-by-module{type=Task, key=media_type}"
    ));
    assert!(has(
        &with("label = { en = \"a\", de = \"b\" }"),
        "owned-by-module{type=Task, key=label}"
    ));
    let field = edit(TASKS, "[[ruleset]]", "[[type.field]]\nname = \"x\"\nbasic = \"text\"\nstandard = \"https://x.org/a#b\"\nlabel = { en = \"a\", de = \"b\" }\n\n[[ruleset]]");
    assert!(has(&field, "owned-by-module{type=Task, key=field}"));
}

#[test]
fn a_field_with_a_kit_and_a_basic_type_or_with_neither_is_refused() {
    assert!(has(
        &edit(
            SHIFT,
            "kit = \"timeSpan\"",
            "kit = \"timeSpan\"\nbasic = \"text\""
        ),
        "field-kind-both{field=span}"
    ));
    assert!(has(
        &edit(SHIFT, "kit = \"timeSpan\"\n", ""),
        "field-kind-missing{field=span}"
    ));
}

#[test]
fn a_field_without_a_standard_reference_is_refused() {
    assert!(has(
        &edit(
            SHIFT,
            "standard = \"https://www.rfc-editor.org/rfc/rfc8984#section-5.1\"\n",
            ""
        ),
        "standard-missing{type=Shift, field=span}"
    ));
}

#[test]
fn an_unknown_kit_or_basic_type_is_named() {
    assert!(has(
        &edit(SHIFT, "kit = \"timeSpan\"", "kit = \"ghost\""),
        "kit-unknown{name=ghost}"
    ));
    assert!(has(
        &edit(SHIFT, "kit = \"personRef\"", "basic = \"ghost\""),
        "basic-unknown{name=ghost}"
    ));
}

#[test]
fn an_own_type_needs_its_standard_envelope_media_type_and_label() {
    let missing = |line: &str, field: &str| {
        let src = edit(SHIFT, line, "");
        let e = fs_blueprint::Blueprint::from_toml(&src);
        let r = run(&src, true);
        assert!(e.is_ok(), "{e:?}");
        assert!(
            r.errors
                .iter()
                .any(|e| e.to_string() == format!("field-missing{{table=type, field={field}}}")),
            "{field}: {:?}",
            r.errors
        );
    };
    missing("envelope = \"Event\"\n", "envelope");
    missing(
        "media_type = \"application/vnd.freesynergy.shift\"\n",
        "media_type",
    );
    missing("label = { en = \"Shift\", de = \"Schicht\" }\n", "label");
    missing(
        "standard = \"https://www.rfc-editor.org/rfc/rfc8984#section-2.1\"\n",
        "standard",
    );
}

#[test]
fn forms_of_names_media_types_and_standard_addresses_are_checked() {
    assert!(has(
        &edit(SHIFT, "name = \"Shift\"", "name = \"shift\""),
        "name-case{key=type.name, found=shift}"
    ));
    assert!(has(
        &edit(SHIFT, "name = \"span\"", "name = \"Span\""),
        "name-case{key=type.field.name, found=Span}"
    ));
    assert!(has(
        &edit(SHIFT, "application/vnd.freesynergy.shift", "shift"),
        "media-type-invalid{type=Shift, found=shift}"
    ));
    assert!(has(
        &edit(SHIFT, "rfc8984#section-2.1", "rfc8984"),
        "standard-invalid{type=Shift, found=https://www.rfc-editor.org/rfc/rfc8984}"
    ));
}

#[test]
fn a_word_list_needs_a_choice_field_and_an_existing_list() {
    let word = |basic: &str, words: &str| {
        edit(
            SHIFT,
            "kit = \"personRef\"",
            &format!("basic = \"{basic}\"\nwords = \"{words}\""),
        )
    };
    let with_module = |t: String| {
        edit(
            &t,
            "[[type]]",
            "[[module]]\nname = \"fs-progress\"\nkind = \"standard\"\nversion = \"0.0.1\"\n\n[[type]]",
        )
    };
    assert!(run(&with_module(word("choice", "fs-progress:state")), true).is_ok());
    assert!(has(
        &with_module(word("choice", "fs-progress:ghost")),
        "words-unknown{ref=fs-progress:ghost}"
    ));
    assert!(has(&word("choice", "ghost"), "words-unknown{ref=ghost}"));
    assert!(has(
        &with_module(word("text", "fs-progress:state")),
        "words-unknown{ref=fs-progress:state}"
    ));
}

#[test]
fn a_type_or_field_twice_is_a_duplicate() {
    assert!(has(
        &edit(SHIFT, "name = \"assignee\"", "name = \"span\""),
        "duplicate{table=type.field, name=span}"
    ));
}

#[test]
fn the_label_of_a_type_and_of_a_field_is_a_pair_of_texts() {
    assert!(has(
        &edit(
            SHIFT,
            "label = { en = \"Shift\", de = \"Schicht\" }",
            "label = { en = \"Shift\" }"
        ),
        "text-pair{key=type.Shift.label, missing=de}"
    ));
    assert!(has(
        &edit(
            SHIFT,
            "label = { en = \"Time\", de = \"Zeit\" }",
            "label = { de = \"Zeit\" }"
        ),
        "text-pair{key=type.Shift.span.label, missing=en}"
    ));
}
