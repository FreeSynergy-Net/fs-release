//! Pattern: none (test). The ONE rule set of the states of a run (`workflows/run/rules.yaml`, standard `workflows/STATES.md`, ADR 0103 E2/E3):
//! the real files pass, the default is exactly the standard (the literals below pin it), and every structure rule of the check has a
//! counter-test on a changed copy of the rule set.

mod support;
use std::collections::BTreeSet;
use std::fs;
use support::workflows::{assert_rules_red, read, root, rules, FILES};

const STATES: [&str; 8] = [
    "offen",
    "läuft",
    "wartet",
    "pausiert",
    "fehlgeschlagen",
    "abgebrochen",
    "erledigt",
    "archiviert",
];
const ROLES: [&str; 6] = [
    "offen",
    "läuft",
    "wartet",
    "pausiert",
    "fehlgeschlagen",
    "abgebrochen",
];
const LIVE: [&str; 5] = ["offen", "läuft", "wartet", "pausiert", "fehlgeschlagen"];
const REASONS: [&str; 3] = ["auf-entscheidung", "auf-anfrage", "in-prüfung"];
const STATES_LINE: &str =
    "states: [offen, läuft, wartet, pausiert, fehlgeschlagen, abgebrochen, erledigt, archiviert]";

/// `source -> target` pairs of the `transitions:` block.
fn pairs(text: &str) -> BTreeSet<String> {
    let mut out = BTreeSet::new();
    for l in text
        .lines()
        .skip_while(|l| !l.starts_with("transitions:"))
        .skip(1)
        .take_while(|l| l.starts_with("  "))
    {
        let (src, rest) = l.trim().split_once(": [").unwrap();
        for t in rest
            .trim_end_matches(']')
            .split(", ")
            .filter(|x| !x.is_empty())
        {
            out.insert(format!("{src} -> {t}"));
        }
    }
    out
}

/// The same pairs out of the transition table of `STATES.md`.
fn table_pairs(doc: &str) -> BTreeSet<String> {
    let mut out = BTreeSet::new();
    for l in doc
        .lines()
        .skip_while(|l| !l.starts_with("| From / Von"))
        .skip(2)
        .take_while(|l| l.starts_with('|'))
    {
        let cells: Vec<&str> = l.trim_matches('|').split('|').map(str::trim).collect();
        let name = |x: &str| x.replace('`', "").trim().to_string();
        for s in cells[0].split(", ").map(name) {
            for t in cells[1].split(", ") {
                let t = name(t.trim_end_matches("(reopen)").trim_end_matches("(repeat)"));
                if !t.starts_with("none") {
                    out.insert(format!("{s} -> {t}"));
                }
            }
        }
    }
    out
}

fn drop_target(r: &str, src: &str, target: &str) -> String {
    let prefix = format!("  {src}: [");
    r.lines()
        .map(|l| match l.strip_prefix(&prefix) {
            Some(rest) => {
                let kept: Vec<&str> = rest
                    .trim_end_matches(']')
                    .split(", ")
                    .filter(|x| *x != target)
                    .collect();
                format!("{prefix}{}]", kept.join(", "))
            }
            None => l.to_string(),
        })
        .collect::<Vec<_>>()
        .join("\n")
        + "\n"
}

#[test]
fn the_real_rule_set_is_the_standard_and_no_flow_carries_states() {
    let r = rules();
    for line in [
        STATES_LINE,
        "name: freesynergy",
        "start: offen",
        "final: [erledigt, abgebrochen, archiviert]",
        "reopen: [erledigt, abgebrochen]",
        "archive-from: [erledigt, abgebrochen, fehlgeschlagen]",
        "  wartet: [auf-entscheidung, auf-anfrage, in-prüfung]",
    ] {
        assert!(r.lines().any(|l| l == line), "{line}");
    }
    assert_eq!(pairs(&r).len(), 27);
    assert!(!r.contains("stationen"));
    for f in FILES {
        let t = read(f);
        assert!(
            !t.contains("freesynergy.net:states") && !t.contains("freesynergy.net:transitions"),
            "{f}.yaml"
        );
    }
}

