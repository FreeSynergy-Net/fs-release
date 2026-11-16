//! Pattern: none (test). Wiring of `doku-aufraeumen` and its callers: the mark lies outside the checkout, the branch name carries date and time,
//! `bestand` gets the retired list, no push inside the tidy-up, only `escalated` stops the callers, `befunde_offen` is a message, the push gate comes after the report.

mod support;
use support::workflows::{in_order, read, step_body, step_names};

fn tidy_is_sound(t: &str) -> bool {
    let names = step_names(t);
    !t.contains(".fs/")
        && t.contains("freesynergy/doku-aufraeumen/<repo>.marke")
        && step_body(t, "bestand").contains("retired: ${ .input.retired }")
        && !names.iter().any(|n| n == "push")
        && !t.contains("call: push-unreviewed")
        && step_body(t, "fertig").contains("push_offen: true")
        && step_body(t, "befunde-offen").contains("actor: machine")
        && step_body(t, "befunde-offen").contains("gate: none")
        && step_body(t, "uebersprungen").contains("eskaliert: false")
        && step_body(t, "weiche").contains("then: uebersprungen")
        && t.contains("YYYYMMDD-HHMM")
}

fn callers_are_sound(release: &str, runde: &str) -> bool {
    let weiter = step_body(release, "aufraeumen-weiter");
    release.contains("%Y%m%d-%H%M")
        && runde.contains("%Y%m%d-%H%M")
        && weiter.contains("status == \"escalated\"")
        && !weiter.contains("!= \"green\"")
        && in_order(
            runde,
            &["bericht", "doku-push-weiche", "doku-push", "eskalation"],
        )
        && step_body(runde, "bericht").contains("then: doku-push-weiche")
        && step_body(runde, "doku-push").contains("gate: strategie-ok")
        && step_body(runde, "doku-push").contains("actor: human")
        && step_body(runde, "doku-push-weiche").contains("push_offen == true")
}

#[test]
fn the_tidy_up_keeps_the_mark_outside_and_does_not_push() {
    assert!(tidy_is_sound(&read("doku-aufraeumen")));
}

#[test]
fn the_callers_only_stop_on_escalated_and_push_behind_the_report() {
    assert!(callers_are_sound(
        &read("release-flow"),
        &read("koordinator-runde")
    ));
}

#[test]
fn counter_a_mark_in_the_checkout_a_push_step_a_human_message_or_a_missing_retired_list_is_caught()
{
    let t = read("doku-aufraeumen");
    assert!(!tidy_is_sound(
        &t.replace("freesynergy/doku-aufraeumen/<repo>.marke", ".fs/x")
    ));
    assert!(!tidy_is_sound(
        &t.replace("retired: ${ .input.retired }", "")
    ));
    assert!(!tidy_is_sound(&t.replacen(
        "  - fertig:",
        "  - push:\n      call: x\n  - fertig:",
        1
    )));
    let b = step_body(&t, "befunde-offen");
    assert!(!tidy_is_sound(&t.replacen(
        &b,
        &b.replace("actor: machine", "actor: human"),
        1
    )));
    assert!(!tidy_is_sound(
        &t.replace("then: uebersprungen", "then: eskalation")
    ));
    assert!(!tidy_is_sound(&t.replace("YYYYMMDD-HHMM", "YYYY-MM-DD")));
}

#[test]
fn counter_a_stop_on_not_green_a_date_only_branch_or_a_push_before_the_report_is_caught() {
    let (r, k) = (read("release-flow"), read("koordinator-runde"));
    assert!(!callers_are_sound(
        &r.replace("status == \"escalated\"", "status != \"green\""),
        &k
    ));
    assert!(!callers_are_sound(
        &r.replace("%Y%m%d-%H%M", "%Y-%m-%d"),
        &k
    ));
    assert!(!callers_are_sound(
        &r,
        &k.replace("then: doku-push-weiche", "then: end")
    ));
    assert!(!callers_are_sound(
        &r,
        &k.replacen("gate: strategie-ok", "gate: none", 1)
    ));
}
