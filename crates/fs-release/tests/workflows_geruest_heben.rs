//! Pattern: none (test). Workflow maintenance run 9: the flow `geruest-heben`: order, check steps, no build before the unclear list, `talk` is a human step with gate user.
//! Each rule has a counter-test on a changed copy.

mod support;
use support::flowcheck::{again_ok, broken, starts_with_card, with_checks};
use support::workflows::{in_order, read, step_body, step_names};

fn heben_ok(t: &str) -> bool {
    let order = [
        "vorhanden-pruefen",
        "karte",
        "lage",
        "trait-entwurf",
        "trait-review",
        "crate-geruest",
        "doku-hilfe",
        "einsortieren",
        "unklar-liste",
        "talk-vorbereiten",
        "talk",
        "bau-buendel",
        "bericht",
        "eskalation",
    ];
    let talk = step_body(t, "talk");
    let sort = step_body(t, "einsortieren");
    // the steps of THIS flow before the bundled build do not compile (no cargo, no build slot); the trait review is an own step and does not call `trait-delta`
    let before = step_names(t)
        .iter()
        .take_while(|n| *n != "bau-buendel")
        .map(|n| step_body(t, n))
        .collect::<String>();
    in_order(t, &order)
        && starts_with_card(t)
        && with_checks(
            t,
            &[
                ("lage", "lage"),
                ("trait-entwurf", "trait-entwurf"),
                ("crate-geruest", "crate-geruest"),
                ("einsortieren", "einsortieren"),
                ("talk-vorbereiten", "talk-vorbereiten"),
                ("unklar-liste", "unklar-liste"),
                ("doku-hilfe", "doku-hilfe"),
            ],
        )
        && step_body(t, "trait-review").contains("call: trait-review-opus:")
        && step_body(t, "trait-review").contains("freesynergy.net:gate: opus-ja")
        && !step_body(t, "trait-review").contains("name: trait-delta")
        && step_body(t, "schicht-weiche").contains(".input.schicht_fertig")
        && step_body(t, "bericht").contains("freesynergy.net:actor: machine")
        && again_ok(t, "trait-review-again", "trait-entwurf", "eskalation")
        && sort.contains("Zunächst NICHT kompilieren (spart Slots)")
        && sort.contains("Herkunft: <repo>@<kopf>:<pfad>")
        && step_body(t, "trait-review").contains("NOT the flow `trait-delta`")
        && !sort.contains("cargo")
        && !before.contains("cargo")
        && !before.contains("build-slot")
        && talk.contains("freesynergy.net:actor: human")
        && talk.contains("freesynergy.net:gate: human")
        && step_body(t, "unklar-liste-check").contains("UNKLAR-<crate>.md")
        && step_body(t, "bau-buendel").contains("build-slot.sh")
        && step_body(t, "bau-buendel").contains("GATE-PIPEFAIL")
        && step_body(t, "schicht-weiche").contains("bau-buendel")
        && again_ok(t, "talk-again", "talk", "eskalation")
}

#[test]
fn geruest_heben_is_sound() {
    assert!(heben_ok(&read("geruest-heben")));
}

#[test]
fn counter_geruest_heben_a_build_before_the_unclear_list_a_machine_talk_or_a_skipped_check_are_caught(
) {
    let t = read("geruest-heben");
    assert!(broken(
        heben_ok,
        &t.replace(
            "no compile, no test, no build call in this step",
            "run cargo test in this step"
        )
    ));
    assert!(broken(
        heben_ok,
        &t.replace("Zunächst NICHT kompilieren (spart Slots)", "")
    ));
    assert!(broken(
        heben_ok,
        &t.replace("call: einsortieren-ai", "call: x\n      run: build-slot.sh")
    ));
    let talk = step_body(&t, "talk");
    assert!(broken(
        heben_ok,
        &t.replace(&talk, &talk.replace("gate: human", "gate: none"))
    ));
    assert!(broken(
        heben_ok,
        &t.replace(&talk, &talk.replace("actor: human", "actor: ai"))
    ));
    assert!(broken(
        heben_ok,
        &t.replace("  - einsortieren-check:", "  - einsortieren-pruefen:")
    ));
    assert!(broken(
        heben_ok,
        &t.replace("then: trait-entwurf\n", "then: continue\n")
    ));
    assert!(broken(
        heben_ok,
        &t.replace("Herkunft: <repo>@<kopf>:<pfad>", "origin")
    ));
}
