//! The contract text is append-only against the one already in the target: inserting, reordering, deleting and changing a type are findings, appending is not.
mod support;
use fs_blueprint::emit::append_only::broken;
use fs_blueprint::emit::write::write;
use support::synth::MemStore;
use support::{edit, generate, FakeCatalog, SHIFT, TASKS};

const NOTE: &str = "[[type.field]]\nname = \"note\"\nbasic = \"text\"\nstandard = \"https://x.org/a#b\"\nlabel = { en = \"Note\", de = \"Notiz\" }\n\n";

fn put(src: &str) -> MemStore {
    let mut store = MemStore::default();
    again(&mut store, src).unwrap();
    store
}

fn again(store: &mut MemStore, src: &str) -> Result<(), Vec<String>> {
    let bp = fs_blueprint::Blueprint::from_toml(src).unwrap();
    let out = generate(&bp, &FakeCatalog).unwrap();
    write(&out, store)
        .map(|_| ())
        .map_err(|e| e.iter().map(ToString::to_string).collect())
}

fn refused(store: &mut MemStore, src: &str) -> String {
    let before = store.files.clone();
    let err = again(store, src).unwrap_err();
    assert_eq!(err.len(), 1, "{err:?}");
    // nothing was written
    assert_eq!(store.files, before);
    err[0].clone()
}

#[test]
fn a_field_inserted_in_the_middle_is_a_finding_and_nothing_is_written() {
    let mut store = put(SHIFT);
    let middle = edit(
        SHIFT,
        "[[type.field]]\nname = \"assignee\"",
        &format!("{NOTE}[[type.field]]\nname = \"assignee\""),
    );
    let err = refused(&mut store, &middle);
    assert!(
        err.starts_with("contract-append-only{path=schema/schedules.capnp, what=struct Shift: assignee @1 :List(Kit.PersonRef); became note @1 :Text;"),
        "{err}"
    );
}

#[test]
fn a_field_appended_at_the_end_is_not_a_finding() {
    let mut store = put(SHIFT);
    let more = edit(SHIFT, "\n[frontend]", &format!("\n{NOTE}[frontend]"));
    again(&mut store, &more).unwrap();
    // and again with the same text
    again(&mut store, &more).unwrap();
}

#[test]
fn a_deleted_field_and_a_changed_type_are_findings() {
    let more = edit(SHIFT, "\n[frontend]", &format!("\n{NOTE}[frontend]"));
    let mut store = put(&more);
    let err = refused(&mut store, SHIFT);
    assert!(err.contains("note @2 :Text; removed"), "{err}");
    let mut store = put(&more);
    let int = edit(&more, "basic = \"text\"", "basic = \"integer\"");
    let err = refused(&mut store, &int);
    assert!(
        err.contains("note @2 :Text; became note @2 :Int64;"),
        "{err}"
    );
}

#[test]
fn a_changed_file_id_is_a_finding() {
    let mut store = put(SHIFT);
    let err = refused(
        &mut store,
        &edit(SHIFT, "0xc04d258971543477", "0xc04d258971543478"),
    );
    assert!(err.contains("file id changed"), "{err}");
}

#[test]
fn an_interface_moved_by_a_method_put_in_front_is_a_finding() {
    let mut store = put(TASKS);
    let moved = edit(TASKS, "provides = [", "provides = [\"board.move\", ");
    let err = refused(&mut store, &moved);
    assert!(
        err.contains("interface Dependency moved or removed"),
        "{err}"
    );
    // a method of an interface that is put in front of the others of it: the numbers move
    let mut store = put(TASKS);
    let front = edit(
        TASKS,
        "[\"dependency.evaluate\", ",
        "[\"dependency.order\", \"dependency.evaluate\", ",
    );
    let err = refused(&mut store, &front);
    assert!(
        err.contains("evaluate @0 () -> (); became order @0 () -> ();"),
        "{err}"
    );
}

#[test]
fn a_struct_added_at_the_end_next_to_an_interface_is_fine_and_a_struct_in_front_is_not() {
    let old = "@0x1;\n\nstruct A {\n  x @0 :Text;\n}\n\nstruct B {\n  y @0 :Text;\n}\n\ninterface I {\n  m @0 () -> ();\n}\n";
    let appended = old.replace(
        "\ninterface I",
        "\nstruct C {\n  z @0 :Text;\n}\n\ninterface I",
    );
    assert_eq!(broken(old, &appended), None);
    let front = old.replace(
        "\nstruct A {",
        "\nstruct C {\n  z @0 :Text;\n}\n\nstruct A {",
    );
    assert_eq!(
        broken(old, &front).as_deref(),
        Some("struct A moved or removed")
    );
    let swapped = old
        .replace("struct A", "struct Q")
        .replace("struct B", "struct A")
        .replace("struct Q", "struct B");
    assert_eq!(
        broken(old, &swapped).as_deref(),
        Some("struct A moved or removed")
    );
}

#[test]
fn a_hand_changed_contract_is_still_generated_changed_not_append_only() {
    let mut store = put(SHIFT);
    store
        .files
        .get_mut("schema/schedules.capnp")
        .unwrap()
        .push_str("# hand\n");
    let err = refused(&mut store, SHIFT);
    assert_eq!(err, "generated-changed{path=schema/schedules.capnp}");
}
