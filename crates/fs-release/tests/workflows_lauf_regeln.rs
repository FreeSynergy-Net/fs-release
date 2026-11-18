//! Pattern: none (test). The ONE rule set of the states of a run (`workflows/run/rules.yaml`, standard `workflows/STATES.md`): the real
//! files pass, every rule of the check has a counter-test on a changed copy of the rule set, and the table of the standard equals the
//! rule set pair by pair.

mod support;
use std::collections::BTreeSet;
use std::fs;
use support::workflows::{assert_rules_red, root, rules, FILES};

const STATES: [&str; 9] = [
    "offen",
    "wartet-auf-entscheidung",
    "auf-anfrage",
    "in-prüfung",
    "pausiert",
    "fehlgeschlagen",
    "abgebrochen",
    "erledigt",
    "archiviert",
];
const STATES_LINE: &str = "states: [offen, wartet-auf-entscheidung, auf-anfrage, in-prüfung, pausiert, fehlgeschlagen, abgebrochen, erledigt, archiviert]";

/// `source -> target` pairs of the `transitions:` block (`stationen` stays a word).
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
fn the_real_rule_set_has_nine_states_ten_rules_and_no_flow_carries_states() {
    let r = rules();
    assert!(r.contains(STATES_LINE));
    assert_eq!(pairs(&r).len(), 42);
    for f in FILES {
        let t = support::workflows::read(f);
        assert!(!t.contains("freesynergy.net:states"), "{f}.yaml");
        assert!(!t.contains("freesynergy.net:transitions"), "{f}.yaml");
    }
}

#[test]
fn the_table_of_the_standard_equals_the_rule_set_pair_by_pair() {
    let doc = fs::read_to_string(root().join("workflows/STATES.md")).unwrap();
    assert_eq!(
        table_pairs(&doc),
        pairs(&rules()),
        "STATES.md table = run/rules.yaml"
    );
    let mut rows: Vec<String> = Vec::new();
    for l in doc
        .lines()
        .skip_while(|l| !l.starts_with("| State / Zustand"))
        .skip(2)
        .take_while(|l| l.starts_with('|'))
    {
        rows.push(l[3..].split('`').next().unwrap().to_string());
    }
    let want: Vec<String> = STATES.iter().map(|s| (*s).to_string()).collect();
    assert_eq!(
        rows, want,
        "STATES.md: one row per state, in the order of the rule set"
    );
}

#[test]
fn mutation_every_single_state_removed_from_the_rule_set_is_red() {
    for s in STATES {
        let shorter: Vec<&str> = STATES.iter().copied().filter(|x| *x != s).collect();
        assert_rules_red(&format!("state '{s}' is missing"), |r| {
            r.replacen(STATES_LINE, &format!("states: [{}]", shorter.join(", ")), 1)
        });
    }
}

#[test]
fn mutation_unknown_stationen_duplicate_and_case_in_the_states_are_red() {
    assert_rules_red("unknown state 'stationen'", |r| {
        r.replacen("archiviert]\n", "archiviert, stationen]\n", 1)
    });
    assert_rules_red("unknown state 'Pausiert'", |r| {
        r.replacen("in-prüfung, pausiert,", "in-prüfung, Pausiert,", 1)
    });
    assert_rules_red("state 'offen' is listed more than once", |r| {
        r.replacen("states: [offen, ", "states: [offen, offen, ", 1)
    });
    assert_rules_red("`states` is missing", |r| {
        r.replacen("states: [", "zustaende: [", 1)
    });
}

#[test]
fn mutation_every_rule_line_removed_completely_is_red() {
    for src in STATES.iter().copied().chain(["stationen"]) {
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
fn mutation_abort_and_pause_from_every_live_state_are_required() {
    for src in [
        "offen",
        "stationen",
        "wartet-auf-entscheidung",
        "auf-anfrage",
        "in-prüfung",
        "pausiert",
        "fehlgeschlagen",
    ] {
        assert_rules_red(
            &format!("transition '{src}' -> 'abgebrochen' is missing"),
            |r| drop_target(&r, src, "abgebrochen"),
        );
    }
    for src in [
        "offen",
        "stationen",
        "wartet-auf-entscheidung",
        "auf-anfrage",
        "in-prüfung",
    ] {
        assert_rules_red(
            &format!("transition '{src}' -> 'pausiert' is missing"),
            |r| drop_target(&r, src, "pausiert"),
        );
    }
}

#[test]
fn mutation_failed_runs_reopen_fail_and_final_states_are_required() {
    assert_rules_red(
        "transition 'fehlgeschlagen' -> 'stationen' is missing",
        |r| drop_target(&r, "fehlgeschlagen", "stationen"),
    );
    assert_rules_red(
        "transition 'stationen' -> 'fehlgeschlagen' is missing",
        |r| drop_target(&r, "stationen", "fehlgeschlagen"),
    );
    assert_rules_red("transition 'erledigt' -> 'offen' is missing", |r| {
        drop_target(&r, "erledigt", "offen")
    });
    assert_rules_red("transition 'abgebrochen' -> 'offen' is missing", |r| {
        drop_target(&r, "abgebrochen", "offen")
    });
}

#[test]
fn mutation_archiviert_only_from_the_three_and_without_way_out() {
    assert_rules_red("'archiviert' has no way out", |r| {
        r.replacen("  archiviert: []", "  archiviert: [offen]", 1)
    });
    assert_rules_red(
        "transition 'pausiert' -> 'archiviert' is not allowed",
        |r| {
            r.replacen(
                "in-prüfung, abgebrochen]",
                "in-prüfung, abgebrochen, archiviert]",
                1,
            )
        },
    );
    assert_rules_red("transition 'offen' -> 'archiviert' is not allowed", |r| {
        r.replacen(
            "pausiert, abgebrochen]",
            "pausiert, abgebrochen, archiviert]",
            1,
        )
    });
}

#[test]
fn mutation_unknown_source_target_and_non_list_rules_are_red() {
    assert_rules_red("transition from unknown state 'foo'", |r| {
        format!("{r}  foo: [offen]\n")
    });
    assert_rules_red("transitions of 'offen': a list of states", |r| {
        r.replacen("  offen: [stationen,", "  offen: [foo, stationen,", 1)
    });
    for bad in ["{a: b}", "[x]"] {
        assert_rules_red("transitions of 'offen': a list of states", |r| {
            r.replacen(
                "  offen: [stationen,",
                &format!("  offen: [{bad}, stationen,"),
                1,
            )
        });
    }
    assert_rules_red("transitions of 'pausiert': a list of states", |r| {
        r.replacen(
            "  pausiert: [offen, stationen, wartet-auf-entscheidung, auf-anfrage, in-prüfung, abgebrochen]",
            "  pausiert: abgebrochen",
            1,
        )
    });
    assert_rules_red("transitions of 'offen': a target is listed twice", |r| {
        r.replacen("  offen: [stationen,", "  offen: [stationen, stationen,", 1)
    });
}

#[test]
fn mutation_start_and_wait_reasons_are_fixed() {
    assert_rules_red("`start` is 'pausiert'", |r| {
        r.replacen("start: offen", "start: pausiert", 1)
    });
    assert_rules_red("`wait-reasons` must map exactly", |r| {
        r.replacen("auf-anfrage: anfrage", "auf-anfrage: frage", 1)
    });
    assert_rules_red("`wait-reasons` must map exactly", |r| {
        r.replacen("  in-prüfung: prüfung\n", "", 1)
    });
}
