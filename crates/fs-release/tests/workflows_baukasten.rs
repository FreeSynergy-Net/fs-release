//! Pattern: none (test). Workflow maintenance run 9: the flows `modul-neu` and `bausatz-neu`: existing first, user count, standard, checks with at most two repeats.
//! Each rule has a counter-test on a changed copy.

mod support;
use support::flowcheck::{again_ok, broken, starts_with_card, with_checks};
use support::workflows::{in_order, read, step_body, step_names};

fn stop_on_hit_ok(t: &str) -> bool {
    in_order(
        t,
        &[
            "vorhanden-pruefen",
            "vorhanden-weiche",
            "nutzer-pruefen",
            "nutzer-weiche",
            "standard-pruefen",
        ],
    ) && step_body(t, "vorhanden-weiche").contains("then: abbruch")
        && step_body(t, "nutzer-weiche").contains("then: abbruch")
        && step_names(t).contains(&"abbruch".to_string())
}

fn modul_ok(t: &str) -> bool {
    let order = [
        "vorhanden-pruefen",
        "karte",
        "nutzer-pruefen",
        "standard-pruefen",
        "ort",
        "kommunikation",
        "beschreibung",
        "trait-review",
        "anlegen",
        "katalog-vorschlag",
        "doku-hilfe",
        "verbraucher-probe",
        "bericht",
        "abbruch",
        "eskalation",
    ];
    in_order(t, &order)
        && starts_with_card(t)
        && stop_on_hit_ok(t)
        && with_checks(
            t,
            &[
                ("standard-pruefen", "standard-pruefen"),
                ("beschreibung", "beschreibung"),
                ("anlegen", "anlegen"),
                ("katalog-vorschlag", "katalog-vorschlag"),
                ("doku-hilfe", "doku-hilfe"),
                ("verbraucher-probe", "anlegen"),
            ],
        )
        && step_body(t, "verbraucher-probe").contains("CONSUMER")
        && step_body(t, "verbraucher-probe").contains("without special rules")
        && again_ok(t, "trait-review-again", "beschreibung", "eskalation")
        && step_body(t, "nutzer-pruefen").contains("at least 2 users")
        && step_body(t, "nutzer-pruefen").contains("at least 1")
        && step_body(t, "ort").contains("crates.io")
        && step_body(t, "kommunikation").contains("WebAssembly")
        && step_body(t, "katalog-vorschlag-check").contains("gen_catalog --check")
        && step_body(t, "trait-review").contains("call: trait-review-opus:")
        && !step_body(t, "trait-review").contains("name: trait-delta")
        && step_body(t, "katalog-vorschlag").contains("freesynergy.net:gate: strategie-ok")
}

#[test]
fn modul_neu_is_sound() {
    assert!(modul_ok(&read("modul-neu")));
}

#[test]
fn counter_modul_neu_a_hit_that_does_not_abort_and_a_green_on_missing_result_are_caught() {
    let t = read("modul-neu");
    assert!(broken(
        modul_ok,
        &t.replace("then: abbruch", "then: continue")
    ));
    assert!(broken(modul_ok, &t.replace("at least 2 users", "one user")));
    let again = step_body(&t, "beschreibung-again");
    let open_case = again.replace(
        "then: eskalation\n      metadata",
        "then: continue\n      metadata",
    );
    assert_ne!(open_case, again);
    assert!(broken(modul_ok, &t.replace(&again, &open_case)));
    assert!(broken(
        modul_ok,
        &t.replace("\n  - ort:\n", "\n  - ort-x:\n")
    ));
}

fn kit_ok(t: &str) -> bool {
    let order = [
        "vorhanden-pruefen",
        "karte",
        "nutzer-pruefen",
        "standard-pruefen",
        "beschreibung",
        "erzeugen-plan",
        "freigabe-pruefen",
        "trait-review",
        "bericht",
        "abbruch",
        "eskalation",
    ];
    in_order(t, &order)
        && starts_with_card(t)
        && stop_on_hit_ok(t)
        && with_checks(
            t,
            &[
                ("standard-pruefen", "standard-pruefen"),
                ("beschreibung", "beschreibung"),
                ("erzeugen-plan", "erzeugen-plan"),
                ("freigabe-pruefen", "freigabe-pruefen"),
            ],
        )
        && again_ok(t, "trait-review-again", "beschreibung", "eskalation")
        && step_body(t, "nutzer-pruefen").contains("at least 2 programs")
        && step_body(t, "erzeugen-plan").contains("no JSON or blob column")
        && step_body(t, "freigabe-pruefen").contains("ADR 0098 point 4.6")
}

#[test]
fn bausatz_neu_is_sound() {
    assert!(kit_ok(&read("bausatz-neu")));
}

#[test]
fn counter_bausatz_neu_one_user_enough_or_a_blob_column_allowed_is_caught() {
    let t = read("bausatz-neu");
    assert!(broken(
        kit_ok,
        &t.replace("at least 2 programs", "one program")
    ));
    assert!(broken(
        kit_ok,
        &t.replace("no JSON or blob column", "JSON columns are fine")
    ));
    assert!(broken(
        kit_ok,
        &t.replace("  - freigabe-pruefen-check:", "  - freigabe-x:")
    ));
}
