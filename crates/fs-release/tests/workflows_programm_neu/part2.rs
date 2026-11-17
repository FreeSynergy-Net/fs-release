use super::*;

/// Every step is reachable from the first one (a stub or a branch that nothing leads to is dead).
pub(super) fn all_reachable(text: &str) -> bool {
    let n = step_names(text).len();
    let mut seen = vec![false; n];
    let mut todo = vec![0];
    while let Some(i) = todo.pop() {
        if i >= n || seen[i] {
            continue;
        }
        seen[i] = true;
        todo.extend(successors(text, i));
    }
    seen.iter().all(|s| *s)
}

/// A jump back must land on a step that can change the thing the check looks at: an agent or a person, never a machine step.
pub(super) fn back_jumps_land_on_workers(text: &str) -> bool {
    let names = step_names(text);
    (0..names.len()).all(|i| {
        successors(text, i)
            .into_iter()
            .filter(|&j| j <= i)
            .all(|j| j < names.len() && !step_body(text, &names[j]).contains("actor: machine"))
    })
}

/// Every red loop of the flow stops in the third red round: its switch has a case whose `when` tests `runde` against exactly `>= 3`,
/// that case goes to `escalate` (a human step), and it stands before the first case that jumps back. `skip` = loops without a counter by design.
pub(super) fn red_loops_escalate(text: &str, escalate: &str, skip: &[&str]) -> bool {
    let names = step_names(text);
    (0..names.len()).all(|i| {
        let back = successors(text, i).into_iter().any(|j| j <= i);
        let b = step_body(text, &names[i]);
        if !back || !b.contains("      switch:\n") || skip.contains(&names[i].as_str()) {
            return true;
        }
        let lines: Vec<&str> = b
            .split("      switch:\n")
            .nth(1)
            .unwrap_or("")
            .lines()
            .collect();
        // the case that counts: `when` with `runde` and `>= 3`, directly followed by `then: <escalate>`
        let Some(k) = lines
            .iter()
            .position(|l| l.contains("runde") && threshold_is_3(l))
        else {
            return false;
        };
        let goes_to_escalate = lines
            .get(k + 1)
            .is_some_and(|l| l.trim() == format!("then: {escalate}"));
        let first_back = lines.iter().position(|l| {
            l.trim()
                .strip_prefix("then: ")
                .and_then(|t| names.iter().position(|n| n == t))
                .is_some_and(|j| j <= i)
        });
        goes_to_escalate && first_back.is_none_or(|f| k < f) && human_step(text, escalate)
    })
}

/// The `when` line compares against exactly `>= 3` (not 30, not 99).
pub(super) fn threshold_is_3(line: &str) -> bool {
    line.split(">= ").nth(1).is_some_and(|rest| {
        let mut c = rest.chars();
        c.next() == Some('3') && !c.next().is_some_and(|d| d.is_ascii_digit())
    })
}

pub(super) fn human_step(text: &str, step: &str) -> bool {
    let b = step_body(text, step);
    b.contains("actor: human") && b.contains("gate: human")
}

/// The step has the flow directive `then: end` at task level (6 spaces), so it ends the flow and does not fall into the next step.
pub(super) fn ends_the_flow(text: &str, step: &str) -> bool {
    let thens: Vec<String> = step_body(text, step)
        .lines()
        .filter(|l| l.len() - l.trim_start().len() == 6)
        .filter_map(|l| l.trim().strip_prefix("then: ").map(str::to_string))
        .collect();
    thens == ["end"]
}

pub(super) fn at(text: &str, s: &str) -> usize {
    position(text, s)
}

/// Security, packages, release, server run, in this order; the Frontend round does not release by itself.
pub(super) fn release_comes_last(text: &str) -> bool {
    at(text, "sprache") < at(text, "release")
        && at(text, "sicherheit-again") < at(text, "pakete")
        && at(text, "pakete-again") < at(text, "release")
        && at(text, "release") < at(text, "server-lauf")
        && at(text, "server-lauf") < at(text, "abnahme")
        && step_body(text, "frontend").contains("          release: false\n")
}

/// Step 5 checks only what step 5 makes; the commands and the round trip belong to step 9a.
pub(super) fn formats_before_commands(text: &str) -> bool {
    let u = step_body(text, "umzug-check");
    let g = step_body(text, "grenz-check");
    u.contains("call: format-check:")
        && g.contains("call: boundary-check:")
        && !u.contains("round trip over the fixtures keeps")
        && u.contains("NO import or export command")
        && g.contains("round trip")
}

pub(super) fn core_outcome_is_checkable(text: &str) -> bool {
    step_body(text, "kern-crate").contains("eigener-kern")
        && step_body(text, "kern-check").contains("no pin is asked for")
}

