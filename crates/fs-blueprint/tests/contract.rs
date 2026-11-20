//! E3-2: the contract text (capnp) of the program's own types: gold, append-only numbers, head line, no output without own types.
mod support;
use fs_blueprint::emit::header::{class_of, untouched};
use fs_blueprint::emit::Class;
use fs_blueprint::Blueprint;
use support::{edit, generate, FakeCatalog, SHIFT, TASKS};

const GOLD: &str = "@0xc04d258971543477;
# Contract 0.1 of schedules: append-only, a field or method only at the end of its struct or interface.
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
fn a_program_without_own_types_and_without_provides_has_no_contract_text() {
    let none = edit(
        TASKS,
        "provides = [\"dependency.evaluate\", \"dependency.cycle-check\"]\n",
        "",
    );
    assert_eq!(tasks_contract(&none), None);
}

#[test]
fn provides_gives_one_interface_with_the_methods_as_declared_numbered_in_order() {
    let want = "@0xc04d258971543476;\n# Contract 0.1 of tasks: append-only, a field or method only at the end of its struct or interface.\n\n\
                interface Dependency {\n  evaluate @0 () -> ();\n  cycleCheck @1 () -> ();\n}\n";
    assert_eq!(tasks_contract(TASKS).as_deref(), Some(want));
}

#[test]
fn a_method_added_at_the_end_keeps_the_numbers_before_it_and_a_second_interface_follows() {
    let more = edit(
        TASKS,
        "\"dependency.cycle-check\"]",
        "\"dependency.cycle-check\", \"board.move\", \"dependency.order\"]",
    );
    let text = tasks_contract(&more).unwrap();
    assert!(text.contains(
        "interface Dependency {\n  evaluate @0 () -> ();\n  cycleCheck @1 () -> ();\n  order @2 () -> ();\n}\n"
    ));
    assert!(text.contains("interface Board {\n  move @0 () -> ();\n}\n"));
    assert!(text.find("interface Dependency") < text.find("interface Board"));
}