#[test]
fn the_tables_of_the_standard_equal_the_rule_set() {
    let doc = fs::read_to_string(root().join("workflows/STATES.md")).unwrap();
    assert_eq!(
        table_pairs(&doc),
        pairs(&rules()),
        "STATES.md transition table = run/rules.yaml"
    );
    let rows: Vec<String> = doc
        .lines()
        .skip_while(|l| !l.starts_with("| State / Zustand"))
        .skip(2)
        .take_while(|l| l.starts_with('|'))
        .map(|l| l[3..].split('`').next().unwrap().to_string())
        .collect();
    assert_eq!(
        rows,
        STATES.map(str::to_string).to_vec(),
        "STATES.md: one row per state, in the order of the rule set"
    );
    for reason in REASONS {
        assert!(
            doc.contains(&format!("`{reason}`")),
            "STATES.md names the reason {reason}"
        );
    }
}

#[test]
fn mutation_every_single_state_removed_from_the_rule_set_is_red() {
    for s in STATES {
        let shorter: Vec<&str> = STATES.iter().copied().filter(|x| *x != s).collect();
        let needle = if ROLES.contains(&s) {
            format!("state '{s}' (")
        } else {
            "list of states".to_string()
        };
        assert_rules_red(&needle, |r| {
            r.replacen(STATES_LINE, &format!("states: [{}]", shorter.join(", ")), 1)
        });
    }
}

#[test]
fn mutation_duplicate_case_stationen_and_no_states_are_red() {
    assert_rules_red("state 'offen' is listed more than once", |r| {
        r.replacen("states: [offen, ", "states: [offen, offen, ", 1)
    });
    assert_rules_red("state 'pausiert' (pause) is missing", |r| {
        r.replacen("läuft, wartet, pausiert,", "läuft, wartet, Pausiert,", 1)
    });
    assert_rules_red("transition rule of 'stationen' is missing", |r| {
        r.replacen("archiviert]\n", "archiviert, stationen]\n", 1)
    });
    assert_rules_red("`states` is missing", |r| {
        r.replacen("states: [", "zustaende: [", 1)
    });
}

#[test]
fn mutation_every_rule_line_removed_completely_is_red() {
    for src in STATES {
        let prefix = format!("  {src}: [");
        assert_rules_red(&format!("transition rule of '{src}' is missing"), |r| {
            r.lines()
                .filter(|l| !l.starts_with(&prefix))
                .collect::<Vec<_>>()
                .join("\n")
                + "\n"
        });
    }
}

#[test]
fn mutation_abort_and_pause_from_every_live_state_including_failed_are_required() {
    for src in LIVE {
        assert_rules_red(
            &format!("transition '{src}' -> 'abgebrochen' is missing"),
            |r| drop_target(&r, src, "abgebrochen"),
        );
        if src != "pausiert" {
            assert_rules_red(
                &format!("transition '{src}' -> 'pausiert' is missing"),
                |r| drop_target(&r, src, "pausiert"),
            );
        }
    }
}

#[test]
fn mutation_the_role_transitions_are_required() {
    for (src, tgt) in [
        ("fehlgeschlagen", "läuft"),
        ("läuft", "fehlgeschlagen"),
        ("pausiert", "läuft"),
        ("läuft", "wartet"),
        ("erledigt", "offen"),
        ("abgebrochen", "offen"),
        ("fehlgeschlagen", "archiviert"),
        ("erledigt", "archiviert"),
        ("abgebrochen", "archiviert"),
    ] {
        assert_rules_red(&format!("transition '{src}' -> '{tgt}' is missing"), |r| {
            drop_target(&r, src, tgt)
        });
    }
}
