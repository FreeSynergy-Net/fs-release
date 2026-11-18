//! The slots of the templates: a closed list, values from the blueprint, unknown and open slots refused.
mod support;
use fs_blueprint::emit::slot::{fill, Slot, Values};
use fs_blueprint::Blueprint;
use support::synth::blueprint;
use support::{edit, generate, FakeCatalog};

#[test]
fn slots_are_a_closed_list_and_the_values_come_from_the_blueprint() {
    let bp = blueprint(1);
    let v = Values::of(&bp).unwrap();
    assert_eq!(v.get(Slot::Program), "beta-shelf-planner");
    assert_eq!(v.get(Slot::Kebab), "fs-beta-shelf-planner");
    assert_eq!(v.get(Slot::Snake), "fs_beta_shelf_planner");
    assert_eq!(v.get(Slot::Pascal), "BetaShelfPlanner");
    assert_eq!(v.get(Slot::AppId), "net.freesynergy.BetaShelfPlanner");
    assert_eq!(v.get(Slot::Capability), "shelving");
    assert_eq!(v.get(Slot::ContractVersion), "0.7");
    assert_eq!(v.get(Slot::ContractId), "0xb04d258971543402");
    assert_eq!(
        fill("{{ kebab }}/{{snake}}", &v, "p").unwrap(),
        "fs-beta-shelf-planner/fs_beta_shelf_planner"
    );
    assert_eq!(
        fill("a {{nope}} b", &v, "p").unwrap_err().to_string(),
        "slot-unknown{name=nope}"
    );
    assert_eq!(
        fill("a {{kebab", &v, "p").unwrap_err().to_string(),
        "slot-open{path=p}"
    );
    assert_eq!(
        fill("no slots {single}", &v, "p").unwrap(),
        "no slots {single}"
    );
}

#[test]
fn a_slot_value_that_brings_a_new_open_slot_is_refused() {
    let text = edit(
        &support::synth::text(0),
        "name = \"alpha-desk\"",
        "name = \"a{{kebab}}\"",
    );
    let bp = Blueprint::from_toml(&text).unwrap();
    let err = generate(&bp, &FakeCatalog).unwrap_err().to_string();
    assert!(err.starts_with("slot-open"), "{err}");
}
