//! Pattern: none (test). The flow `frontend-beschreiben` (the LAYOUT of a Frontend as describing data, no function, no connection): existing first, the six steps in order, layout and theme apart, binding not written,
//! the check command of the PUBLISHED host, missing components as cards with a crop, the gaps as cards (no script, no Python in the flow), the sheet per program with the honest comparison (no verdict per picture), gaps L1 to L7 with target lanes, the copy block rule. Structure test only (user's depth of checking 24.11.: no mutant chains).

mod support;
use support::workflows::{in_order, read, step_body, step_names};

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
        "fremdbilder-weiche",
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
        && step_body(text, "fremdbilder-pruefen").contains("as: '${ $context + { fremdbilder: . } }'")
        && step_body(text, "fremdbilder-pruefen").contains("git merge-base origin/main HEAD")
        && step_body(text, "fremdbilder-weiche").contains("$context.fremdbilder.runde")
        && step_body(text, "fremdbilder-weiche").contains("then: eskalation")
        && step_body(text, "fremdbilder-weiche").contains("green:\n            when: '${ $context.fremdbilder.red == false }'\n            then: bericht")
        // no way to the report without the picture check
        && step_body(text, "host-weiche").contains("then: fremdbilder-pruefen")
        && step_body(text, "pruefbefehl-again").contains("then: fremdbilder-pruefen")
        && !step_body(text, "host-weiche").contains("then: bericht")
        && !step_body(text, "pruefbefehl-again").contains("then: bericht")
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
fn foreign_picture_check_looks_at_every_commit_and_never_uses_a_deleting_commit() {
    for file in ["frontend-beschreiben", "dossier"] {
        let body = step_body(&read(file), "fremdbilder-pruefen");
        assert!(body.contains("git log --diff-filter=A --name-only --format= $(git merge-base origin/main HEAD)..HEAD"));
        assert!(body.contains("EVERY commit of the range"));
        assert!(body.contains("NEVER a deleting commit"));
        assert!(body.contains("not yet pushed commits"));
        assert!(body.contains("case-insensitive"));
        for ext in ["tif", "avif", "heic", "heif", "jxl", "pdf"] {
            assert!(body.contains(ext), "missing extension {ext}");
        }
        assert!(body.contains("gepusht-red"));
    }
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

/// A word `w` stands in `body` as a whole word (not a part of a longer name).
fn has_word(body: &str, w: &str) -> bool {
    body.match_indices(w).any(|(i, _)| {
        let before = body[..i].chars().next_back();
        let after = body[i + w.len()..].chars().next();
        let edge = |c: Option<char>| c.is_none_or(|c| !(c.is_alphanumeric() || c == '_'));
        edge(before) && edge(after)
    })
}

/// Every switch reads only fields of `$context.<name>.<field>` that the summary of the step which exports `<name>` fixes
/// (the same error as form-neu M1: a field nobody sets makes the branch dead).
#[test]
fn every_switch_reads_only_fields_the_producing_step_fixes() {
    for file in ["frontend-beschreiben", "dossier"] {
        let text = read(file);
        let mut producer = std::collections::HashMap::new();
        for step in step_names(&text) {
            let body = step_body(&text, &step);
            if let Some(rest) = body.split("as: '${ $context + { ").nth(1) {
                producer.insert(rest.split(':').next().unwrap().to_string(), step.clone());
            }
        }
        let mut checked = 0;
        for step in step_names(&text) {
            let body = step_body(&text, &step);
            if !body.trim_start().starts_with("switch:") {
                continue;
            }
            for when in body.lines().filter(|l| l.trim_start().starts_with("when:")) {
                for read_ in when.split("$context.").skip(1) {
                    let mut parts =
                        read_.split(|c: char| !(c.is_alphanumeric() || c == '_' || c == '.'));
                    let mut path = parts.next().unwrap().split('.');
                    let (name, field) = (path.next().unwrap(), path.next());
                    let Some(field) = field else { continue };
                    let prod = producer
                        .get(name)
                        .unwrap_or_else(|| panic!("{file}/{step}: no step exports `{name}`"));
                    assert!(
                        has_word(&step_body(&text, prod), field),
                        "{file}/{step} reads `{name}.{field}`, but `{prod}` does not fix it in its output"
                    );
                    checked += 1;
                }
            }
        }
        assert!(checked >= 5, "{file}: too few reads checked ({checked})");
    }
}
