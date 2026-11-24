//! Pattern: none (test). The flow `frontend-beschreiben` (the LAYOUT of a Frontend as describing data, no function, no connection): existing first, the six steps in order, layout and theme apart, binding not written,
//! the check command of the PUBLISHED host, missing components as cards with a crop, the gaps as cards (no script, no Python in the flow), the user's verdict, the copy block rule. Counter-tests on changed copies.

mod support;
use support::workflows::{in_order, read, step_body};

fn flow_is_sound(text: &str) -> bool {
    let order = [
        "vorhanden-pruefen",
        "vorhanden-pruefen-check",
        "vorhanden-pruefen-again",
        "vorhanden-weiche",
        "bilder-holen",
        "bilder-weiche",
        "dossier",
        "anordnung-lesen",
        "layout-schreiben",
        "theme-schreiben",
        "frontend-nur-komponenten",
        "frontend-nur-komponenten-weiche",
        "frontend-befund-talk",
        "host-holen",
        "host-weiche",
        "pruefbefehl",
        "pruefbefehl-again",
        "bild-nebeneinander",
        "bild-waechter",
        "bild-again",
        "abnahme",
        "abnahme-weiche",
        "fehlende-components-melden",
        "bericht",
        "abbruch",
        "eskalation",
    ];
    in_order(text, &order)
        && step_body(text, "vorhanden-pruefen").contains("MANDATORY FIRST STEP")
        && step_body(text, "vorhanden-pruefen-again").contains("then: eskalation")
        && step_body(text, "vorhanden-weiche").contains("$context.befund.erweitern")
        && step_body(text, "vorhanden-weiche")
            .contains("ohne-ergebnis:\n            then: eskalation")
        && step_body(text, "bilder-holen").contains("fs-capture shot --title")
        && step_body(text, "bilder-weiche").contains("then: dossier")
        && step_body(text, "bilder-weiche").contains("ohne-ergebnis:\n            then: eskalation")
        && step_body(text, "dossier").contains("name: dossier")
        && step_body(text, "dossier").contains("then: bilder-holen")
        && step_body(text, "anordnung-lesen").contains("ARRANGEMENT")
        && step_body(text, "layout-schreiben").contains("beschreibung/layout/")
        && step_body(text, "layout-schreiben").contains("no data source, no command, no right")
        && step_body(text, "layout-schreiben").contains("`bindung/` stays empty")
        && step_body(text, "theme-schreiben").contains("beschreibung/theme/")
        && step_body(text, "theme-schreiben").contains("WITHOUT touching `layout/`")
        && step_body(text, "frontend-nur-komponenten").contains("gate: opus-ja")
        && step_body(text, "frontend-nur-komponenten-weiche").contains(">= 3 }")
        && step_body(text, "host-holen").contains("gh release download")
        && step_body(text, "host-holen").contains("never a locally built binary")
        && step_body(text, "pruefbefehl").contains("--check --description")
        && step_body(text, "pruefbefehl-again").contains(">= 3")
        && step_body(text, "bild-nebeneinander").contains("--picture")
        && !step_body(text, "bild-nebeneinander").contains("--sample-data")
        && step_body(text, "abnahme").contains("name: picture-acceptance")
        && step_body(text, "abnahme").contains("freesynergy.net:gate: human")
        && step_body(text, "abnahme-weiche").contains("$context.verdict.accepted == true")
        && step_body(text, "fehlende-components-melden").contains("picture crop to lane C")
        && step_body(text, "fehlende-components-melden").contains("never a new `[x]` line")
        && step_body(text, "fehlende-components-melden").contains("Befehl fehlt in Modul X")
        && text.contains("[KOPIERBLOCK-OFFENE-SHELL]")
        && text.contains("NO own script, NO Python")
        && text.contains("ONLY LAYOUT")
}

#[test]
fn frontend_beschreiben_is_layout_only_data_with_the_published_host_and_cards_for_the_gaps() {
    assert!(flow_is_sound(&read("frontend-beschreiben")));
}

