//! Pattern: none (test). The flow `koordinator-runde` (bundle 0.0.7): who does which step, the order, the gates, the red loops with escalation and
//! the rules. Agents run in the BACKGROUND on purpose: the coordinator is its own session and the messages go to it (instruction of the strategy,
//! 2026-11-15 evening; the earlier foreground rule applied only to coordinators inside the strategy session). The coordinator still ends only when
//! all agents have reported (`einsammeln` waits for all). Every result is green only when it says so; a missing result is never green.
//! Each rule has a counter-test on a changed copy. The flows `programm-grundgeruest` and `testlauf` are in `workflows_belt.rs`.

mod support;
use support::workflows::{read, step_body, step_names};

fn pos(text: &str, step: &str) -> usize {
    step_names(text)
        .iter()
        .position(|n| n == step)
        .unwrap_or_else(|| panic!("no step {step}"))
}

fn in_order(text: &str, order: &[&str]) -> bool {
    let names = step_names(text);
    order.iter().all(|o| names.iter().any(|n| n == o))
        && order.windows(2).all(|w| pos(text, w[0]) < pos(text, w[1]))
}

fn escalates(text: &str, again: &str, back: &str) -> bool {
    let b = step_body(text, again);
    b.contains("then: eskalation") && b.contains(">= 3") && b.contains(&format!("then: {back}"))
}

const ORDER: [&str; 25] = [
    "handoffs-lesen",
    "posteingang-lesen",
    "straenge",
    "agenten-starten",
    "einsammeln",
    "einsammeln-weiche",
    "auswerten",
    "gates-pruefen",
    "gates-again",
    "korrektur",
    "korrektur-einsammeln",
    "korrektur-weiche",
    "doku",
    "doku-weiter",
    "pruefung",
    "urteile-sammeln",
    "pruefung-again",
    "regel-check",
    "regel-weiche",
    "regel-rot",
    "sichern",
    "push-unreviewed",
    "handoff",
    "bericht",
    "eskalation",
];

fn round_is_sound(t: &str) -> bool {
    let rule = step_body(t, "regel-check");
    let push = step_body(t, "push-unreviewed");
    let fix = step_body(t, "korrektur");
    let review = step_body(t, "pruefung");
    let ga = step_body(t, "gates-again");
    in_order(t, &ORDER[..24])
        && starts_in_background(t)
        && step_body(t, "doku").contains("name: doku-mitziehen")
        && step_body(t, "doku").contains("actor: ai")
        && step_body(t, "doku-weiter").contains("then: eskalation")
        && step_body(t, "doku-weiter").contains("$context.doku.status != \"green\"")
        && step_body(t, "gates-pruefen").contains("name: testlauf")
        && step_body(t, "gates-pruefen").contains("actor: ai")
        && step_body(t, "pruefung").contains("gate: opus-ja")
        && rule.contains("actor: machine")
        && ["sleep", "pgrep", "big logs", "built or tested itself"]
            .iter()
            .all(|w| rule.contains(w))
        && push.contains("actor: human")
        && push.contains("unreviewed/*")
        && !push.contains("branches: main")
        && t.matches("name: report-to-strategy").count() == 1
        && step_body(t, "einsammeln").contains("no sleep, no pgrep")
        && step_body(t, "einsammeln").contains("wait: all")
        && step_body(t, "einsammeln-weiche").contains("$context.berichte.alle == true")
        && step_body(t, "einsammeln-weiche").contains("then: eskalation")
        && escalates(t, "gates-again", "korrektur")
        && ga.contains("$context.gates.status == \"green\"")
        && ga.contains("ohne-ergebnis")
        && fix.contains("name: agent-run")
        && !fix.contains("name: bugfix")
        && fix.contains("background: true")
        && step_body(t, "korrektur-einsammeln").contains("wait: all")
        && step_body(t, "korrektur-weiche").contains("then: gates-pruefen")
        && review.contains("resolve-head")
        && review.contains("head: ${ $context.kopf.head }")
        && review.contains("delta: ${ $schritt.delta }")
        && !review.contains(".input.head")
        && escalates(t, "pruefung-again", "korrektur")
        && step_body(t, "pruefung-again").contains("| all")
        && step_body(t, "pruefung-again").contains("ohne-ergebnis")
        && step_body(t, "regel-weiche").contains("$context.regel.red == false")
        && step_body(t, "regel-weiche").contains("then: regel-rot")
        && step_body(t, "eskalation").contains("then: sichern")
        && step_body(t, "bericht").contains("then: end")
        && !t.contains("then: continue")
}

