//! Pattern: none (test). Workflow maintenance run 9: the flow `ausnahme-gespraech` (no agent decides an exception alone, no `[[exception]]` without `talk`), the step `doku-hilfe`
//! and the name note for `geruest-lift`. Each rule has a counter-test on a changed copy.

mod support;
use support::flowcheck::{broken, starts_with_card, with_checks};
use support::workflows::{in_order, read, step_body, step_names};

fn exception_ok(t: &str) -> bool {
    let order = [
        "karte",
        "vorbereiten",
        "gespraech",
        "gespraech-weiche",
        "modul-anlegen",
        "eintragen",
        "bericht",
        "eskalation",
    ];
    let talk = step_body(t, "gespraech");
    let weiche = step_body(t, "gespraech-weiche");
    in_order(t, &order)
        && starts_with_card(t)
        && with_checks(t, &[("vorbereiten", "vorbereiten"), ("eintragen", "eintragen")])
        && talk.contains("freesynergy.net:actor: human")
        && talk.contains("freesynergy.net:gate: user")
        && talk.contains("call: talk:")
        && weiche.contains("entscheidung == null")
        && weiche.contains("then: eskalation")
        && step_body(t, "modul-anlegen").contains("name: modul-neu")
        && step_body(t, "eintragen").contains("talk: ${ $context.protokoll.pfad }")
        && step_body(t, "eintragen-check").contains("without `talk` is red")
        && step_body(t, "vorbereiten-check").contains("at least 2 alternatives")
        // no step but the conversation and the sort-in decides: no AI or machine step of the flow is a gate `user`
        && step_names(t).iter().all(|n| n == "gespraech" || !step_body(t, n).contains("gate: user"))
}

#[test]
fn ausnahme_gespraech_is_sound_and_no_agent_decides_alone() {
    assert!(exception_ok(&read("ausnahme-gespraech")));
}

#[test]
fn counter_ausnahme_gespraech_agent_talk_missing_talk_and_decision_less_run_are_caught() {
    let t = read("ausnahme-gespraech");
    let talk = step_body(&t, "gespraech");
    assert!(broken(
        exception_ok,
        &t.replace(&talk, &talk.replace("actor: human", "actor: ai"))
    ));
    assert!(broken(
        exception_ok,
        &t.replace(&talk, &talk.replace("gate: user", "gate: none"))
    ));
    assert!(broken(
        exception_ok,
        &t.replace("talk: ${ $context.protokoll.pfad }", "")
    ));
    assert!(broken(
        exception_ok,
        &t.replace("without `talk` is red", "is fine")
    ));
    assert!(broken(
        exception_ok,
        &t.replace("entscheidung == null", "entscheidung == \"x\"")
    ));
    assert!(broken(
        exception_ok,
        &t.replace("  - gespraech-weiche:", "  - gespraech-w:")
    ));
}

fn help_ok(t: &str) -> bool {
    let b = step_body(t, "doku-hilfe");
    in_order(t, &["doku-hilfe", "doku-hilfe-check", "doku-hilfe-again"])
        && b.contains("programmseite: ${ .input.programmseite }")
        && b.contains("hilfe_marker")
        && b.contains("docs/en")
        && b.contains("docs/de")
        && b.contains("one source")
        && step_body(t, "doku-hilfe-check").contains("BOTH languages")
        && step_body(t, "doku-hilfe-again").contains(">= 3")
}

#[test]
fn doku_hilfe_stands_in_modul_neu_geruest_heben_and_grundfassung_after_its_anchor() {
    for f in ["modul-neu", "geruest-heben", "grundfassung"] {
        assert!(help_ok(&read(f)), "{f}");
    }
    assert!(in_order(
        &read("grundfassung"),
        &["nutzer-doku", "doku-hilfe", "aufrufer"]
    ));
    assert!(in_order(
        &read("modul-neu"),
        &["katalog-vorschlag-again", "doku-hilfe", "bericht"]
    ));
    assert!(in_order(
        &read("geruest-heben"),
        &["crate-geruest-again", "doku-hilfe", "einsortieren"]
    ));
}

#[test]
fn counter_doku_hilfe_input_languages_and_single_source_are_caught() {
    let t = read("modul-neu");
    assert!(broken(
        help_ok,
        &t.replace("programmseite: ${ .input.programmseite }", "seite: x")
    ));
    assert!(broken(
        help_ok,
        &t.replace("BOTH languages", "one language")
    ));
    assert!(broken(help_ok, &t.replace("one source", "two sources")));
    assert!(broken(help_ok, &t.replace("`docs/de`", "x")));
    assert!(broken(
        help_ok,
        &t.replace("  - doku-hilfe-check:", "  - doku-hilfe-x:")
    ));
}

#[test]
fn the_old_lift_flows_keep_their_names_and_the_readme_names_geruest_lift_as_the_future_name() {
    // the plan renames `lift` / `lift-v1plus` to `geruest-lift` (plan section c2); no rename here, only the note
    assert!(!support::workflows::FILES.contains(&"geruest-lift"));
    let readme =
        std::fs::read_to_string(support::workflows::root().join("workflows/README.md")).unwrap();
    assert!(readme.matches("geruest-lift").count() >= 2, "de + en note");
}
