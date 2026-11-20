//! `[[help]]`: one text per topic in both languages, written to `packaging/help.toml`; a missing text stays a visible finding.
mod support;
use fs_blueprint::emit::artifact::Emitter;
use fs_blueprint::emit::header::untouched;
use fs_blueprint::emit::help::{HelpTexts, PATH};
use fs_blueprint::Blueprint;
use support::{generate, only, run, FakeCatalog, TASKS};

const TWO: &str = "\n[[help]]\ntopic = \"start\"\ntext = { en = \"Press start.\", de = \"Start drücken.\" }\n\n\
                   [[help]]\ntopic = \"boards\"\ntext = { en = \"A board has {$count} cards.\", de = \"Ein Brett hat {$count} Karten.\" }\n";

fn with(extra: &str) -> String {
    format!("{TASKS}{extra}")
}

#[test]
fn the_help_texts_are_written_per_topic_in_both_languages() {
    let bp = Blueprint::from_toml(&with(TWO)).unwrap();
    let a = generate(&bp, &FakeCatalog).unwrap();
    let f = a.iter().find(|x| x.path == PATH).expect("help.toml");
    assert!(untouched(&f.content));
    let t: toml::Table = toml::from_str(&f.content).unwrap();
    let list = t["help"].as_array().unwrap();
    assert_eq!(list.len(), 2);
    assert_eq!(list[0]["topic"].as_str(), Some("start"));
    assert_eq!(list[0]["text"]["de"].as_str(), Some("Start drücken."));
    assert_eq!(
        list[1]["text"]["en"].as_str(),
        Some("A board has {$count} cards.")
    );
}

#[test]
fn a_program_without_help_gets_no_help_file() {
    let bp = Blueprint::from_toml(TASKS).unwrap();
    assert!(generate(&bp, &FakeCatalog)
        .unwrap()
        .iter()
        .all(|x| x.path != PATH));
}

#[test]
fn a_text_missing_in_one_language_is_a_finding_in_the_check_and_in_the_generator_never_a_silent_gap(
) {
    let no_de = with(&TWO.replace(", de = \"Start drücken.\"", ""));
    let r = run(&no_de, true);
    assert_eq!(only(&r, "text-pair"), 1, "{:?}", r.codes());
    let bp = Blueprint::from_toml(&no_de).unwrap();
    assert_eq!(
        generate(&bp, &FakeCatalog).unwrap_err().to_string(),
        "text-pair{key=help.start.text, missing=de}"
    );
    assert_eq!(
        HelpTexts.emit(&bp, &FakeCatalog).unwrap_err().to_string(),
        "text-pair{key=help.start.text, missing=de}"
    );
}

#[test]
fn a_topic_twice_and_texts_with_different_variables_are_findings() {
    let twice = with(&TWO.replace("\"boards\"", "\"start\""));
    assert_eq!(only(&run(&twice, true), "duplicate"), 1);
    let vars = with(&TWO.replace("{$count} Karten", "{$other} Karten"));
    assert_eq!(only(&run(&vars, true), "text-vars"), 1);
}
