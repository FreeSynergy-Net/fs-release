//! Nested types and references (M1): the fs-forms example Survey / Question / Choice / Condition through check, tables and contract.
mod support;
use fs_blueprint::emit::append_only::broken;
use fs_blueprint::Blueprint;
use support::{codes, edit, generate, FakeCatalog};

const SURVEY: &str = include_str!("data/survey.toml");

fn file(src: &str, ext: &str) -> String {
    let bp = Blueprint::from_toml(src).unwrap();
    let all = generate(&bp, &FakeCatalog).unwrap();
    all.iter()
        .find(|a| a.path.ends_with(ext))
        .unwrap()
        .content
        .clone()
}

fn field(name: &str, reference: &str) -> String {
    format!("[[type.field]]\nname = \"{name}\"\nreference = \"{reference}\"\nmultiple = true\nstandard = \"https://x.org/a#b\"\nlabel = {{ en = \"a\", de = \"b\" }}\n\n")
}

#[test]
fn the_example_passes_the_check_and_a_nested_only_type_needs_no_envelope() {
    assert_eq!(codes(SURVEY), Vec::<&str>::new());
}

#[test]
fn a_nested_type_is_a_child_table_and_a_reference_list_a_link_table() {
    let text = file(SURVEY, ".sql");
    assert!(text.contains("CREATE TABLE \"question_choices\" (\n  \"id\" INTEGER PRIMARY KEY,\n  \"owner\" INTEGER NOT NULL REFERENCES \"question\"(\"id\"),\n  \"pos\" INTEGER NOT NULL,\n  \"key\" TEXT,\n  \"label\" TEXT\n);"), "{text}");
    // a single nested value: at most one child per owner
    assert!(
        text.contains("CREATE TABLE \"question_condition\" ("),
        "{text}"
    );
    assert!(
        text.contains("  \"equals\" TEXT,\n  UNIQUE (\"owner\")\n);"),
        "{text}"
    );
    assert_eq!(text.matches("UNIQUE").count(), 1, "{text}");
    assert!(
        text.contains("\"question\" INTEGER REFERENCES \"question\"(\"id\")"),
        "{text}"
    );
    // a reference list: an IRI is text, an own type is the key
    assert!(text.contains("CREATE TABLE \"survey_scope\" ("), "{text}");
    assert!(text.contains("\"target\" TEXT NOT NULL,"), "{text}");
    assert!(
        text.contains("\"target\" INTEGER NOT NULL REFERENCES \"question\"(\"id\"),"),
        "{text}"
    );
    // the nested types have no table of their own name, and there is no JSON
    assert!(!text.contains("CREATE TABLE \"choice\""), "{text}");
    let statements: String = text.lines().filter(|l| !l.starts_with("--")).collect();
    assert!(!statements.to_lowercase().contains("json"), "{text}");
}

#[test]
fn the_contract_nests_the_struct_inside_the_parent_and_lists_references() {
    let text = file(SURVEY, ".capnp");
    assert!(text.contains("  choices @1 :List(Choice);\n  condition @2 :Condition;\n\n  struct Choice {\n    key @0 :Text;\n    label @1 :Text;\n  }\n\n  struct Condition {\n    question @0 :Int64;\n    equals @1 :Text;\n  }\n}\n"), "{text}");
    assert!(
        text.contains("scope @2 :List(Text);") && text.contains("questions @4 :List(Int64);"),
        "{text}"
    );
    assert!(
        !text.contains("\nstruct Choice") && !text.contains("\nstruct Condition"),
        "{text}"
    );
}

#[test]
fn the_kinds_are_exclusive_and_the_targets_must_exist() {
    let both = edit(
        SURVEY,
        "nested = \"Condition\"",
        "nested = \"Condition\"\nbasic = \"text\"",
    );
    assert!(codes(&both).contains(&"field-kind-both"));
    let unknown = edit(SURVEY, "nested = \"Condition\"", "nested = \"Nope\"");
    assert!(codes(&unknown).contains(&"type-unknown"));
    let unknown = edit(
        SURVEY,
        "reference = \"Question\"\nstandard",
        "reference = \"Nope\"\nstandard",
    );
    assert!(codes(&unknown).contains(&"type-unknown"));
}

