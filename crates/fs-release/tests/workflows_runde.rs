//! Pattern: none (test). The flows `koordinator-runde`, `programm-grundgeruest` and `testlauf` (bundle 0.0.7): who does which step, the order,
//! the gates, the red loops with escalation, and the rules (one agent per step in the background, one report, the fixed trait, only targeted
//! tests while working). Each rule has a counter-test on a changed copy.

mod support;
use support::workflows::{read, step_body, step_names};

fn pos(text: &str, step: &str) -> usize {
    step_names(text)
        .iter()
        .position(|n| n == step)
        .unwrap_or_else(|| panic!("no step {step}"))
}

fn in_order(text: &str, order: &[&str]) -> bool {
    order.windows(2).all(|w| pos(text, w[0]) < pos(text, w[1]))
}

fn escalates(text: &str, again: &str, back: &str) -> bool {
    let b = step_body(text, again);
    b.contains("then: eskalation") && b.contains(">= 3") && b.contains(&format!("then: {back}"))
}

fn round_is_sound(t: &str) -> bool {
    let start = step_body(t, "agenten-starten");
    let rule = step_body(t, "regel-check");
    let push = step_body(t, "push-unreviewed");
    in_order(
        t,
        &[
            "handoffs-lesen",
            "posteingang-lesen",
            "straenge",
            "agenten-starten",
            "einsammeln",
            "auswerten",
            "gates-pruefen",
            "gates-again",
            "doku",
            "pruefung",
            "regel-check",
            "sichern",
            "push-unreviewed",
            "handoff",
            "bericht",
        ],
    ) && start.contains("background: true")
        && start.contains("report_to: coordinator")
        && start.contains("Haiku searches and never writes code")
        && start.contains("for:")
        && step_body(t, "doku").contains("name: doku-mitziehen")
        && step_body(t, "gates-pruefen").contains("name: testlauf")
        && step_body(t, "gates-pruefen").contains("actor: machine")
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
        && escalates(t, "gates-again", "korrektur")
        && step_body(t, "korrektur").contains("then: gates-pruefen")
}

#[test]
fn the_round_reads_first_starts_background_agents_and_sends_one_report() {
    assert!(round_is_sound(&read("koordinator-runde")));
}

#[test]
fn counter_a_round_with_a_foreground_agent_a_second_report_or_a_late_read_is_caught() {
    let t = read("koordinator-runde");
    assert!(!round_is_sound(
        &t.replace("background: true", "background: false")
    ));
    assert!(!round_is_sound(
        &t.replace("report_to: coordinator", "report_to: strategy")
    ));
    assert!(!round_is_sound(&t.replacen(
        "  - bericht:",
        "  - bericht-zwei:\n      run:\n        workflow:\n          name: report-to-strategy\n  - bericht:",
        1
    )));
    assert!(!round_is_sound(
        &t.replace("name: doku-mitziehen", "name: x")
    ));
    assert!(!round_is_sound(
        &t.replace("name: testlauf", "name: slot-check")
    ));
    assert!(!round_is_sound(&t.replace("sleep", "wait")));
    assert!(!round_is_sound(&t.replace(">= 3", ">= 99")));
    let swapped = t
        .replacen("  - straenge:", "  - zz:", 1)
        .replacen("  - handoffs-lesen:", "  - straenge:", 1)
        .replacen("  - zz:", "  - handoffs-lesen:", 1);
    assert!(!round_is_sound(&swapped));
}

#[test]
fn counter_a_push_by_an_agent_or_of_main_is_caught() {
    let t = read("koordinator-runde");
    let push = step_body(&t, "push-unreviewed");
    let by_ai = push.replace("actor: human", "actor: ai");
    assert!(!round_is_sound(&t.replacen(&push, &by_ai, 1)));
    assert!(!round_is_sound(&t.replace("unreviewed/*", "main")));
}

fn belt_is_sound(t: &str) -> bool {
    let s1 = step_body(t, "stufe-1");
    let s3 = step_body(t, "stufe-3");
    let ok = step_body(t, "strategie-ok");
    in_order(
        t,
        &[
            "stufe-1",
            "stufe-1-check",
            "stufe-1-again",
            "adr-suche",
            "plan",
            "plan-check",
            "plan-again",
            "strategie-ok",
            "trait-sperre",
            "stufe-3",
            "trait-unveraendert",
            "trait-again",
            "gates",
            "speicher-check",
            "lauf-check",
            "stufe-3-again",
            "review",
            "bilder",
            "doku",
        ],
    ) && s1.contains("fork:")
        && s1.contains("dossier-doku")
        && s1.contains("recherche")
        && s1.contains("never code")
        && s3.contains("fork:")
        && s3.contains("kern")
        && s3.contains("frontend")
        && s3.contains("ADR 0086")
        && s3.contains("block assignment")
        && ok.contains("actor: human")
        && ok.contains("gate: strategie-ok")
        && step_body(t, "plan").contains("FIXED trait")
        && step_body(t, "plan").contains("gate: opus-ja")
        && step_body(t, "trait-sperre").contains("trait.sha256")
        && step_body(t, "trait-unveraendert").contains("actor: machine")
        && step_body(t, "trait-again").contains("then: plan")
        && step_body(t, "speicher-check").contains("except `original`")
        && step_body(t, "review").contains("gate: opus-ja")
        && step_body(t, "bilder").contains("name: picture-acceptance")
        && step_body(t, "doku").contains("name: doku-mitziehen")
        && escalates(t, "stufe-1-again", "stufe-1")
        && escalates(t, "plan-again", "plan")
        && escalates(t, "stufe-3-again", "stufe-3")
}

