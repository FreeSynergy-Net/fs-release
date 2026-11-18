//! Texts are pairs (de + en, the same variables) and every finding is a code with parameters and has its messages (addendum points 8 and 9).
mod support;
use fs_blueprint::form::variables;
use fs_blueprint::messages::{message, render, MESSAGES};
use fs_blueprint::{Coded, CODES};
use support::{edit, run, SHIFT, TASKS};

#[test]
fn a_text_with_one_language_names_the_missing_one() {
    let r = run(&edit(TASKS, "summary = { en = \"Tasks, dependencies and boards\", de = \"Aufgaben, Abhängigkeiten und Boards\" }", "summary = { en = \"Tasks\" }"), true);
    assert!(r
        .errors
        .iter()
        .any(|e| e.to_string() == "text-pair{key=program.summary, missing=de}"));
    let r = run(
        &edit(
            TASKS,
            "de = \"Aufgaben als",
            "en2 = \"x\"\nde = \"Aufgaben als",
        ),
        true,
    );
    assert!(r.has("text-pair") || r.has("unknown-key") || !r.is_ok());
}

#[test]
fn texts_with_different_variables_are_refused_and_equal_ones_pass() {
    let one = edit(TASKS, "summary = { en = \"Tasks, dependencies and boards\", de = \"Aufgaben, Abhängigkeiten und Boards\" }", "summary = { en = \"Hello {$name}\", de = \"Hallo\" }");
    assert!(run(&one, true)
        .errors
        .iter()
        .any(|e| e.to_string() == "text-vars{key=program.summary}"));
    let same = edit(TASKS, "summary = { en = \"Tasks, dependencies and boards\", de = \"Aufgaben, Abhängigkeiten und Boards\" }", "summary = { en = \"Hello {$name}\", de = \"Hallo {$name}\" }");
    assert!(!run(&same, true).has("text-vars"));
}

#[test]
fn the_variables_of_a_message_are_read() {
    let v = variables("a {$one} b {$two}{$one} {x}");
    assert_eq!(v.into_iter().collect::<Vec<_>>(), ["one", "two"]);
}

#[test]
fn every_code_has_a_message_in_de_and_en_with_the_variables_of_its_parameters() {
    assert_eq!(MESSAGES.len(), CODES.len());
    for (code, names) in CODES {
        for lang in ["en", "de"] {
            let m = message(code, lang).unwrap_or_else(|| panic!("no {lang} message for {code}"));
            let want: Vec<String> = names.iter().map(|n| (*n).to_string()).collect();
            assert_eq!(
                variables(m).into_iter().collect::<Vec<_>>(),
                {
                    let mut w = want.clone();
                    w.sort();
                    w
                },
                "{code} {lang}"
            );
        }
        assert_eq!(
            variables(message(code, "en").unwrap()),
            variables(message(code, "de").unwrap()),
            "{code}"
        );
    }
    assert!(message("no-such-code", "en").is_none() && message("name-rule", "fr").is_none());
}

#[test]
fn a_finding_is_rendered_from_its_parameters() {
    let c = Coded::new("path-overlap")
        .with("path", "a/")
        .with("first", "1")
        .with("second", "2");
    assert_eq!(
        render(&c, "en").unwrap(),
        "The path a/ is covered by exceptions 1 and 2"
    );
    assert_eq!(
        render(&c, "de").unwrap(),
        "Der Pfad a/ wird von den Ausnahmen 1 und 2 gedeckt"
    );
}

#[test]
fn every_finding_of_every_kind_of_fault_is_a_code_of_the_list_with_its_parameters_and_no_sentence()
{
    let faults = [
        edit(TASKS, "schema = 2", "schema = 1"),
        edit(TASKS, "name = \"tasks\"", "name = \"Fs-Tasks\""),
        edit(TASKS, "skeleton = \"0.1.0\"", "skeleton = \"9.9.9\""),
        edit(TASKS, "capability = \"tasks\"", "capability = \"nope\""),
        edit(
            TASKS,
            "categories = [\"Office\", \"ProjectManagement\"]",
            "categories = [\"ProjectManagement\"]",
        ),
        edit(TASKS, "id = \"0xc04d258971543476\"", "id = \"0x1\""),
        edit(TASKS, "version = \"0.1\"", "version = \"1\""),
        edit(TASKS, "\"claim\"]", "\"claim\", \"ghost\"]"),
        edit(
            TASKS,
            "presets = [\"rfc8984\", \"freesynergy\"]",
            "presets = [\"rfc8984\", \"x\"]",
        ),
        edit(TASKS, "default = \"freesynergy\"", "default = \"x\""),
        edit(TASKS, "from = \"fs-planning\"", "from = \"board\""),
        edit(TASKS, "from = \"fs-planning\"", "from = \"nobody\""),
        edit(
            TASKS,
            "layout = \"forms/board.toml\"",
            "layout = \"forms/none.toml\"",
        ),
        edit(TASKS, "covers = [", "covers = [] # ["),
        edit(
            TASKS,
            "paths = [\"crates/fs-tasks-core/src/secret_guard.rs\"]",
            "paths = [\"../x\", \"nope.rs\", \"crates/fs-tasks-core/src/lib.rs\"]",
        ),
        edit(SHIFT, "kit = \"timeSpan\"", "kit = \"ghost\""),
    ];
    let mut seen = 0;
    for src in &faults {
        let r = run(src, false);
        assert!(!r.errors.is_empty(), "no finding for a fault");
        for e in r.errors.iter().chain(&r.warnings) {
            seen += 1;
            assert!(e.conforms(), "{e}");
            assert!(
                !e.code.contains(' ') && e.code.chars().all(|c| c.is_ascii_lowercase() || c == '-'),
                "{e}"
            );
            assert!(
                render(e, "en").is_some() && render(e, "de").is_some(),
                "{e}"
            );
        }
    }
    assert!(seen > 20);
}

#[test]
fn a_code_with_a_wrong_parameter_list_does_not_conform() {
    assert!(!Coded::new("name-rule").conforms());
    assert!(!Coded::new("made-up").conforms());
    assert!(Coded::new("name-rule").with("name", "x").conforms());
}
