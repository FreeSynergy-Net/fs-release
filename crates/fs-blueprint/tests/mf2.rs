//! Texts are simple MF2 messages (ADR 0057, ADR 0099 points 8 and 9): plain text and `{$name}` placeholders; the blueprint and the message catalog of the tool.
mod support;
use fs_blueprint::messages::MESSAGES;
use fs_blueprint::mf2::is_simple;
use support::{edit, run, TASKS};

const SUMMARY: &str = "summary = { en = \"Tasks, dependencies and boards\", de = \"Aufgaben, Abhängigkeiten und Boards\" }";

#[test]
fn what_is_a_simple_message() {
    for ok in [
        "",
        "plain",
        "a {$x} b",
        "{$a}{$b-c_d}",
        "esc \\{ \\} \\\\ \\|",
        "ä ö {$x}",
    ] {
        assert!(is_simple(ok), "{ok:?}");
    }
    for bad in [
        "{",
        "}",
        "a } b",
        "{x}",
        "{$}",
        "{$x",
        "{$x y}",
        "{ $x }",
        "{$x:number}",
        "{#b}",
        "{|lit|}",
        "\\n",
        "\\",
        "trailing \\",
        ".input {$x}",
    ] {
        assert!(!is_simple(bad), "{bad:?}");
    }
}

#[test]
fn every_message_of_the_tool_is_a_simple_message() {
    for (code, en, de) in MESSAGES {
        assert!(is_simple(en), "{code} en: {en}");
        assert!(is_simple(de), "{code} de: {de}");
    }
}

#[test]
fn a_text_of_the_blueprint_that_is_no_simple_message_is_refused_with_its_language() {
    let bad_en = edit(
        TASKS,
        SUMMARY,
        "summary = { en = \"Tasks {x}\", de = \"Aufgaben\" }",
    );
    let r = run(&bad_en, true);
    assert!(
        r.errors
            .iter()
            .any(|e| e.to_string() == "text-syntax{key=program.summary, lang=en}"),
        "{:?}",
        r.errors
    );
    let bad_de = edit(
        TASKS,
        SUMMARY,
        "summary = { en = \"Tasks\", de = \"Aufgaben }\" }",
    );
    let r = run(&bad_de, true);
    assert!(
        r.errors
            .iter()
            .any(|e| e.to_string() == "text-syntax{key=program.summary, lang=de}"),
        "{:?}",
        r.errors
    );
    let good = edit(
        TASKS,
        SUMMARY,
        "summary = { en = \"Tasks \\\\{ {$n}\", de = \"Aufgaben \\\\{ {$n}\" }",
    );
    assert!(!run(&good, true).has("text-syntax"));
}
