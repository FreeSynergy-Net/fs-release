//! Schema 2 only (user rule 2026-11-18): schema 1 is refused with a code, no migration, no read path; unknown keys are refused.
mod support;
use fs_blueprint::{Blueprint, Coded};
use support::{codes, edit, run, TASKS};

fn first(src: &str) -> Coded {
    Blueprint::from_toml(src).expect_err("must be refused")
}

#[test]
fn schema_1_is_unsupported_with_found_and_known() {
    let e = first(&edit(TASKS, "schema = 2", "schema = 1"));
    assert_eq!(e.to_string(), "schema-unsupported{found=1, known=2}");
}

#[test]
fn a_schema_1_file_in_the_old_layout_is_unsupported_too_not_a_list_of_unknown_keys() {
    let old = "schema = 1\n\n[program]\nname = \"x\"\n\n[[ruleset]]\nname = \"r\"\n\n[frontend]\nstandard = \"simple\"\n";
    assert_eq!(first(old).code, "schema-unsupported");
    assert!(run(old, true).has("schema-unsupported"));
}

#[test]
fn schema_0_is_unsupported_and_3_is_newer() {
    assert_eq!(
        first(&edit(TASKS, "schema = 2", "schema = 0")).code,
        "schema-unsupported"
    );
    assert_eq!(
        first(&edit(TASKS, "schema = 2", "schema = 3")).to_string(),
        "schema-newer{found=3, known=2}"
    );
}

#[test]
fn a_missing_schema_is_a_missing_field() {
    assert_eq!(
        first(&edit(TASKS, "schema = 2\n", "")).to_string(),
        "field-missing{table=, field=schema}"
    );
}

#[test]
fn a_table_component_is_an_unknown_key() {
    let src = format!("{TASKS}\n[[component]]\nname = \"x\"\n");
    assert_eq!(first(&src).to_string(), "unknown-key{path=component}");
}

#[test]
fn an_expiry_field_in_an_exception_is_an_unknown_key() {
    let src = edit(
        TASKS,
        "talk = \"conventions",
        "expires = \"2027-01-01\"\ntalk = \"conventions",
    );
    assert_eq!(
        first(&src).to_string(),
        "unknown-key{path=exception.expires}"
    );
}

#[test]
fn a_switch_for_mcp_is_an_unknown_key_at_the_top_and_in_a_table() {
    assert_eq!(
        first(&edit(TASKS, "schema = 2\n", "schema = 2\nmcp = true\n")).to_string(),
        "unknown-key{path=mcp}"
    );
    assert_eq!(
        first(&edit(
            TASKS,
            "capability = \"tasks\"\n",
            "capability = \"tasks\"\nmcp = true\n"
        ))
        .to_string(),
        "unknown-key{path=program.mcp}"
    );
    assert_eq!(
        first(&edit(
            TASKS,
            "[frontend]\n",
            "[frontend]\nskeleton-off = \"windows\"\n"
        ))
        .to_string(),
        "unknown-key{path=frontend.skeleton-off}"
    );
}

#[test]
fn an_unknown_language_in_a_text_names_the_text() {
    let src = edit(
        TASKS,
        "summary = { en = \"Tasks, dependencies and boards\",",
        "summary = { fr = \"x\", en = \"Tasks, dependencies and boards\",",
    );
    assert_eq!(
        first(&src).to_string(),
        "unknown-key{path=program.summary.fr}"
    );
}

#[test]
fn a_missing_mandatory_key_names_its_table_and_key() {
    assert_eq!(
        first(&edit(TASKS, "capability = \"tasks\"\n", "")).to_string(),
        "field-missing{table=program, field=capability}"
    );
    assert_eq!(
        first(&edit(TASKS, "default = \"freesynergy\"\n", "")).to_string(),
        "field-missing{table=ruleset, field=default}"
    );
    assert_eq!(
        first(&edit(
            TASKS,
            "talk = \"conventions/talks/2026-11-xx-tasks-secret-guard.md\"\n",
            ""
        ))
        .to_string(),
        "field-missing{table=exception, field=talk}"
    );
    assert_eq!(
        first(&edit(
            TASKS,
            "paths = [\"crates/fs-tasks-core/src/secret_guard.rs\"]\n",
            ""
        ))
        .to_string(),
        "field-missing{table=exception, field=paths}"
    );
}

#[test]
fn a_missing_table_is_a_missing_field_at_the_top() {
    let src = TASKS.split("[frontend]").next().unwrap().to_string() + "[[exception]]\nwhat=\"a\"\nwhy=\"b\"\nchecked=[\"c\"]\nalternatives=[]\ntalk=\"t\"\npaths=[\"p\"]\n";
    assert_eq!(
        first(&src).to_string(),
        "field-missing{table=, field=frontend}"
    );
}

#[test]
fn text_that_is_no_toml_is_a_parse_error_with_a_position() {
    let e = first("schema = 2\n[program\n");
    assert_eq!(e.code, "parse-error");
    assert_eq!(e.param("line"), Some("2"));
}

#[test]
fn a_clean_example_has_no_structure_error_code() {
    for c in codes(TASKS) {
        assert!(c == "version-unknown", "{c}");
    }
}

// User decision 45: there is no key that brings Rust code (or any program-specific text) into the generated `main`.
#[test]
fn there_is_no_key_for_the_main_not_as_table_not_as_key_not_in_program_or_frontend() {
    for (from, to, path) in [
        (
            "schema = 2\n",
            "schema = 2\nmain = \"fn main() {}\"\n",
            "main",
        ),
        (
            "[frontend]\n",
            "[frontend]\nmain = \"x\"\n",
            "frontend.main",
        ),
        (
            "capability = \"tasks\"\n",
            "capability = \"tasks\"\nentry = \"x\"\n",
            "program.entry",
        ),
        (
            "[contract]\n",
            "[contract]\nstart = \"x\"\n",
            "contract.start",
        ),
    ] {
        assert_eq!(
            first(&edit(TASKS, from, to)).to_string(),
            format!("unknown-key{{path={path}}}")
        );
    }
    let src = format!("{TASKS}\n[main]\ncode = \"x\"\n");
    assert_eq!(first(&src).to_string(), "unknown-key{path=main}");
}