#[test]
fn frontend_beschreiben_is_red_without_each_rule() {
    let weiche = "bilder-weiche:\n      switch:\n        - runden:\n            when: '${ $context.vorbild.bilder_fehlen == true and ($context.vorbild.runde // 0) >= 2 }'\n            then: eskalation\n        - fehlen:\n            when: '${ $context.vorbild.bilder_fehlen == true }'\n            then: dossier";
    for (needle, what, edit) in [
        (
            "MANDATORY FIRST STEP",
            "no existing-first step",
            ("MANDATORY FIRST STEP", "FIRST"),
        ),
        (
            "then: dossier",
            "missing pictures are not fetched first",
            (
                weiche,
                &weiche.replace("then: dossier", "then: anordnung-lesen"),
            ),
        ),
        (
            "ARRANGEMENT",
            "no arrangement step",
            ("ARRANGEMENT", "list"),
        ),
        (
            "no data source, no command, no right",
            "the layout may carry a binding",
            (
                "no data source, no command, no right",
                "a source may be named",
            ),
        ),
        (
            "`bindung/` stays empty",
            "the flow may write the binding",
            ("`bindung/` stays empty", "`bindung/` is written"),
        ),
        (
            "WITHOUT touching `layout/`",
            "theme not a layer of its own",
            ("WITHOUT touching `layout/`", "together with `layout/`"),
        ),
        (
            "gate: opus-ja",
            "no Opus review of the description",
            (
                "freesynergy.net:gate: opus-ja",
                "freesynergy.net:gate: none",
            ),
        ),
        (
            "gh release download",
            "no published host",
            ("gh release download", "cargo build"),
        ),
        (
            "never a locally built binary",
            "a local binary may be used",
            ("never a locally built binary", "a local binary"),
        ),
        (
            "--check --description",
            "no check command",
            ("--check --description", "--version"),
        ),
        (
            "--picture",
            "no picture of the layout",
            ("--picture <workdir>", "--no-picture <workdir>"),
        ),
        (
            "name: picture-acceptance",
            "no user verdict flow",
            ("name: picture-acceptance", "name: other"),
        ),
        (
            "picture crop to lane C",
            "missing components are not carded with a crop",
            ("picture crop to lane C", "note"),
        ),
        (
            "never a new `[x]` line",
            "a card may be a ticked line",
            ("never a new `[x]` line", "also a ticked line"),
        ),
        (
            "Befehl fehlt in Modul X",
            "gaps not carried as cards",
            ("Befehl fehlt in Modul X", "gap"),
        ),
        (
            "NO own script, NO Python",
            "scripts allowed in the flow",
            ("NO own script, NO Python", "scripts are fine"),
        ),
        (
            "ONLY LAYOUT",
            "function allowed",
            ("ONLY LAYOUT", "LAYOUT and data"),
        ),
        (
            "[KOPIERBLOCK-OFFENE-SHELL]",
            "no copy block rule",
            ("[KOPIERBLOCK-OFFENE-SHELL]", "[X]"),
        ),
    ] {
        let t = read("frontend-beschreiben").replace(edit.0, edit.1);
        assert!(!flow_is_sound(&t), "{needle}: {what}");
    }
    // example data are no duty any more: a sample-data word in the picture step is red
    let t = read("frontend-beschreiben")
        .replace("--picture <workdir>", "--sample-data --picture <workdir>");
    assert!(
        !flow_is_sound(&t),
        "--sample-data: example source is no duty"
    );
}

#[test]
fn the_flow_calls_no_script_and_no_python_and_names_the_commands_of_our_programs() {
    let t = read("frontend-beschreiben");
    assert!(!t.contains("python") && !t.contains(".py\"") && !t.contains("run:\n        shell"));
    assert!(step_body(&t, "pruefbefehl").contains("--check"));
    assert!(t.contains("NO `set -e`, NO `set -u`, NO `set -o pipefail`"));
}
