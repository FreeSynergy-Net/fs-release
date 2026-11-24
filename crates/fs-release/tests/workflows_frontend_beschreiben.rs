//! Pattern: none (test). The flow `frontend-beschreiben` (the LAYOUT of a Frontend as describing data, no function, no connection): existing first, the six steps in order, layout and theme apart, binding not written,
//! the check command of the PUBLISHED host, missing components as cards with a crop, the gaps as cards (no script, no Python in the flow), the sheet per program with the honest comparison (no verdict per picture), gaps L1 to L7 with target lanes, the copy block rule. Structure test only (user's depth of checking 24.11.: no mutant chains).

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
        "fehlende-components-melden",
        "host-holen",
        "host-weiche",
        "pruefbefehl",
        "pruefbefehl-again",
        "bild-nebeneinander",
        "bild-waechter",
        "bild-again",
        "vergleich",
        "fremdbilder-pruefen",
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
        && step_body(text, "layout-schreiben").contains("forms/<ansicht>.toml")
        && step_body(text, "layout-schreiben").contains("no data source, no command, no right")
        && step_body(text, "layout-schreiben").contains("never a fake source")
        && step_body(text, "theme-schreiben").contains("style layer of ADR 0074")
        && step_body(text, "theme-schreiben").contains("WITHOUT touching `forms/*.toml`")
        && step_body(text, "frontend-nur-komponenten").contains("gate: opus-ja")
        && step_body(text, "frontend-nur-komponenten-weiche").contains(">= 3 }")
        && step_body(text, "host-holen").contains("gh release download")
        && step_body(text, "host-holen").contains("never a locally built binary")
        && step_body(text, "pruefbefehl").contains("--check --description")
        && step_body(text, "pruefbefehl-again").contains(">= 3")
        && step_body(text, "bild-nebeneinander").contains("--picture")
        && !step_body(text, "bild-nebeneinander").contains("--sample-data")
        && step_body(text, "vergleich").contains("NO verdict of the user per picture")
        && step_body(text, "vergleich").contains("fehlt | anders | gleich")
        && !text.contains("name: picture-acceptance")
        && step_body(text, "bild-nebeneinander").contains("bilder/layout/<programm>/")
        && step_body(text, "host-holen").contains("Gap L7")
        && text.contains("L7 Host/component state fetch")
        && text.contains("TO CLARIFY FIRST")
        && text.contains("NO parallel tree")
        && text.contains("NEED NO HOST")
        && text.contains(".scratch/katalog-plan/bilder/<programm>/vorbild/")
        && step_body(text, "pruefbefehl-again").contains("l5-form-ohne-quelle")
        && step_body(text, "host-weiche").contains("then: bericht")
        && text.contains("Target lane: C1")
        && step_body(text, "fehlende-components-melden").contains("picture crop to lane C")
        && step_body(text, "fehlende-components-melden").contains("never a new `[x]` line")
        && step_body(text, "fehlende-components-melden").contains("Befehl fehlt in Modul X")
        && text.contains("[KOPIERBLOCK-OFFENE-SHELL]")
        && text.contains("NO own script, NO Python")
        && text.contains("ONLY LAYOUT")
        && text.contains("REALITY INSTEAD OF TESTS")
        && text.contains("[FREMDBILDER-NIE-INS-REPO]")
        && step_body(text, "bilder-holen").contains("Writes ONLY to `.scratch`")
        && step_body(text, "bild-nebeneinander")
            .contains("stays LOCAL under `.scratch/katalog-plan/bilder/layout/<programm>/`")
        && step_body(text, "fremdbilder-pruefen").contains("git diff --name-only")
        && step_body(text, "fremdbilder-pruefen").contains("a FINDING")
}

#[test]
fn frontend_beschreiben_is_layout_only_data_with_the_published_host_and_cards_for_the_gaps() {
    assert!(flow_is_sound(&read("frontend-beschreiben")));
}

#[test]
fn the_flow_calls_no_script_and_no_python_and_names_the_commands_of_our_programs() {
    let t = read("frontend-beschreiben");
    assert!(!t.contains("python") && !t.contains(".py\"") && !t.contains("run:\n        shell"));
    assert!(step_body(&t, "pruefbefehl").contains("--check"));
    assert!(t.contains("NO `set -e`, NO `set -u`, NO `set -o pipefail`"));
}

#[test]
fn foreign_pictures_never_go_into_a_repo_in_both_flows() {
    let d = read("dossier");
    assert!(d.contains("[FREMDBILDER-NIE-INS-REPO]"));
    assert!(step_body(&d, "fremdbilder-pruefen").contains("a FINDING"));
    assert!(in_order(
        &d,
        &["baustein-zuordnung-again", "fremdbilder-pruefen", "publish"]
    ));
}
