use super::*;

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
    // no counter for the corrections, a missing counter taken as 0, a higher limit, no check before the agents start
    let ks = step_body(&t, "korrektur-stand");
    assert!(!round_is_sound(&t.replacen(
        &ks,
        &ks.replace("$context.korrekturen.runde >= 3", "$context.x >= 3"),
        1
    )));
    assert!(!round_is_sound(&t.replacen(
        &ks,
        &ks.replace(">= 3", ">= 99"),
        1
    )));
    assert!(!round_is_sound(&t.replacen(
        &ks,
        &ks.replace("| type) != \"number\"", "// 0) != 7"),
        1
    )));
    assert!(!round_is_sound(&t.replacen("runde: 0\n", "runde: 7\n", 1)));
    assert!(!round_is_sound(&t.replace(
        "$context.korrekturen.runde + 1",
        "($context.korrekturen.runde // 0) + 1"
    )));
    assert!(!round_is_sound(
        &t.replace("korrektur-stand:", "korrektur-weg:")
    ));
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

pub(super) fn agent_run_has_a_round_variant(t: &str) -> bool {
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
