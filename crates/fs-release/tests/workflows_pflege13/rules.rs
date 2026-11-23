//! The rules the pull request path of `upstream-anfrage` and the cut of `entscheidung` must keep, checked on the parsed tree.
//! `Err` names the first rule that is broken.

use super::model::*;
use serde_json::Value;

type R = Result<(), String>;

fn need(ok: bool, msg: &str) -> R {
    if ok {
        Ok(())
    } else {
        Err(msg.to_string())
    }
}

/// The PR path, in this order: after `pr-weiche` every gate of the issue path, then the human.
const CHAIN: [&str; 17] = [
    "art-weiche",
    "pr-nach-antwort",
    "pr-weiche",
    "pr-ziel-regeln",
    "pr-erstkontakt-lesen",
    "pr-ki-weiche",
    "pr-ki-hinweis",
    "pr-haltung-weiche",
    "pr-mit-vermerk",
    "pr-ohne-vermerk",
    "pr-dubletten",
    "pr-dubletten-weiche",
    "pr-regeln",
    "pr-text-gate",
    "pr-text-weiche",
    "pr-kopierblock",
    "pr-vorschau-weiche",
];

/// A PR gate calls the SAME call as its issue step (reuse, no copy).
const SAME_CALL: [(&str, &str); 6] = [
    ("pr-ziel-regeln", "ziel-regeln-lesen"),
    ("pr-erstkontakt-lesen", "erstkontakt-lesen"),
    ("pr-ki-hinweis", "ki-hinweis"),
    ("pr-dubletten", "dubletten"),
    ("pr-regeln", "rules"),
    ("pr-text-gate", "text-gate"),
];

fn chain(d: &Value) -> R {
    let mut last = None;
    for n in CHAIN
        .iter()
        .chain(["pr-freigabe", "pr-hand-text", "pr-rot"].iter())
    {
        let p = pos(d, n).ok_or(format!("step {n} is missing"))?;
        need(
            last.is_none_or(|l| p > l),
            &format!("step {n} is out of order"),
        )?;
        last = Some(p);
    }
    need(
        pos(d, "pr-kopierblock").is_some_and(|k| k < pos(d, "pr-freigabe").unwrap()),
        "copy block after the approval",
    )
}

fn route(d: &Value) -> R {
    let art = cases(d, "art-weiche");
    need(
        art.iter()
            .any(|c| c.1.contains("art.art == \"pr\"") && c.2 == "pr-nach-antwort"),
        "art-weiche has no pr case",
    )?;
    need(
        call(d, "pr-nach-antwort") == "pr-gate",
        "pr-nach-antwort is not pr-gate",
    )?;
    need(
        with(d, "pr-nach-antwort", "frist_tage") == "${ .input.frist_tage // 14 }",
        "time limit is not 14 days",
    )?;
    let red = case_to(d, "pr-weiche", "pr-rot").ok_or("pr-weiche never goes to pr-rot")?;
    for w in ["pr_tor == null", "frei != true", "fundstellen // []"] {
        need(red.contains(w), &format!("pr-weiche: {w} missing"))?;
    }
    need(
        cases(d, "pr-weiche")
            .iter()
            .any(|c| c.1.is_empty() && c.2 == "continue"),
        "pr-weiche has no way on",
    )?;
    need(field(d, "pr-rot", "then") == "end", "pr-rot does not end")?;
    need(
        field(d, "record", "then") == "end",
        "the issue path does not end",
    )?;
    need(
        step(d, "pr-rot").is_some_and(|s| s["set"]["status"] == "held-back"),
        "pr-rot is not held back",
    )?;
    for (n, to, w) in [
        ("pr-ki-weiche", "ki-abgelehnt", "fundstellen == null"),
        ("pr-dubletten-weiche", "abgelehnt", "treffer == null"),
        ("pr-text-weiche", "pr-rot", "hits == null"),
        ("pr-vorschau-weiche", "abgelehnt", "vorschau_ok != true"),
    ] {
        let when = case_to(d, n, to).ok_or(format!("{n} never goes to {to}"))?;
        need(when.contains(w), &format!("{n}: {w} missing"))?;
        need(
            cases(d, n)
                .iter()
                .any(|c| c.1.is_empty() && c.2 == "continue"),
            &format!("{n} has no way on"),
        )?;
    }
    Ok(())
}