fn starts_in_background(t: &str) -> bool {
    let start = step_body(t, "agenten-starten");
    start.contains("background: true")
        && start.contains("report_to: coordinator")
        && start.contains("Haiku searches and never writes code")
        && start.contains("for:")
}

#[test]
fn the_round_reads_first_starts_background_agents_waits_for_all_and_sends_one_report() {
    assert!(round_is_sound(&read("koordinator-runde")));
    // the reason for the background start is in the header, not only the rule
    let t = read("koordinator-runde");
    assert!(t.lines().take(14).any(|l| l.contains("Why background")));
    assert!(t.contains("INSIDE the strategy session"));
}

#[test]
fn counter_a_foreground_start_a_second_report_a_late_head_or_a_release_is_caught() {
    let t = read("koordinator-runde");
    let bad = |from: &str, to: &str| {
        assert!(t.contains(from), "{from}");
        assert!(!round_is_sound(&t.replace(from, to)), "{from}");
    };
    bad("background: true", "background: false");
    bad("report_to: coordinator", "report_to: strategy");
    bad("name: doku-mitziehen", "name: x");
    bad("name: testlauf", "name: slot-check");
    bad("sleep", "wait");
    bad(">= 3", ">= 99");
    bad("wait: all", "wait: none");
    // R2: the fix goes through bugfix.yaml (ends in release-flow) instead of agent-run
    let fix = step_body(&t, "korrektur");
    assert!(!round_is_sound(&t.replacen(
        &fix,
        &fix.replace("name: agent-run", "name: bugfix"),
        1
    )));
    // R3: the review takes the head of the round start
    assert!(!round_is_sound(&t.replace(
        "head: ${ $context.kopf.head }",
        "head: ${ .input.head }"
    )));
    // R4: an escalation of the docs gate is ignored
    let dw = step_body(&t, "doku-weiter");
    assert!(!round_is_sound(&t.replacen(
        &dw,
        &dw.replace("then: eskalation", "then: pruefung"),
        1
    )));
    // R5: the escalation ends without securing and report
    let es = step_body(&t, "eskalation");
    assert!(!round_is_sound(&t.replacen(
        &es,
        &es.replace("then: sichern", "then: end"),
        1
    )));
    // R6: a missing result counts as green
    bad("when: '${ $context.regel.red == false }'", "when: 'true'");
    let ga = step_body(&t, "gates-again");
    assert!(!round_is_sound(&t.replacen(
        &ga,
        &ga.replace(
            "        - ohne-ergebnis:\n            then: eskalation\n",
            "        - ohne-ergebnis:\n            then: continue\n"
        ),
        1
    )));
    let bw = step_body(&t, "bericht");
    assert!(!round_is_sound(&t.replacen(
        &bw,
        &bw.replace("then: end", "then: eskalation"),
        1
    )));
    assert!(!round_is_sound(&t.replacen(
        "  - bericht:",
        "  - bericht-zwei:\n      run:\n        workflow:\n          name: report-to-strategy\n  - bericht:",
        1
    )));
    let swapped = t
        .replacen("  - straenge:", "  - zz:", 1)
        .replacen("  - handoffs-lesen:", "  - straenge:", 1)
        .replacen("  - zz:", "  - handoffs-lesen:", 1);
    assert!(!round_is_sound(&swapped));
    // the review must come after the docs
    let moved = t
        .replacen("  - doku:", "  - zz:", 1)
        .replacen("  - pruefung:", "  - doku:", 1)
        .replacen("  - zz:", "  - pruefung:", 1);
    assert!(!round_is_sound(&moved));
}

#[test]
fn counter_a_push_by_an_agent_or_of_main_is_caught() {
    let t = read("koordinator-runde");
    let push = step_body(&t, "push-unreviewed");
    let by_ai = push.replace("actor: human", "actor: ai");
    assert!(!round_is_sound(&t.replacen(&push, &by_ai, 1)));
    assert!(!round_is_sound(&t.replace("unreviewed/*", "main")));
}

#[test]
fn the_new_flows_are_checked_by_the_schema_check() {
    for f in ["koordinator-runde", "programm-grundgeruest", "testlauf"] {
        support::workflows::assert_red(f, "freesynergy.net:actor", |t| {
            t.replacen("freesynergy.net:actor: ", "freesynergy.net:acter: ", 1)
        });
    }
}