#[test]
fn a_cycle_and_a_second_parent_are_findings() {
    let cycle = edit(
        SURVEY,
        "name = \"equals\"\nbasic = \"text\"",
        "name = \"equals\"\nnested = \"Question\"",
    );
    assert!(
        codes(&cycle).contains(&"nested-cycle"),
        "{:?}",
        codes(&cycle)
    );
    let twice = edit(
        SURVEY,
        "name = \"title\"\nbasic = \"text\"",
        "name = \"title\"\nnested = \"Choice\"",
    );
    assert!(
        codes(&twice).contains(&"nested-twice"),
        "{:?}",
        codes(&twice)
    );
}

#[test]
fn a_type_that_is_not_nested_still_needs_its_envelope() {
    let top = edit(
        SURVEY,
        "[[type]]\nname = \"Choice\"",
        "[[type]]\nname = \"Loose\"\nlabel = { en = \"a\", de = \"b\" }\n\n[[type]]\nname = \"Choice\"",
    );
    assert!(codes(&top).contains(&"field-missing"), "{:?}", codes(&top));
}

const OLD: &str = "@0x1;\n\nstruct A {\n  x @0 :Text;\n  c @1 :List(C);\n\n  struct C {\n    k @0 :Text;\n  }\n}\n";

#[test]
fn appended_nested_and_reference_fields_are_fine() {
    let more = OLD
        .replace(
            "  c @1 :List(C);\n",
            "  c @1 :List(C);\n  r @2 :List(Int64);\n  d @3 :D;\n",
        )
        .replace(
            "    k @0 :Text;\n  }\n",
            "    k @0 :Text;\n    j @1 :Text;\n  }\n\n  struct D {\n    z @0 :Text;\n  }\n",
        );
    assert_eq!(broken(OLD, &more), None);
    // through the generator: a nested type and a reference appended at the end of Survey
    let extra = format!("{}[[type.field]]\nname = \"extra\"\nnested = \"Extra\"\nstandard = \"https://x.org/a#b\"\nlabel = {{ en = \"a\", de = \"b\" }}\n\n", field("more", "Question"));
    let appended = edit(
        SURVEY,
        "[[type]]\nname = \"Question\"",
        &format!("{extra}[[type]]\nname = \"Question\""),
    )
    .replace(
        "[frontend]",
        "[[type]]\nname = \"Extra\"\nlabel = { en = \"a\", de = \"b\" }\n\n[frontend]",
    );
    assert_eq!(
        broken(&file(SURVEY, ".capnp"), &file(&appended, ".capnp")),
        None
    );
}

#[test]
fn a_nested_or_reference_field_in_the_middle_is_a_break() {
    let mid = OLD.replace("  c @1 :List(C);", "  r @1 :List(Int64);\n  c @2 :List(C);");
    assert_eq!(
        broken(OLD, &mid).as_deref(),
        Some("struct A: c @1 :List(C); became r @1 :List(Int64);")
    );
    // through the generator: a reference in front of `choices`
    let moved = edit(
        SURVEY,
        "[[type.field]]\nname = \"choices\"",
        &format!(
            "{}[[type.field]]\nname = \"choices\"",
            field("extra", "Question")
        ),
    );
    let why = broken(&file(SURVEY, ".capnp"), &file(&moved, ".capnp")).unwrap();
    assert!(
        why.starts_with("struct Question: choices @1 :List(Choice); became extra @1 :List(Int64);"),
        "{why}"
    );
}

#[test]
fn a_removed_nested_struct_and_a_changed_nested_member_are_breaks() {
    let gone = OLD.replace("\n  struct C {\n    k @0 :Text;\n  }\n", "");
    assert_eq!(
        broken(OLD, &gone).as_deref(),
        Some("nested struct A.C removed")
    );
    let renamed = OLD.replace("struct C", "struct E");
    assert_eq!(
        broken(OLD, &renamed).as_deref(),
        Some("nested struct A.C removed")
    );
    let changed = OLD.replace("k @0 :Text;", "k @0 :Int64;");
    assert_eq!(
        broken(OLD, &changed).as_deref(),
        Some("nested struct A.C: k @0 :Text; became k @0 :Int64;")
    );
}
