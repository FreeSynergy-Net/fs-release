//! Pattern: none (test). Rule CI-BELEG on the PARSED workflows `agent-run` and `review`: the CI at the exact head is read by the run itself, a report or a `green`
//! without run ID, conclusion and exact head is not passed on to the review. The real files are green; every mutant (a change of the parsed tree) is red.

#[path = "workflows_pflege13/model.rs"]
#[allow(dead_code)]
mod model;

use model::*;
use serde_json::Value;

type R = Result<(), String>;

const WHEN: &str = "${ $context.ci == null or ($context.ci.runs // [] | length) == 0 or (($workflow.input.erwartete_laeufe // []) | length) == 0 or ([$context.ci.runs[] | select(.id == null or .conclusion == null or .conclusion == \"\" or .headSha != $workflow.input.head)] | length) > 0 or ([($workflow.input.erwartete_laeufe // [])[] as $n | select(([$context.ci.runs[].name] | index($n)) == null) | $n] | length) > 0 }";

fn need(ok: bool, msg: &str) -> R {
    if ok {
        Ok(())
    } else {
        Err(msg.to_string())
    }
}

/// `file` has the reading step `read_step` before the review step `after`, the gate step right behind it.
fn ci_proof(d: &Value, read_step: &str, after: &str) -> R {
    need(
        call(d, read_step) == "branch-ci",
        "the CI is not read by branch-ci",
    )?;
    need(
        with(d, read_step, "head") == "${ .input.head }",
        "the CI is not read at the exact head",
    )?;
    need(
        step(d, read_step).is_some_and(|s| s["export"]["as"] == "${ $context + { ci: . } }"),
        "the CI result is not kept",
    )?;
    let gate = cases(d, "ci-beleg-weiche");
    need(
        gate.iter()
            .any(|c| c.0 == "ohne-lauf-id" && c.1 == WHEN && c.2 == "end"),
        "no stop without run ID, conclusion or exact head",
    )?;
    need(
        gate.iter().any(|c| c.1.is_empty() && c.2 == "continue"),
        "no way on for a complete proof",
    )?;
    let (r, g, a) = (pos(d, read_step), pos(d, "ci-beleg-weiche"), pos(d, after));
    need(
        r.is_some() && g == r.map(|x| x + 1) && g < a,
        "read, gate and review are not in this order",
    )?;
    need(
        meta(d, "ci-beleg-weiche", "actor") == "machine",
        "the gate is not a machine step",
    )
}

fn agent_run(d: &Value) -> R {
    ci_proof(d, "ci-beleg", "review")
}

fn review(d: &Value) -> R {
    ci_proof(d, "ci-lesen", "read")
}

type Mut = (&'static str, fn(&mut Value));

const MUTANTS: [Mut; 8] = [
    ("G1 no gate step", |d| drop_step(d, "ci-beleg-weiche")),
    ("G2 stop case continues", |d| {
        set_then(d, "ci-beleg-weiche", "ohne-lauf-id", "continue");
    }),
    ("G3 condition without the run ID", |d| {
        let w = WHEN.replace("select(.id == null or ", "select(");
        set_when(d, "ci-beleg-weiche", "ohne-lauf-id", &w);
    }),
    ("G4 condition without the head", |d| {
        let w = WHEN.replace(" or .headSha != $workflow.input.head", "");
        set_when(d, "ci-beleg-weiche", "ohne-lauf-id", &w);
    }),
    ("G5 CI of another head", |d| {
        for n in ["ci-beleg", "ci-lesen"] {
            if step(d, n).is_some() {
                set_with(d, n, "head", "${ .input.branch }");
            }
        }
    }),
    ("G7 empty conclusion counts as proof", |d| {
        let w = WHEN.replace(" or .conclusion == \"\"", "");
        set_when(d, "ci-beleg-weiche", "ohne-lauf-id", &w);
    }),
    ("G8 expected runs not checked", |d| {
        let i = WHEN
            .find(" or ([($workflow.input.erwartete_laeufe")
            .unwrap();
        set_when(
            d,
            "ci-beleg-weiche",
            "ohne-lauf-id",
            &format!("{} }}", &WHEN[..i]),
        );
    }),
    ("G6 gate behind the review", |d| {
        move_step_to_end(d, "ci-beleg-weiche");
    }),
];

#[test]
fn the_real_files_are_green() {
    assert_eq!(agent_run(&load("agent-run")), Ok(()));
    assert_eq!(review(&load("review")), Ok(()));
}

#[test]
fn every_mutant_is_red_in_both_files() {
    for (file, check) in [
        ("agent-run", agent_run as fn(&Value) -> R),
        ("review", review),
    ] {
        let orig = load(file);
        for (name, m) in MUTANTS {
            let mut d = orig.clone();
            m(&mut d);
            assert_ne!(d, orig, "{file} {name} changed nothing");
            let why = check(&d)
                .err()
                .unwrap_or_else(|| panic!("{file} {name} stays GREEN"));
            println!("RED {file} {name}: {why}");
        }
    }
}
