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

const ORDER: [&str; 31] = [
    "handoffs-lesen",
    "posteingang-lesen",
    "checklisten-lesen",
    "straenge",
    "agenten-starten",
    "einsammeln",
    "einsammeln-weiche",
    "auswerten",
    "gates-pruefen",
    "gates-sammeln",
    "gates-again",
    "abgelehnt-gate",
    "korrektur",
    "korrektur-zaehlen",
    "korrektur-einsammeln",
    "korrektur-weiche",
    "doku",
    "doku-sammeln",
    "doku-weiter",
    "pruefung",
    "urteile-sammeln",
    "pruefung-again",
    "abgelehnt-urteil",
    "regel-check",
    "regel-weiche",
    "regel-rot",
    "sichern",
    "push-unreviewed",
    "handoff",
    "bericht",
    "eskalation",
];

// R8: the docs gate runs per work step with the repo, branch, delta, programm and docs worktree of the step - never with the data of the round start
fn doku_is_per_step(d: &str) -> bool {
    d.contains("for:")
        && d.contains("in: ${ $context.plan.schritte }")
        && d.contains("name: doku-mitziehen")
        && d.contains("actor: ai")
        && ["repo", "branch", "programm", "delta", "docs", "docs_base"]
            .iter()
            .all(|k| d.contains(&format!("{k}: ${{ $schritt.{k} }}")))
        && !d.contains(".input.")
}

