//! `[[setting]]`: the settings of the program as data (key, type, default, label), written to `packaging/settings.toml`; no widget.
mod support;
use fs_blueprint::emit::artifact::Emitter;
use fs_blueprint::emit::header::untouched;
use fs_blueprint::emit::settings::{Settings, PATH};
use fs_blueprint::Blueprint;
use support::{codes, generate, only, run, FakeCatalog, TASKS};

const TWO: &str = "\n[[setting]]\nkey = \"theme\"\ntype = \"choice\"\ndefault = \"dark\"\nlabel = { en = \"Theme\", de = \"Darstellung\" }\n\n\
                   [[setting]]\nkey = \"title\"\ntype = \"text\"\ndefault = \"A \\\"quoted\\\" title\"\nlabel = { en = \"Title\", de = \"Titel\" }\n";

fn with(extra: &str) -> String {
    format!("{TASKS}{extra}")
}

#[test]
fn the_settings_are_written_as_the_table_of_the_blueprint_and_read_back_the_same() {
    let bp = Blueprint::from_toml(&with(TWO)).unwrap();
    let a = generate(&bp, &FakeCatalog).unwrap();
    let f = a.iter().find(|x| x.path == PATH).expect("settings.toml");
    assert!(untouched(&f.content));
    let t: toml::Table = toml::from_str(&f.content).unwrap();
    let list = t["setting"].as_array().unwrap();
    assert_eq!(list.len(), 2);
    assert_eq!(list[0]["key"].as_str(), Some("theme"));
    assert_eq!(list[0]["type"].as_str(), Some("choice"));
    assert_eq!(list[0]["default"].as_str(), Some("dark"));
    assert_eq!(list[0]["label"]["de"].as_str(), Some("Darstellung"));
    assert_eq!(list[1]["default"].as_str(), Some("A \"quoted\" title"));
}

#[test]
fn a_program_without_settings_gets_no_settings_file() {
    let bp = Blueprint::from_toml(TASKS).unwrap();
    assert!(generate(&bp, &FakeCatalog)
        .unwrap()
        .iter()
        .all(|x| x.path != PATH));
}

#[test]
fn a_setting_key_twice_an_unknown_type_and_a_missing_label_are_findings() {
    assert!(codes(&with(TWO)).is_empty(), "{:?}", codes(&with(TWO)));
    let twice = with(TWO).replace("\"title\"", "\"theme\"");
    assert_eq!(only(&run(&twice, true), "duplicate"), 1);
    let bad_type = with(TWO).replace("type = \"text\"", "type = \"nope\"");
    assert_eq!(only(&run(&bad_type, true), "basic-unknown"), 1);
    let no_de = with(TWO).replace(", de = \"Titel\"", "");
    let r = run(&no_de, true);
    assert_eq!(only(&r, "text-pair"), 1, "{:?}", r.codes());
}

#[test]
fn the_emitter_itself_refuses_a_missing_label_instead_of_writing_an_empty_one() {
    let bp = Blueprint::from_toml(&with(&TWO.replace(", de = \"Titel\"", ""))).unwrap();
    let e = Settings.emit(&bp, &FakeCatalog).unwrap_err();
    assert_eq!(
        e.to_string(),
        "text-pair{key=setting.title.label, missing=de}"
    );
}
