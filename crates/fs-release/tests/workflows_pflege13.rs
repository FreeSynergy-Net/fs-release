//! Pattern: none (test). The pull request path of `upstream-anfrage` (same gates as an issue, attitude switch for Blinc, only the human sends) and
//! the cut of `entscheidung` (connected with `ask` and `record`), checked on the PARSED workflow (steps, switch cases, calls, `if`, `then`), not by text search.
//! The real file is green; every mutant (a change of the parsed tree) is red.

#[path = "workflows_pflege13/model.rs"]
mod model;
#[path = "workflows_pflege13/rules.rs"]
mod rules;

use model::*;
use rules::{cut_ok, pr_ok};
use serde_json::Value;

type Mut = (&'static str, fn(&mut Value));

#[test]
fn the_real_pull_request_path_is_green() {
    let d = load("upstream-anfrage");
    assert_eq!(pr_ok(&d), Ok(()));
}

#[test]
fn the_real_cut_is_connected_and_green() {
    assert_eq!(cut_ok(&load("entscheidung")), Ok(()));
}

const PR_MUTANTS: [Mut; 26] = [
    ("M1 no `if` at pr-hand-text", |d| {
        remove_key(d, "pr-hand-text", "if")
    }),
    ("M2 pr-weiche never red", |d| {
        set_when(d, "pr-weiche", "pr-wartet", "${ false }")
    }),
    ("M3 no check of fundstellen", |d| {
        replace_in_when(d, "fundstellen // []", "[]")
    }),
    ("M4 no pr case in art-weiche", |d| {
        drop_case(d, "art-weiche", "pr")
    }),
    ("M5 pr-rot without `then: end`", |d| {
        remove_key(d, "pr-rot", "then")
    }),
    ("M6 time limit 140", |d| {
        set_with(
            d,
            "pr-nach-antwort",
            "frist_tage",
            "${ .input.frist_tage // 140 }",
        )
    }),
    ("M7 pr-hand-text calls send-issue", |d| {
        set_text(d, "pr-hand-text", "call", "send-issue:0.0.17@fs")
    }),
    ("M8 no Blinc case", |d| {
        drop_case(d, "pr-haltung-weiche", "blinc")
    }),
    ("M9 Blinc condition false", |d| {
        set_when(d, "pr-haltung-weiche", "blinc", "${ false }")
    }),
    ("M10 no rules of the target", |d| {
        drop_step(d, "pr-ziel-regeln")
    }),
    ("M11 no pr-ki-weiche", |d| drop_step(d, "pr-ki-weiche")),
    ("M12 no duplicates", |d| drop_step(d, "pr-dubletten")),
    ("M13 duplicates do not stop", |d| {
        set_then(d, "pr-dubletten-weiche", "doppelt", "continue")
    }),
    ("M14 no text gate", |d| drop_step(d, "pr-text-gate")),
    ("M15 text gate hits do not stop", |d| {
        set_then(d, "pr-text-weiche", "treffer", "continue")
    }),
    ("M16 no copy block", |d| drop_step(d, "pr-kopierblock")),
    ("M17 no preview gate", |d| {
        drop_step(d, "pr-vorschau-weiche")
    }),
    ("M18 text gate moved behind the approval", |d| {
        move_step_to_end(d, "pr-text-gate")
    }),
    ("M19 AI note always on", |d| {
        set_with(d, "pr-text-gate", "ki_hinweis", "true")
    }),
    ("M20 Blinc keeps the note", |d| {
        edit(d, "pr-ohne-vermerk", |s| s["set"]["ki"] = true.into())
    }),
    ("M21 approval by the machine", |d| {
        set_meta(d, "pr-freigabe", "actor", "machine")
    }),
    ("M22 approval without gate", |d| {
        set_meta(d, "pr-freigabe", "gate", "none")
    }),
    ("M23 approval of the raw text", |d| {
        set_with(d, "pr-freigabe", "block", "${ .input.pr_text }")
    }),
    ("M24 pr-hand-text by the machine", |d| {
        set_meta(d, "pr-hand-text", "actor", "machine")
    }),
    ("M25 pr-ki-hinweis another call", |d| {
        set_text(d, "pr-ki-hinweis", "call", "note-history:0.0.17@fs")
    }),
    ("M26 mit-vermerk falls into ohne-vermerk", |d| {
        remove_key(d, "pr-mit-vermerk", "then")
    }),
];

fn replace_in_when(d: &mut Value, from: &str, to: &str) {
    for c in ["pr-wartet"] {
        let now = cases(d, "pr-weiche")
            .into_iter()
            .find(|x| x.0 == c)
            .unwrap()
            .1;
        set_when(d, "pr-weiche", c, &now.replace(from, to));
    }
}

#[test]
fn every_pull_request_mutant_is_red() {
    let orig = load("upstream-anfrage");
    for (name, m) in PR_MUTANTS {
        let mut d = orig.clone();
        m(&mut d);
        assert_ne!(d, orig, "{name} changed nothing");
        let why = pr_ok(&d)
            .err()
            .unwrap_or_else(|| panic!("{name} stays GREEN"));
        println!("RED {name}: {why}");
    }
}

const CUT_MUTANTS: [Mut; 6] = [
    ("C1 ask without cut.open", |d| {
        set_text(d, "ask", "if", "${ $context.verdict.open }")
    }),
    ("C2 record without cut.open", |d| {
        set_text(d, "record", "if", "${ $context.verdict.open }")
    }),
    ("C3 ask only by cut", |d| {
        set_text(d, "ask", "if", "${ $context.cut.open }")
    }),
    ("C4 cut without gate", |d| {
        set_meta(d, "cut", "gate", "none")
    }),
    ("C5 record gets no cut", |d| {
        remove_key_in_with(d, "record", "cut")
    }),
    ("C6 cut writes another field", |d| {
        edit(d, "cut", |s| {
            s["export"]["as"] = "${ $context + { x: . } }".into()
        })
    }),
];

fn remove_key_in_with(d: &mut Value, n: &str, k: &str) {
    edit(d, n, |s| {
        s["with"].as_object_mut().unwrap().remove(k);
    });
}

#[test]
fn every_cut_mutant_is_red() {
    let orig = load("entscheidung");
    for (name, m) in CUT_MUTANTS {
        let mut d = orig.clone();
        m(&mut d);
        assert_ne!(d, orig, "{name} changed nothing");
        let why = cut_ok(&d)
            .err()
            .unwrap_or_else(|| panic!("{name} stays GREEN"));
        println!("RED {name}: {why}");
    }
}

/// The counter-probe of the old tautology: a real change of the actor of `pr-freigabe` is red, the real file is not.
#[test]
fn the_counter_probe_of_the_human_gate_is_real() {
    let orig = load("upstream-anfrage");
    assert_eq!(meta(&orig, "pr-freigabe", "actor"), "human");
    let mut d = orig.clone();
    set_meta(&mut d, "pr-freigabe", "actor", "ai");
    assert!(pr_ok(&orig).is_ok() && pr_ok(&d).is_err());
}
