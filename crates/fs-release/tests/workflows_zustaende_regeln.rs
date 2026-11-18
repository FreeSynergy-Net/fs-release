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