pub(super) fn p0_exits_are_reported(text: &str) -> bool {
    let w = step_body(text, "katalog-weiter");
    w.contains("when: '${ $context.katalog.fehler }'")
        && w.contains("then: vorhanden-bericht")
        && !w.contains("then: end")
        && ["vorhanden-bericht", "katalog-fehler"].iter().all(|s| {
            let b = step_body(text, s);
            b.contains("gate: human") && b.contains("actor: human")
        })
}

/// Every `.input.X` that the called flow reads is passed by the calling step.
pub(super) fn run_inputs_are_given(caller: &str) -> Vec<String> {
    run_inputs_missing(caller, &read(caller))
}

pub(super) fn run_inputs_missing(caller: &str, text: &str) -> Vec<String> {
    let mut missing = vec![];
    for s in step_names(text) {
        let b = step_body(text, &s);
        let Some(target_flow) = b
            .lines()
            .skip_while(|l| !l.contains("workflow:"))
            .find_map(|l| l.trim().strip_prefix("name: ").map(str::to_string))
        else {
            continue;
        };
        let given: Vec<String> = b
            .lines()
            .skip_while(|l| l.trim() != "input:")
            .skip(1)
            .filter_map(|l| l.trim().split_once(": ").map(|(k, _)| k.to_string()))
            .collect();
        let callee_text = read(&target_flow);
        let mut used: Vec<String> = callee_text
            .split(".input.")
            .skip(1)
            .map(|r| {
                r.chars()
                    .take_while(|c| c.is_ascii_alphanumeric() || *c == '_')
                    .collect()
            })
            .collect();
        used.sort();
        used.dedup();
        for u in used {
            if !given.contains(&u) {
                missing.push(format!("{caller} step {s} -> {target_flow}: {u}"));
            }
        }
    }
    missing
}

/// Every `.input.X` of programm-neu is named in its header (the caller knows what to give).
pub(super) fn inputs_are_declared(text: &str) -> bool {
    let header: String = text
        .lines()
        .take_while(|l| l.starts_with('#'))
        .collect::<Vec<_>>()
        .join("\n");
    let mut used: Vec<String> = text
        .split(".input.")
        .skip(1)
        .map(|r| {
            r.chars()
                .take_while(|c| c.is_ascii_alphanumeric() || *c == '_')
                .collect()
        })
        .collect();
    used.dedup();
    used.iter().all(|u| header.contains(u.as_str()))
}

#[test]
fn the_graph_has_no_dead_step_and_the_back_jumps_land_on_workers() {
    let text = read(FLOW);
    assert!(all_reachable(&text));
    assert!(back_jumps_land_on_workers(&text));
    // counter-tests
    assert!(!all_reachable(&text.replacen(
        "then: facette-abzweig",
        "then: continue",
        1
    )));
    assert!(!back_jumps_land_on_workers(&text.replacen(
        "then: wessen-daten",
        "then: ausgang-check",
        1
    )));
    assert!(!back_jumps_land_on_workers(&text.replacen(
        "then: standards\n",
        "then: standards-check\n",
        1
    )));
}

#[test]
fn every_red_loop_of_programm_neu_stops_in_the_third_red_round() {
    let text = read(FLOW);
    assert!(red_loops_escalate(&text, "eskalation", &["befunde"]));
    let one = "        - runden:\n            when: '${ ($context.standards.red and ($context.standards.runde // 0) >= 3) }'\n            then: eskalation\n";
    assert!(text.contains(one));
    // counter-tests: the case removed, the threshold 99 or 30, the case after the jump back, the target not a human step
    assert!(!red_loops_escalate(
        &text.replacen(one, "", 1),
        "eskalation",
        &["befunde"]
    ));
    assert!(!red_loops_escalate(
        &text.replacen(
            ">= 3) }'\n            then: eskalation",
            ">= 99) }'\n            then: eskalation",
            1
        ),
        "eskalation",
        &["befunde"]
    ));
    assert!(!red_loops_escalate(
        &text.replacen(
            ">= 3) }'\n            then: eskalation",
            ">= 30) }'\n            then: eskalation",
            1
        ),
        "eskalation",
        &["befunde"]
    ));
    assert!(!red_loops_escalate(
        &text.replacen(one, &one.replace("then: eskalation", "then: standards"), 1),
        "eskalation",
        &["befunde"]
    ));
    assert!(!red_loops_escalate(
        &text.replacen("then: eskalation", "then: katalog-fehler", 1),
        "eskalation",
        &["befunde"]
    ));
    assert!(step_body(&text, "eskalation").contains("actor: human"));
}
