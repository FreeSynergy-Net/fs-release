//! Jumps and exact conditions of the PR path: no step may jump past the gates to `pr-freigabe`, and the conditions are compared whole (not by `contains`).

use super::model::*;
use serde_json::Value;

type R = Result<(), String>;

/// Targets every step of the PR chain may use without a reason: go on, or stop (held back / refused).
const STOPS: [&str; 4] = ["continue", "pr-rot", "abgelehnt", "ki-abgelehnt"];
/// The only explicit jumps inside the chain: (step, target).
const JUMPS: [(&str, &str); 2] = [
    ("pr-haltung-weiche", "pr-ohne-vermerk"),
    ("pr-mit-vermerk", "pr-dubletten"),
];

/// All targets of a step: its own `then` and the `then` of every switch case.
fn targets(d: &Value, n: &str) -> Vec<String> {
    let mut t = vec![field(d, n, "then").to_string()];
    t.extend(cases(d, n).into_iter().map(|c| c.2));
    t.retain(|x| !x.is_empty());
    t
}

pub fn jumps_ok(d: &Value) -> R {
    let from = pos(d, "pr-nach-antwort").ok_or("no pr-nach-antwort")?;
    let to = pos(d, "pr-vorschau-weiche").ok_or("no pr-vorschau-weiche")?;
    for (i, n) in names(d).iter().enumerate() {
        for t in targets(d, n) {
            if (from..=to).contains(&i)
                && !STOPS.contains(&t.as_str())
                && !JUMPS.contains(&(n.as_str(), t.as_str()))
            {
                return Err(format!("{n} jumps to {t}: past the gates"));
            }
            // only the order of the steps leads to the human, never a jump
            if t == "pr-freigabe" || t == "pr-hand-text" {
                return Err(format!("{n} jumps straight to {t}"));
            }
        }
    }
    Ok(())
}

/// (step, case, the whole condition).
const WHEN: [(&str, &str, &str); 7] = [
    (
        "pr-weiche",
        "pr-wartet",
        r"${ $context.pr_tor == null or $context.pr_tor.frei != true or ($context.pr_tor.fundstellen // [] | length) == 0 }",
    ),
    (
        "pr-ki-weiche",
        "ki-nein",
        r#"${ $context.ziel.ki == null or $context.ziel.fundstellen == null or ($context.ziel.fundstellen | length) == 0 or (["erlaubt", "verlangt", "ohne-vermerk", "abgelehnt"] | index($context.ziel.ki)) == null }"#,
    ),
    (
        "pr-haltung-weiche",
        "ziel-lehnt-ki-ab",
        r#"${ $context.ziel.ki == "abgelehnt" }"#,
    ),
    (
        "pr-haltung-weiche",
        "blinc",
        r#"${ .input.upstream == "blinc" }"#,
    ),
    (
        "pr-dubletten-weiche",
        "doppelt",
        r"${ $context.dubletten.treffer == null or ($context.dubletten.treffer | length) > 0 }",
    ),
    (
        "pr-text-weiche",
        "treffer",
        r"${ $context.rules.hits == null or $context.textcheck.hits == null or ($context.rules.hits | length) > 0 or ($context.textcheck.hits | length) > 0 }",
    ),
    (
        "pr-vorschau-weiche",
        "ohne-vorschau",
        r"${ $context.block == null or $context.block.vorschau_ok != true }",
    ),
];

pub fn exact_ok(d: &Value) -> R {
    for (n, case, want) in WHEN {
        let got = cases(d, n)
            .into_iter()
            .find(|c| c.0 == case)
            .ok_or(format!("{n} has no case {case}"))?;
        if got.1 != want {
            return Err(format!("{n}/{case}: the condition is not the expected one"));
        }
    }
    // a target that refuses AI contributions stops the PR
    let kanidm = case_to(d, "pr-haltung-weiche", "pr-rot");
    if kanidm.is_none() {
        return Err("a pull request to a target with ki == abgelehnt does not stop".into());
    }
    if pos(d, "pr-haltung-weiche") > pos(d, "pr-mit-vermerk") {
        return Err("pr-haltung-weiche after pr-mit-vermerk".into());
    }
    Ok(())
}

/// `ask` and `record` run exactly for `verdict.open or cut.open or entwurf.open`.
pub fn exact_cut_ok(d: &Value) -> R {
    for n in ["ask", "record"] {
        if field(d, n, "if") != "${ $context.verdict.open or $context.cut.open or $context.entwurf.open }" {
            return Err(format!(
                "{n}: the condition is not verdict.open or cut.open or entwurf.open"
            ));
        }
    }
    Ok(())
}
