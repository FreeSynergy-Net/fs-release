//! E3-1: the type model (`[[type]]`, `[[type.field]]`) is read as written, in file order; types of modules are not the program's own.
mod support;
use fs_blueprint::Blueprint;
use support::{SHIFT, TASKS};

#[test]
fn own_types_and_their_fields_come_in_file_order() {
    let bp = Blueprint::from_toml(SHIFT).unwrap();
    let types: Vec<_> = bp.own_types().collect();
    assert_eq!(types.len(), 1);
    assert_eq!(types[0].name, "Shift");
    let fields: Vec<_> = types[0]
        .field
        .iter()
        .map(|f| (f.name.as_str(), f.kit.as_deref(), f.multiple))
        .collect();
    assert_eq!(
        fields,
        [
            ("span", Some("timeSpan"), false),
            ("assignee", Some("personRef"), true)
        ]
    );
}

#[test]
fn a_type_of_a_module_is_not_an_own_type() {
    let bp = Blueprint::from_toml(TASKS).unwrap();
    assert_eq!(bp.types.len(), 1);
    assert_eq!(bp.own_types().count(), 0);
}