#[test]
fn the_belt_has_two_parallel_stages_a_fixed_trait_and_the_approval_gate() {
    assert!(belt_is_sound(&read("programm-grundgeruest")));
}

#[test]
fn counter_a_belt_without_the_gate_the_lock_or_the_parallel_stage_is_caught() {
    let t = read("programm-grundgeruest");
    assert!(!belt_is_sound(
        &t.replace("gate: strategie-ok", "gate: none")
    ));
    assert!(!belt_is_sound(&t.replace("trait.sha256", "nothing")));
    assert!(!belt_is_sound(&t.replacen("fork:", "set:", 1)));
    assert!(!belt_is_sound(
        &t.replace("then: plan\n", "then: continue\n")
    ));
    assert!(!belt_is_sound(&t.replace("except `original`", "")));
    assert!(!belt_is_sound(
        &t.replace("name: doku-mitziehen", "name: x")
    ));
    assert!(!belt_is_sound(&t.replace(">= 3", ">= 99")));
    let strat = step_body(&t, "strategie-ok");
    let by_ai = strat.replace("actor: human", "actor: ai");
    assert!(!belt_is_sound(&t.replacen(&strat, &by_ai, 1)));
    let swapped = t
        .replacen("  - strategie-ok:", "  - zz:", 1)
        .replacen("  - stufe-3:", "  - strategie-ok:", 1)
        .replacen("  - zz:", "  - stufe-3:", 1);
    assert!(!belt_is_sound(&swapped));
}

fn testlauf_is_sound(t: &str) -> bool {
    let full = step_body(t, "voll");
    in_order(
        t,
        &[
            "geaendert",
            "slot",
            "gezielt",
            "gezielt-again",
            "fix",
            "end-weiche",
            "voll",
            "zeit",
            "voll-again",
        ],
    ) && ["geaendert", "slot", "gezielt", "voll", "zeit"]
        .iter()
        .all(|s| step_body(t, s).contains("actor: machine"))
        && step_body(t, "gezielt").contains("build-slot.sh")
        && step_body(t, "geaendert").contains("-p")
        && step_body(t, "geaendert").contains("--test")
        && full.contains("build-slot.sh")
        && full.contains("BACKGROUND")
        && full.contains("never the whole log")
        && step_body(t, "end-weiche").contains("== \"end\"")
        && step_body(t, "slot").contains("at most 3")
        && step_body(t, "fix").contains("then: gezielt")
        && escalates(t, "gezielt-again", "fix")
        && escalates(t, "voll-again", "fix")
}

#[test]
fn the_test_run_is_targeted_the_full_run_an_end_gate_in_the_background_and_timed() {
    assert!(testlauf_is_sound(&read("testlauf")));
}

#[test]
fn counter_a_full_run_while_working_or_in_the_foreground_is_caught() {
    let t = read("testlauf");
    assert!(!testlauf_is_sound(&t.replace("== \"end\"", "== \"immer\"")));
    assert!(!testlauf_is_sound(&t.replace("BACKGROUND", "FOREGROUND")));
    assert!(!testlauf_is_sound(
        &t.replace("never the whole log", "the whole log")
    ));
    assert!(!testlauf_is_sound(&t.replace("build-slot.sh", "cargo")));
    assert!(!testlauf_is_sound(&t.replace(">= 3", ">= 99")));
    let voll = step_body(&t, "voll");
    let by_ai = voll.replace("actor: machine", "actor: ai");
    assert!(!testlauf_is_sound(&t.replacen(&voll, &by_ai, 1)));
    assert!(!testlauf_is_sound(&t.replace("--test", "--all")));
}

#[test]
fn the_new_flows_are_checked_by_the_schema_check() {
    for f in ["koordinator-runde", "programm-grundgeruest", "testlauf"] {
        support::workflows::assert_red(f, "freesynergy.net:actor", |t| {
            t.replacen("freesynergy.net:actor: ", "freesynergy.net:acter: ", 1)
        });
    }
}
