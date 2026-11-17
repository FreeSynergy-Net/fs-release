//! Pattern: none (test). Every step of every file under `workflows/` is reachable from the first step: the successor of a step is its `then`,
//! otherwise the next step; the cases of a `switch` count one by one (a case without `then` goes to the next step); `end` and `exit` stop.
//! A step that nobody reaches is dead (lesson katalog 7efabd09: `kette-rebase` had no way in; trait-delta: `review-weiche` fell into `auflagen-fix`). No exception list.

mod support;
use std::collections::{BTreeSet, HashMap};
use support::workflows::{read, FILES};

fn target(t: &str) -> Option<String> {
    t.strip_prefix("then: ").map(|v| v.trim().to_string())
}

/// The steps that no path from the first step reaches (in file order).
pub fn unreachable_steps(text: &str) -> Vec<String> {
    let mut steps: Vec<(String, Vec<String>)> = Vec::new();
    for line in text.lines() {
        if let Some(n) = line.strip_prefix("  - ").and_then(|l| l.strip_suffix(':')) {
            steps.push((n.to_string(), Vec::new()));
        } else if !line.is_empty() && !line.starts_with(' ') {
            // a top-level key (`output:`) ends the list of steps
            if let Some(last) = steps.last_mut() {
                last.1.push("\u{0}stop".into());
            }
        } else if let Some(last) = steps.last_mut() {
            if !last.1.iter().any(|l| l == "\u{0}stop") {
                last.1.push(line.to_string());
            }
        }
    }
    let names: Vec<String> = steps.iter().map(|s| s.0.clone()).collect();
    let mut succ: HashMap<String, Vec<String>> = HashMap::new();
    for (i, (n, body)) in steps.iter().enumerate() {
        let next = names.get(i + 1).cloned().unwrap_or_else(|| "end".into());
        let norm = |t: String| if t == "continue" { next.clone() } else { t };
        let targets: Vec<String> = if body.iter().any(|l| l == "      switch:") {
            let mut cases: Vec<Option<String>> = Vec::new();
            for l in body {
                if l.starts_with("        - ") && l.ends_with(':') {
                    cases.push(None);
                } else if let (Some(t), Some(c)) = (
                    l.strip_prefix("            ").and_then(target),
                    cases.last_mut(),
                ) {
                    *c = Some(t);
                }
            }
            cases
                .into_iter()
                .map(|c| norm(c.unwrap_or_else(|| "continue".into())))
                .collect()
        } else {
            let top = body.iter().find_map(|l| {
                l.strip_prefix("      ")
                    .filter(|r| !r.starts_with(' '))
                    .and_then(target)
            });
            vec![norm(top.unwrap_or_else(|| "continue".into()))]
        };
        succ.insert(n.clone(), targets);
    }
    let mut seen: BTreeSet<String> = BTreeSet::new();
    let mut todo: Vec<String> = names.first().cloned().into_iter().collect();
    while let Some(n) = todo.pop() {
        if let Some(next) = succ.get(&n) {
            if seen.insert(n) {
                todo.extend(next.iter().cloned());
            }
        }
    }
    names.into_iter().filter(|n| !seen.contains(n)).collect()
}

#[test]
fn every_step_of_every_workflow_is_reachable_from_the_first_step() {
    let mut dead = Vec::new();
    for f in FILES {
        for s in unreachable_steps(&read(f)) {
            dead.push(format!("{f}:{s}"));
        }
    }
    assert!(dead.is_empty(), "unreachable steps: {dead:?}");
}

#[test]
fn counter_trait_delta_green_review_must_not_fall_into_the_fix() {
    let text = read("trait-delta");
    assert_eq!(unreachable_steps(&text), [] as [std::string::String; 0]);
    let case = "and $context.review.hash == $context.probe.hash }'\n            then: hash\n";
    assert!(text.contains(case));
    let bad = text.replace(case, &case.replace("then: hash", "then: continue"));
    let dead = unreachable_steps(&bad);
    assert!(dead.contains(&"hash".to_string()) && dead.contains(&"fertig".to_string()));
    let bad2 = text.replace(case, &case.replace("then: hash", "then: auflagen-fix"));
    assert!(unreachable_steps(&bad2).contains(&"fertig".to_string()));
}

#[test]
fn counter_katalog_without_the_chain_switch_is_caught() {
    let text = read("katalog");
    assert_eq!(unreachable_steps(&text), [] as [std::string::String; 0]);
    // the version 7efabd09: no switch after `alt-kopf`, `kette-rebase` has no way in
    let body = support::workflows::step_body(&text, "kette-weiche");
    let old = text.replacen(&format!("  - kette-weiche:\n{body}\n"), "", 1);
    assert_ne!(old, text);
    assert_eq!(unreachable_steps(&old), vec!["kette-rebase".to_string()]);
    // the switch exists but the later entries go to `eintrag`: `kette-rebase` is dead again
    let wrong = text.replace(
        "            then: kette-rebase\n",
        "            then: eintrag\n",
    );
    assert!(unreachable_steps(&wrong).contains(&"kette-rebase".to_string()));
}

#[test]
fn counter_a_fall_through_and_a_dead_step_are_caught() {
    let text = "do:\n  - a:\n      then: c\n  - b:\n      then: end\n  - c:\n      switch:\n        - x:\n            when: 'y'\n            then: end\n        - z:\n            then: continue\n  - d:\n      then: end\noutput:\n  as: x\n";
    assert_eq!(unreachable_steps(text), vec!["b".to_string()]);
}
