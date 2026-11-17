use super::*;

pub(super) fn has_step(text: &str, step: &str) -> bool {
    step_names(text).iter().any(|n| n == step)
}

/// Steps in this order (a missing step is no order: false, never a panic).
pub(super) fn order(text: &str, steps: &[&str]) -> bool {
    steps.iter().all(|s| has_step(text, s)) && in_order(text, steps)
}

pub(super) fn has(text: &str, step: &str, phrases: &[&str]) -> bool {
    has_step(text, step) && {
        let body = step_body(text, step);
        phrases.iter().all(|p| body.contains(p))
    }
}

/// The `  - name:` blocks of a file (the layout has the steps at level two); the lines before the first step are the head.
pub(super) fn split(text: &str) -> (Vec<String>, Vec<(String, Vec<String>)>) {
    let mut head = Vec::new();
    let mut blocks: Vec<(String, Vec<String>)> = Vec::new();
    for l in text.lines() {
        if l.starts_with("  - ") && l.ends_with(':') {
            blocks.push((l.to_string(), vec![l.to_string()]));
        } else if let Some(b) = blocks.last_mut() {
            b.1.push(l.to_string());
        } else {
            head.push(l.to_string());
        }
    }
    (head, blocks)
}

pub(super) fn join(head: Vec<String>, blocks: Vec<(String, Vec<String>)>) -> String {
    let mut out = head;
    out.extend(blocks.into_iter().flat_map(|b| b.1));
    out.join("\n") + "\n"
}

pub(super) fn without(text: &str, step: &str) -> String {
    let (head, mut blocks) = split(text);
    let at = blocks.iter().position(|b| b.0 == format!("  - {step}:"));
    blocks.remove(at.unwrap());
    join(head, blocks)
}

pub(super) fn move_after(text: &str, step: &str, after: &str) -> String {
    let (head, mut blocks) = split(text);
    let name = |s: &str| format!("  - {s}:");
    let i = blocks.iter().position(|b| b.0 == name(step)).unwrap();
    let moved = blocks.remove(i);
    let j = blocks.iter().position(|b| b.0 == name(after)).unwrap();
    blocks.insert(j + 1, moved);
    join(head, blocks)
}

pub(super) fn actor(text: &str, step: &str) -> String {
    step_body(text, step)
        .lines()
        .find_map(|l| l.trim().strip_prefix("freesynergy.net:actor: "))
        .unwrap_or("")
        .to_string()
}

pub(super) fn ends(text: &str, step: &str) -> bool {
    has_step(text, step) && step_body(text, step).contains("\n      then: end\n")
}

/// The loops end in the escalation: the `switch` has a case `runden` with `>= 3` that goes to `eskalation`, and a case that goes back.
pub(super) fn loop_ends(text: &str, switch: &str, back: &str) -> bool {
    has(
        text,
        switch,
        &[
            "- runden:",
            ">= 3",
            "then: eskalation",
            &format!("then: {back}"),
        ],
    )
}

// ---------------------------------------------------------------- stoerung-diagnose

pub(super) fn diagnose(t: &str) -> bool {
    order(
        t,
        &[
            "symptom-aufnehmen",
            "ort-pruefen",
            "ort-weiche",
            "logs-lesen",
            "hypothesen",
            "reproduktion",
            "repro-weiche",
            "ursache-belegen",
            "beleg-weiche",
            "kein-fix",
            "kein-fix-weiche",
            "bericht",
            "uebergabe",
            "nutzeranleitung",
            "eskalation",
        ],
    ) && has(
        t,
        "ort-pruefen",
        &[
            "real server",
            "session of the user account",
            "own directory",
        ],
    ) && has(t, "ort-weiche", &["then: nutzeranleitung"])
        && has(t, "logs-lesen", &["only reads", "real server"])
        && has(t, "reproduktion", &["OWN directory", "never a real server"])
        && loop_ends(t, "repro-weiche", "hypothesen")
        && has(t, "ursache-belegen", &["PROVES the cause", "belegt: false"])
        && loop_ends(t, "beleg-weiche", "hypothesen")
        && has(t, "kein-fix", &["NO FIX", "product code"])
        && has(t, "kein-fix-weiche", &["then: eskalation", "bugfix"])
        && has(t, "bericht", &["cause", "proof", "PROPOSAL"])
        && has(
            t,
            "uebergabe",
            &["NEW run", "Workflow bugfix", "never fixes"],
        )
        && ends(t, "uebergabe")
        && ends(t, "nutzeranleitung")
        && ends(t, "eskalation")
        && actor(t, "kein-fix") == "machine"
        && actor(t, "uebergabe") == "human"
        && !t.contains("\n      run:\n")
        && !t.contains("release-flow")
        && !t.contains("name: bugfix")
}

#[test]
fn diagnosis_is_read_only_in_the_own_directory_and_never_fixes() {
    let t = read("stoerung-diagnose");
    assert!(diagnose(&t));
    assert!(t.contains("KEIN-ZUGRIFF-AUF-NUTZERSITZUNG") && t.contains("KEIN-AGENT-AUF-SERVERN"));
    assert!(t.contains("BEFUND-einzelinstanz") && t.contains("REFUTED"));
}

#[test]
fn counter_diagnosis_mutants_are_caught() {
    let t = read("stoerung-diagnose");
    assert!(!diagnose(&without(&t, "kein-fix")));
    assert!(!diagnose(&without(&t, "ort-pruefen")));
    assert!(!diagnose(&move_after(&t, "kein-fix", "bericht")));
    assert!(!diagnose(&move_after(
        &t,
        "reproduktion",
        "ursache-belegen"
    )));
    assert!(!diagnose(
        &t.replace("then: nutzeranleitung", "then: continue")
    ));
    assert!(!diagnose(
        &t.replace("never a real server", "a real server")
    ));
    assert!(!diagnose(&t.replace("NO FIX", "a small fix")));
    // a fall-through into the escalation: the hand-over no longer ends
    assert!(!diagnose(&t.replacen("      then: end\n", "", 1)));
    // the loop without the third round
    assert!(!diagnose(&t.replace(">= 3", ">= 9")));
    // a fix in the same run: a sub-workflow `bugfix` is called
    assert!(!diagnose(&t.replace(
        "  - uebergabe:\n",
        "  - uebergabe:\n      run:\n        workflow:\n          name: bugfix\n"
    )));
    assert_red("stoerung-diagnose", "no step of this flow", |x| {
        x.replacen("then: nutzeranleitung", "then: nirgends", 1)
    });
}

// ---------------------------------------------------------------- branches-aufraeumen

/// Words that mean a removal; an agent or machine step (not the two that describe the user script) must contain none of them.
pub(super) const REMOVAL: [&str; 13] = [
    " rm ",
    "`rm",
    "rm -",
    "delete",
    "--delete",
    "prune",
    "branch -d",
    "rmi",
    "volume rm",
    "--apply",
    "unlink",
    "shred",
    "push",
];

pub(super) fn no_removal_by_agents(t: &str) -> bool {
    step_names(t).iter().all(|s| {
        let a = actor(t, s);
        let body = step_body(t, s).to_lowercase();
        let user_script = s == "skript-erzeugen" || s == "skript-pruefen";
        a == "human" || user_script || REMOVAL.iter().all(|w| !body.contains(w))
    })
}