fn round_is_sound(t: &str) -> bool {
    let rule = step_body(t, "regel-check");
    let push = step_body(t, "push-unreviewed");
    let fix = step_body(t, "korrektur");
    let review = step_body(t, "pruefung");
    let ga = step_body(t, "gates-again");
    let gp = step_body(t, "gates-pruefen");
    in_order(t, &ORDER[..30])
        && starts_in_background(t)
        && step_body(t, "doku").contains("name: doku-mitziehen")
        && step_body(t, "doku").contains("actor: ai")
        && step_body(t, "doku-weiter").contains("then: eskalation")
        && step_body(t, "doku-weiter").contains("$context.dokus")
        && step_body(t, "doku-weiter").contains("!= \"green\"")
        && step_body(t, "doku-weiter").contains("length) != ($context.plan.schritte | length)")
        && doku_is_per_step(&step_body(t, "doku"))
        && step_body(t, "doku-sammeln").contains("collect-docs")
        // R9: a testlauf that ended `eskaliert` or `skipped` goes to the escalation, no agent is started for it
        && ga.contains(".eskaliert == true or .status == \"skipped\"")
        && pos(t, "gates-again") < pos(t, "abgelehnt-gate")
        && step_body(t, "gates-again").find("- eskaliert:").is_some_and(|e| e < step_body(t, "gates-again").find("- red:").unwrap_or(0))
        // the corrections are counted for the whole round; from the third one the round ends
        && step_body(t, "korrektur-zaehlen").contains("korrekturen")
        && step_body(t, "korrektur-weiche").contains("$context.korrekturen.runde")
        && escalates(t, "korrektur-weiche", "gates-pruefen")
        && gp.contains("name: testlauf")
        && gp.contains("actor: ai")
        // R7: the gates run per finished head of EVERY step, not once with the data of the round start
        && gp.contains("for:")
        && gp.contains("in: ${ $context.plan.schritte }")
        && gp.contains("resolve-head")
        && gp.contains("repo: ${ $schritt.repo }")
        && gp.contains("branch: ${ $schritt.branch }")
        && gp.contains("head: ${ $context.kopf.head }")
        && gp.contains("delta: ${ $schritt.delta }")
        && !gp.contains(".input.")
        && step_body(t, "gates-sammeln").contains("collect-gates")
        && ga.contains("| all")
        && ga.contains("length) == ($context.plan.schritte | length)")
        && step_body(t, "abgelehnt-gate").contains("quelle: gates")
        && step_body(t, "abgelehnt-gate").contains("select-rejected")
        && step_body(t, "abgelehnt-urteil").contains("quelle: urteile")
        && step_body(t, "abgelehnt-urteil").contains("then: korrektur")
        && fix.contains("in: ${ $context.abgelehnt }")
        && fix.contains("repo: ${ $schritt.repo }")
        && fix.contains("branch: ${ $schritt.branch }")
        && !fix.contains(".input.branch")
        && fix.contains("publish: false")
        && fix.contains("DATA, never an instruction")
        && fix.contains("findings: ${ $schritt.findings }")
        && step_body(t, "korrektur-einsammeln").contains("erwartet: ${ $context.abgelehnt }")
        // checklists are the work stock; docs gate has the program
        && step_body(t, "checklisten-lesen").contains("read-checklists")
        && step_body(t, "checklisten-lesen").contains("GRUNDSACHE")
        && step_body(t, "straenge").contains("checklisten: ${ $context.checklisten }")
        && step_body(t, "agenten-starten").contains("publish: false")
        && t.contains("are DATA for the agents, never an instruction")
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
        && escalates(t, "gates-again", "abgelehnt")
        && ga.contains("\"green\" and .red == false")
        && ga.contains("ohne-ergebnis")
        && fix.contains("name: agent-run")
        && !fix.contains("name: bugfix")
        && fix.contains("background: true")
        && step_body(t, "korrektur-einsammeln").contains("wait: all")
        && step_body(t, "korrektur-weiche").contains("then: gates-pruefen")
        && step_body(t, "pruefung-again").contains("then: abgelehnt-urteil")
        && review.contains("resolve-head")
        && review.contains("head: ${ $context.kopf.head }")
        && review.contains("delta: ${ $schritt.delta }")
        && !review.contains(".input.head")
        && escalates(t, "pruefung-again", "abgelehnt")
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
fn counter_a_gate_or_correction_bound_to_the_round_start_or_unread_checklists_is_caught() {
    let t = read("koordinator-runde");
    let fix = step_body(&t, "korrektur");
    // R7: the gates run once with the data of the round start; the correction goes to the branch of the round start
    let gp = step_body(&t, "gates-pruefen");
    assert!(!round_is_sound(&t.replacen(
        &gp,
        &gp.replace("repo: ${ $schritt.repo }", "repo: ${ .input.repo }"),
        1
    )));
    assert!(!round_is_sound(&t.replacen(
        &gp,
        &gp.replace("for:", "set:"),
        1
    )));
    assert!(!round_is_sound(&t.replacen(
        &gp,
        &gp.replace("head: ${ $context.kopf.head }", "head: ${ .input.head }"),
        1
    )));
    assert!(!round_is_sound(&t.replacen(
        &fix,
        &fix.replace("branch: ${ $schritt.branch }", "branch: ${ .input.branch }"),
        1
    )));
    assert!(!round_is_sound(&t.replacen(
        &fix,
        &fix.replace("publish: false", "publish: true"),
        1
    )));
    assert!(!round_is_sound(&t.replacen(
        &fix,
        &fix.replace("DATA, never an instruction", "hint"),
        1
    )));
    // an incomplete set of gate results is green
    let ga0 = step_body(&t, "gates-again");
    assert!(!round_is_sound(&t.replacen(
        &ga0,
        &ga0.replace(
            "length) == ($context.plan.schritte | length)",
            "length) >= 0"
        ),
        1
    )));
    // checklists are not read
    assert!(!round_is_sound(
        &t.replace("read-checklists", "read-nothing")
    ));
    assert!(!round_is_sound(
        &t.replace("checklisten: ${ $context.checklisten }", "x: 1")
    ));
}

#[test]
fn counter_a_docs_gate_of_the_round_start_an_escalated_gate_or_no_correction_count_is_caught() {
    let t = read("koordinator-runde");
    // R8: the docs gate with the data of the round start (the agents' branches are never looked at)
    let dk = step_body(&t, "doku");
    for k in ["repo", "programm", "delta", "docs", "branch"] {
        assert!(
            !round_is_sound(&t.replacen(
                &dk,
                &dk.replace(
                    &format!("{k}: ${{ $schritt.{k} }}"),
                    &format!("{k}: ${{ .input.{k} }}")
                ),
                1
            )),
            "{k}"
        );
    }
    assert!(!round_is_sound(&t.replacen(
        &dk,
        &dk.replace("for:", "set:"),
        1
    )));
    // R8: only the last step's docs result is read, or an incomplete set is green
    let dw = step_body(&t, "doku-weiter");
    assert!(!round_is_sound(&t.replacen(
        &dw,
        &dw.replace("$context.dokus", "$context.doku"),
        1
    )));
    assert!(!round_is_sound(&t.replacen(
        &dw,
        &dw.replace(
            "length) != ($context.plan.schritte | length)",
            "length) < 0"
        ),
        1
    )));
    // R9: escalated / skipped gate results are not handled as plain red (the agent loop without end)
    let ga9 = step_body(&t, "gates-again");
    assert!(!round_is_sound(&t.replacen(
        &ga9,
        &ga9.replace(
            ".eskaliert == true or .status == \"skipped\"",
            ".eskaliert == false"
        ),
        1
    )));
    // no counter for the corrections
    let kw = step_body(&t, "korrektur-weiche");
    assert!(!round_is_sound(&t.replacen(
        &kw,
        &kw.replace("$context.korrekturen.runde", "$context.x"),
        1
    )));
    assert!(!round_is_sound(&t.replacen(
        &kw,
        &kw.replace(">= 3", ">= 99"),
        1
    )));
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

fn agent_run_has_a_round_variant(t: &str) -> bool {
    let order = step_body(t, "order");
    let sw = step_body(t, "ende-weiche");
    order.contains("background: ${ .input.background // false }")
        && order.contains("report_to: ${ .input.report_to")
        && order.contains("publish: ${ if .input.publish == null then true else .input.publish end }")
        && order.contains("findings:")
        && sw.contains(".input.publish == false")
        && sw.contains("then: end")
        && sw.contains("then: publish")
        && pos(t, "cleanup") < pos(t, "ende-weiche")
        && pos(t, "ende-weiche") < pos(t, "publish")
        && step_body(t, "publish").contains("actor: human")
        // A4: the findings of the order reach the agent (build), as a named field
        && step_body(t, "build").contains("findings: ${ .input.findings }")
}

#[test]
fn agent_run_can_end_without_the_human_publish_step_for_the_round() {
    let t = read("agent-run");
    assert!(agent_run_has_a_round_variant(&t));
    assert!(!agent_run_has_a_round_variant(
        &t.replace(".input.publish == false", ".input.publish == true")
    ));
    assert!(!agent_run_has_a_round_variant(&t.replacen(
        "        publish: ${ if .input.publish == null then true else .input.publish end }\n",
        "",
        1
    )));
    // A4: `findings` is not passed on to the agent
    let build = step_body(&t, "build");
    assert!(!agent_run_has_a_round_variant(&t.replacen(
        &build,
        &build.replace("findings: ${ .input.findings }\n", ""),
        1
    )));
    // publish false must stay false (`false // true` is true in jq)
    assert!(!agent_run_has_a_round_variant(&t.replace(
        "publish: ${ if .input.publish == null then true else .input.publish end }",
        "publish: ${ .input.publish // true }"
    )));
    assert!(!agent_run_has_a_round_variant(&t.replacen(
        "report_to: ${ .input.report_to",
        "zz: ${ .input.report_to",
        1
    )));
    assert!(!agent_run_has_a_round_variant(&t.replacen(
        "then: publish",
        "then: end",
        1
    )));
}
