//! E3-2: the contract text (capnp) of the program's own types: gold, append-only numbers, head line, no output without own types.
mod support;
use fs_blueprint::emit::header::{class_of, untouched};
use fs_blueprint::emit::Class;
use fs_blueprint::Blueprint;
use support::{edit, generate, FakeCatalog, SHIFT, TASKS};

const GOLD: &str = "@0xc04d258971543477;
# Contract 0.1 of schedules: append-only, a field only at the end of its struct.
using Kit = import \"/fs-types/kit.capnp\";

struct Shift {
  span @0 :Kit.TimeSpan;
  assignee @1 :List(Kit.PersonRef);
}
";

fn contract_of(src: &str) -> Option<String> {
    let bp = Blueprint::from_toml(src).unwrap();
    let all = generate(&bp, &FakeCatalog).unwrap();
    let a = all.iter().find(|a| a.path == "schema/schedules.capnp")?;
    assert_eq!(a.class, Class::Generated);
    assert_eq!(class_of(&a.content), Some(Class::Generated));
    assert!(untouched(&a.content));
    Some(a.content.split_once('\n').unwrap().1.to_string())
}

#[test]
fn the_contract_text_is_the_gold() {
    assert_eq!(contract_of(SHIFT).as_deref(), Some(GOLD));
}

#[test]
fn a_new_field_at_the_end_keeps_every_number_before_it() {
    let more = edit(
        SHIFT,
        "\n[frontend]",
        "\n[[type.field]]\nname = \"note\"\nbasic = \"text\"\nstandard = \"https://www.rfc-editor.org/rfc/rfc8984#section-4.2.1\"\nlabel = { en = \"Note\", de = \"Notiz\" }\n\n[frontend]",
    );
    let text = contract_of(&more).unwrap();
    assert!(text.contains(
        "  span @0 :Kit.TimeSpan;\n  assignee @1 :List(Kit.PersonRef);\n  note @2 :Text;\n"
    ));
}

#[test]
fn basic_types_map_to_typed_capnp_types() {
    let with = |basic: &str| {
        edit(
            SHIFT,
            "\n[frontend]",
            &format!("\n[[type.field]]\nname = \"x\"\nbasic = \"{basic}\"\nstandard = \"https://x.org/a#b\"\nlabel = {{ en = \"a\", de = \"b\" }}\n\n[frontend]"),
        )
    };
    for (basic, line) in [
        ("integer", "  x @2 :Int64;\n"),
        ("whole", "  x @2 :Int64;\n"),
        ("flag", "  x @2 :Bool;\n"),
        ("keywords", "  x @2 :List(Text);\n"),
        ("email", "  x @2 :Text;\n"),
    ] {
        assert!(contract_of(&with(basic)).unwrap().contains(line), "{basic}");
    }
}

fn tasks_contract(src: &str) -> Option<String> {
    let bp = Blueprint::from_toml(src).unwrap();
    let all = generate(&bp, &FakeCatalog).unwrap();
    all.iter()
        .find(|a| a.path == "schema/tasks.capnp")
        .map(|a| a.content.split_once('\n').unwrap().1.to_string())
}

#[test]
fn a_program_without_own_types_has_no_contract_text_also_when_it_provides_contracts() {
    // `provides` only NAMES contracts of fs-interfaces (one source): nothing of them is written into the program
    assert!(TASKS.contains("provides = [{ contract"));
    assert_eq!(tasks_contract(TASKS), None);
}

#[test]
fn no_generated_file_carries_an_interface_of_a_provided_contract() {
    let bp = Blueprint::from_toml(TASKS).unwrap();
    for a in generate(&bp, &FakeCatalog).unwrap() {
        assert!(!a.content.contains("interface "), "{}", a.path);
    }
    let bp = Blueprint::from_toml(SHIFT).unwrap();
    for a in generate(&bp, &FakeCatalog).unwrap() {
        assert!(!a.content.contains("interface "), "{}", a.path);
    }
}
