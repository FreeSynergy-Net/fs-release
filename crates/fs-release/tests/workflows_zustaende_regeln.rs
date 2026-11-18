//! Pattern: none (test). Complete loops over the state rules (Opus review mutants F1-F7): EVERY rule pair of a real flow removed one by one,
//! EVERY state removed from the states line, EVERY state name (and `stationen`) as the name of a step. Each turns the check red.

mod support;
use std::fmt::Write;
use support::workflows::{assert_red, read};

const SRC_STATES: [&str; 8] = [
    "offen",
    "wartet-auf-entscheidung",
    "auf-anfrage",
    "in-prüfung",
    "pausiert",
    "abgebrochen",
    "erledigt",
    "archiviert",
];

/// The rule lines of the transitions block of a real flow: (source, targets).
fn rules(text: &str) -> Vec<(String, Vec<String>)> {
    text.lines()
        .skip_while(|l| !l.starts_with("    freesynergy.net:transitions:"))
        .skip(1)
        .take_while(|l| l.starts_with("      "))
        .map(|l| {
            let (src, rest) = l.trim().split_once(": [").unwrap();
            let inner = rest.trim_end_matches(']');
            let targets = inner
                .split(", ")
                .filter(|x| !x.is_empty())
                .map(str::to_string)
                .collect();
            (src.to_string(), targets)
        })
        .collect()
}

/// Removes the first occurrence of `item` from the rule line of `src`.
fn without_pair(t: &str, src: &str, item: &str) -> String {
    let prefix = format!("      {src}: [");
    let mut out = String::new();
    for l in t.lines() {
        if l.starts_with(&prefix) {
            let mut items: Vec<&str> = l[prefix.len()..l.len() - 1].split(", ").collect();
            let at = items.iter().position(|x| *x == item).unwrap();
            items.remove(at);
            let _ = writeln!(out, "{prefix}{}]", items.join(", "));
        } else {
            out.push_str(l);
            out.push('\n');
        }
    }
    out
}

#[test]
fn the_real_flow_has_all_nine_rules_with_many_pairs() {
    let r = rules(&read("bugfix"));
    assert_eq!(r.len(), 9, "{r:?}");
    assert_eq!(r.iter().map(|(_, t)| t.len()).sum::<usize>(), 38);
}

#[test]
fn mutation_every_single_rule_pair_removed_is_red() {
    let text = read("bugfix");
    for (src, targets) in rules(&text) {
        for target in targets {
            assert_red(
                "bugfix",
                &format!("transition '{src}' -> '{target}' is missing"),
                |t| without_pair(&t, &src, &target),
            );
        }
    }
}

#[test]
fn mutation_every_single_state_removed_from_the_states_line_is_red() {
    for s in SRC_STATES {
        assert_red("bugfix", &format!("state '{s}' is missing"), |t| {
            let line = format!("    freesynergy.net:states: [{}]", SRC_STATES.join(", "));
            let shorter: Vec<&str> = SRC_STATES.iter().copied().filter(|x| *x != s).collect();
            t.replacen(
                &line,
                &format!("    freesynergy.net:states: [{}]", shorter.join(", ")),
                1,
            )
        });
    }
}

#[test]
fn mutation_every_state_name_and_stationen_as_a_step_name_is_red() {
    for name in SRC_STATES.iter().copied().chain(["stationen"]) {
        assert_red("bugfix", "carries the name of a state", |t| {
            t.replacen("  - finding:\n", &format!("  - {name}:\n"), 1)
        });
    }
}

#[test]
fn mutation_every_rule_line_removed_completely_is_red() {
    let text = read("bugfix");
    for (src, _) in rules(&text) {
        assert_red(
            "bugfix",
            &format!("transition rule of '{src}' is missing"),
            |t| {
                let prefix = format!("      {src}: [");
                t.lines()
                    .filter(|l| !l.starts_with(&prefix))
                    .collect::<Vec<_>>()
                    .join("\n")
                    + "\n"
            },
        );
    }
}

#[test]
fn mutation_a_rule_as_string_instead_of_a_list_is_red() {
    assert_red(
        "bugfix",
        "transitions of 'pausiert': a list of states",
        |t| {
            t.replacen(
            "      pausiert: [offen, stationen, wartet-auf-entscheidung, auf-anfrage, in-prüfung, abgebrochen]",
            "      pausiert: abgebrochen",
            1,
        )
        },
    );
}

#[test]
fn mutation_state_names_are_exact_lowercase() {
    assert_red("bugfix", "unknown state 'Pausiert'", |t| {
        t.replacen(
            "in-prüfung, pausiert, abgebrochen",
            "in-prüfung, Pausiert, abgebrochen",
            1,
        )
    });
}

#[test]
fn mutation_a_duplicate_state_is_red() {
    assert_red("bugfix", "state 'offen' is listed more than once", |t| {
        t.replacen("states: [offen, ", "states: [offen, offen, ", 1)
    });
}

#[test]
fn mutation_a_rule_target_that_is_a_dict_or_a_list_is_red_without_a_crash() {
    let rule = "offen: [stationen, wartet-auf-entscheidung, auf-anfrage, pausiert, abgebrochen]";
    for bad in ["{a: b}", "[x]"] {
        assert_red("bugfix", "transitions of 'offen': a list of states", |t| {
            t.replacen(
                rule,
                &format!("offen: [stationen, {bad}, wartet-auf-entscheidung, auf-anfrage, pausiert, abgebrochen]"),
                1,
            )
        });
    }
}

/// The transition table of STATES.md as pairs `source -> target` (`stationen` stays a word, `(reopen)` is a remark).
fn table_pairs(doc: &str) -> Vec<String> {
    let mut pairs = Vec::new();
    for l in doc
        .lines()
        .skip_while(|l| !l.starts_with("| From / Von"))
        .skip(2)
        .take_while(|l| l.starts_with('|'))
    {
        let cells: Vec<&str> = l.trim_matches('|').split('|').map(str::trim).collect();
        let name = |x: &str| x.replace('`', "").trim().to_string();
        let sources: Vec<String> = cells[0].split(", ").map(name).collect();
        let targets: Vec<String> = cells[1]
            .split(", ")
            .map(|x| name(x.trim_end_matches("(reopen)")))
            .filter(|x| !x.starts_with("none"))
            .collect();
        for s in &sources {
            for t in &targets {
                pairs.push(format!("{s} -> {t}"));
            }
        }
    }
    pairs.sort();
    pairs
}

#[test]
fn the_transition_table_of_the_standard_equals_the_rules_pair_by_pair() {
    let doc =
        std::fs::read_to_string(support::workflows::root().join("workflows/STATES.md")).unwrap();
    let mut want: Vec<String> = rules(&read("bugfix"))
        .iter()
        .flat_map(|(s, ts)| ts.iter().map(move |t| format!("{s} -> {t}")))
        .collect();
    want.sort();
    assert_eq!(want.len(), 38);
    assert_eq!(
        table_pairs(&doc),
        want,
        "STATES.md transition table = the 38 rules"
    );
}