fn gates_and_attitude(d: &Value) -> R {
    for (p, i) in SAME_CALL {
        need(
            !call(d, p).is_empty() && call(d, p) == call(d, i),
            &format!("{p} is not the call of {i}"),
        )?;
    }
    need(
        call(d, "pr-kopierblock") == call(d, "kopierblock"),
        "pr-kopierblock is not copy-block",
    )?;
    // Blinc: no AI note; every other target: the note by the rules of the target
    let blinc = case_to(d, "pr-haltung-weiche", "pr-ohne-vermerk").ok_or("no Blinc way")?;
    need(
        blinc.contains("upstream == \"blinc\""),
        "the Blinc condition is missing",
    )?;
    need(
        field(d, "pr-mit-vermerk", "then") == "pr-dubletten",
        "pr-mit-vermerk falls into pr-ohne-vermerk",
    )?;
    need(
        step(d, "pr-ohne-vermerk").is_some_and(|s| s["set"]["ki"] == false),
        "pr-ohne-vermerk does not set false",
    )?;
    let ex = |n: &str| {
        step(d, n)
            .map(|s| s["export"]["as"].to_string())
            .unwrap_or_default()
    };
    need(
        ex("pr-ohne-vermerk").contains("vermerk: ."),
        "pr-ohne-vermerk exports no vermerk",
    )?;
    need(
        ex("pr-mit-vermerk").contains("vermerk: ."),
        "pr-mit-vermerk exports no vermerk",
    )?;
    let mit = step(d, "pr-mit-vermerk")
        .map(|s| s["set"]["ki"].to_string())
        .unwrap_or_default();
    need(
        mit.contains("$context.ki_hinweis == true"),
        "pr-mit-vermerk ignores ki_hinweis",
    )?;
    need(
        with(d, "pr-text-gate", "ki_hinweis") == "${ $context.vermerk.ki }",
        "text gate does not use vermerk.ki",
    )?;
    need(
        with(d, "pr-text-gate", "draft") == "${ .input.pr_text }",
        "text gate checks another text",
    )?;
    need(
        with(d, "pr-kopierblock", "draft") == "${ .input.pr_text }",
        "copy block of another text",
    )?;
    need(
        with(d, "pr-regeln", "ziel") == "${ $context.ziel }",
        "pr-regeln without the rules of the target",
    )
}

fn human(d: &Value) -> R {
    need(
        call(d, "pr-freigabe") == "approve",
        "pr-freigabe is not approve",
    )?;
    need(
        meta(d, "pr-freigabe", "actor") == "human" && meta(d, "pr-freigabe", "gate") == "human",
        "pr-freigabe is not human",
    )?;
    need(
        with(d, "pr-freigabe", "block") == "${ $context.block }",
        "approval of another text than the copy block",
    )?;
    need(
        call(d, "pr-hand-text") == "hand-text",
        "pr-hand-text is not hand-text",
    )?;
    need(
        field(d, "pr-hand-text", "if") == "${ $context.freigabe.ja == true }",
        "pr-hand-text without the YES",
    )?;
    need(
        field(d, "pr-hand-text", "then") == "end",
        "pr-hand-text does not end",
    )?;
    need(
        meta(d, "pr-hand-text", "actor") == "human",
        "pr-hand-text is not human",
    )?;
    need(
        with(d, "pr-hand-text", "block") == "${ $context.block }",
        "hand-text of another text",
    )?;
    for n in super::model::names(d)
        .iter()
        .filter(|n| n.starts_with("pr-"))
    {
        need(
            !["send-issue", "choose-channel", "blinc-channel"].contains(&call(d, n)),
            &format!("{n} sends"),
        )?;
    }
    Ok(())
}

pub fn pr_ok(d: &Value) -> R {
    chain(d)?;
    route(d)?;
    gates_and_attitude(d)?;
    human(d)
}

/// `cut` is connected with `ask` and `record`.
pub fn cut_ok(d: &Value) -> R {
    need(
        meta(d, "cut", "gate") == "strategie-ok",
        "cut has no gate strategie-ok",
    )?;
    let ex = step(d, "cut")
        .map(|s| s["export"]["as"].to_string())
        .unwrap_or_default();
    need(ex.contains("cut: ."), "cut does not write $context.cut")?;
    for n in ["ask", "record"] {
        let f = field(d, n, "if");
        need(
            f.contains("$context.cut.open") && f.contains("$context.verdict.open"),
            &format!("{n} does not react to cut.open"),
        )?;
    }
    need(
        with(d, "record", "cut") == "${ $context.cut }",
        "record does not get the cut",
    )?;
    need(pos(d, "cut") < pos(d, "ask"), "cut after ask")
}
