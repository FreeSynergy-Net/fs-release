//! Pattern: none (test). Counter-tests of the state rules (Opus review mutants E1-E9): every live state reaches `pausiert` and
//! `abgebrochen`, unknown rule source/target, a flow without `document.metadata`, `stationen` is no state, the table of the standard.

mod support;
use std::fs;
use support::workflows::{assert_red, root};

const STATES_LINE: &str = "    freesynergy.net:states: [offen, wartet-auf-entscheidung, auf-anfrage, in-prüfung, pausiert, abgebrochen, erledigt, archiviert]\n";

/// Removes `target` from the rule line `      <src>: [...]` of a flow (the other targets stay).
fn without_target(t: &str, src: &str, target: &str) -> String {
    let prefix = format!("      {src}: [");
    t.lines()
        .map(|l| {
            if l.starts_with(&prefix) {
                let inner = &l[prefix.len()..l.len() - 1];
                let kept: Vec<&str> = inner.split(", ").filter(|x| *x != target).collect();
                format!("{prefix}{}]", kept.join(", "))
            } else {
                l.to_string()
            }
        })
        .collect::<Vec<_>>()
        .join("\n")
        + "\n"
}

#[test]
fn mutation_every_live_state_reaches_pausiert_and_abgebrochen() {
    for src in [
        "offen",
        "stationen",
        "wartet-auf-entscheidung",
        "auf-anfrage",
        "in-prüfung",
    ] {
        for target in ["pausiert", "abgebrochen"] {
            assert_red(
                "bugfix",
                &format!("transition '{src}' -> '{target}' is missing"),
                |t| without_target(&t, src, target),
            );
        }
    }
    assert_red(
        "bugfix",
        "transition 'pausiert' -> 'abgebrochen' is missing",
        |t| without_target(&t, "pausiert", "abgebrochen"),
    );
}

#[test]
fn mutation_unknown_transition_source_or_target_is_red() {
    assert_red("bugfix", "transition from unknown state 'foo'", |t| {
        t.replacen(
            "      archiviert: []\n",
            "      archiviert: []\n      foo: [offen]\n",
            1,
        )
    });
    assert_red("bugfix", "transitions of 'offen': a list of states", |t| {
        t.replacen(
            "offen: [stationen, wartet-auf-entscheidung, auf-anfrage, pausiert, abgebrochen]",
            "offen: [stationen, wartet-auf-entscheidung, auf-anfrage, pausiert, abgebrochen, foo]",
            1,
        )
    });
}

#[test]
fn mutation_a_file_without_document_metadata_is_red() {
    assert_red("bugfix", "document.metadata is missing", |t| {
        let mut skip = false;
        let mut out = String::new();
        for l in t.lines() {
            if l == "  metadata:" {
                skip = true;
            } else if skip && !l.starts_with("    ") {
                skip = false;
            }
            if !skip {
                out.push_str(l);
                out.push('\n');
            }
        }
        out
    });
}

#[test]
fn mutation_stationen_is_not_a_state() {
    assert_red("bugfix", "unknown state 'stationen'", |t| {
        t.replacen(
            "erledigt, archiviert]",
            "erledigt, archiviert, stationen]",
            1,
        )
    });
}

#[test]
fn the_state_table_of_the_standard_is_exactly_the_set_of_states() {
    let doc = fs::read_to_string(root().join("workflows/STATES.md")).unwrap();
    let mut rows: Vec<String> = Vec::new();
    let mut in_table = false;
    for l in doc.lines() {
        if l.starts_with("| State / Zustand") {
            in_table = true;
        } else if in_table && !l.starts_with('|') {
            break;
        } else if in_table && l.starts_with("| `") {
            rows.push(l[3..].split('`').next().unwrap().to_string());
        }
    }
    rows.sort();
    let mut want: Vec<String> = STATES_LINE
        .split('[')
        .nth(1)
        .unwrap()
        .trim_end_matches("]\n")
        .split(", ")
        .map(str::to_string)
        .collect();
    want.sort();
    assert_eq!(rows, want, "STATES.md: one table row per state, no more");
}
