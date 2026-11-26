//! Pattern: none (test). `picture-acceptance`: rule [EINTRITT-SYSTEM] - the entry preconditions come before every picture; a red or missing precondition takes no picture.

mod support;
use support::workflows::{read, step_body, step_names};

#[test]
fn picture_acceptance_stops_before_any_picture_when_an_entry_precondition_is_red() {
    let text = read("picture-acceptance");
    let names = step_names(&text);
    let at = |n: &str| {
        names
            .iter()
            .position(|x| x == n)
            .unwrap_or_else(|| panic!("no step {n}"))
    };
    assert!(
        at("vorbedingungen") < at("vorbedingungen-weiche")
            && at("vorbedingungen-weiche") < at("menue-bilder")
    );
    let weiche = step_body(&text, "vorbedingungen-weiche");
    assert!(weiche.contains("then: vorbedingung-fehlt") && !weiche.contains("then: pictures"));
    let step = step_body(&text, "vorbedingungen");
    for k in [
        "freesynergy-filesystem",
        "--check-system",
        "format 3 and 4",
        "red=0",
        "`repos`",
        "`person`",
    ] {
        assert!(
            step.contains(k) || text.contains(k),
            "precondition names {k}"
        );
    }
    assert!(step_body(&text, "vorbedingung-fehlt").contains("no picture is taken"));
}
