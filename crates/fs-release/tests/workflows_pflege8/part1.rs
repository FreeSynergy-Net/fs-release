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

/// Moves the block of `step` directly behind the block of `after` (the layout is `  - name:` lines at level two).
pub(super) fn move_after(text: &str, step: &str, after: &str) -> String {
    let is_head = |l: &str| l.starts_with("  - ") && l.ends_with(':');
    let mut head = Vec::new();
    let mut blocks: Vec<(String, Vec<String>)> = Vec::new();
    for l in text.lines() {
        if is_head(l) {
            blocks.push((l.to_string(), vec![l.to_string()]));
        } else if let Some(b) = blocks.last_mut() {
            b.1.push(l.to_string());
        } else {
            head.push(l.to_string());
        }
    }
    let name = |s: &str| format!("  - {s}:");
    let i = blocks.iter().position(|b| b.0 == name(step)).unwrap();
    let moved = blocks.remove(i);
    let j = blocks.iter().position(|b| b.0 == name(after)).unwrap();
    blocks.insert(j + 1, moved);
    let mut out = head;
    out.extend(blocks.into_iter().flat_map(|b| b.1));
    out.join("\n") + "\n"
}

/// The main view: step after `fenster`, before `manifest`, with the rules of AUSSEHEN-SOFORT and AUSSEHEN-LEHREN.
pub(super) fn main_view(text: &str) -> bool {
    order(text, &["fenster", "hauptansicht", "manifest"])
        && has(
            text,
            "hauptansicht",
            &[
                "main-view-ai:0.0.17@fs",
                "src/preview.rs",
                "mode(capabilities)",
                "EXACTLY ONCE",
                "AUSSEHEN-LEHREN",
                "LEFT bar with symbols",
                "LARGE headings",
                "NO coloured badges",
                "empty state",
                "bilder-gate",
                "existing fs-components blocks",
            ],
        )
        && text.contains("call: main-view-ai:0.0.17@fs")
}

#[test]
fn main_view_step_comes_after_the_window() {
    let t = read("grundfassung");
    assert!(main_view(&t));
    assert!(!main_view(&t.replace("  - hauptansicht:", "  - ansicht:")));
    assert!(!main_view(&move_after(&t, "hauptansicht", "manifest")));
    assert!(!main_view(&t.replace("src/preview.rs", "src/model.rs")));
    assert!(!main_view(&t.replace("mode(capabilities)", "mode()")));
    assert!(!main_view(&t.replace("NO coloured badges", "badges")));
}

/// The picture pair gate: model | ours with the same state before the verdict; red goes to `bild-eskalation`.
pub(super) fn pair_gate(text: &str) -> bool {
    order(
        text,
        &[
            "bild",
            "waechter",
            "bildpaar",
            "bildpaar-weiche",
            "picture-ok",
        ],
    ) && has(
        text,
        "bildpaar",
        &[
            "SAME filter",
            "main-empty",
            "bilder-gate",
            "no tag without it",
        ],
    ) && has(
        text,
        "bildpaar-weiche",
        &["$context.bildpaar.red != false", "then: bild-eskalation"],
    ) && has(text, "picture-ok", &["paare: ${ $context.bildpaar }"])
        && has(text, "vor-tag", &["picture OK of the strategy (b7)"])
}

#[test]
fn picture_pair_gate_comes_before_the_verdict() {
    let t = read("grundfassung");
    assert!(pair_gate(&t));
    assert!(!pair_gate(
        &t.replace("  - bildpaar-weiche:", "  - bildpaar-aus:")
    ));
    assert!(!pair_gate(&move_after(&t, "bildpaar", "picture-ok")));
    assert!(!pair_gate(&t.replace("SAME filter", "filter")));
    assert!(!pair_gate(&t.replace(
        "$context.bildpaar.red != false }\n            then: bild-eskalation",
        "false }\n            then: continue"
    )));
    assert!(!pair_gate(
        &t.replace("paare: ${ $context.bildpaar }", "paare: x")
    ));
    assert!(!pair_gate(
        &t.replace("picture OK of the strategy (b7)", "picture OK")
    ));
}

/// The mandatory test `server-unresponsive`: in `binary-test`, in `gates`, in the lessons; a missing test is red.
pub(super) fn unresponsive(text: &str) -> bool {
    has(
        text,
        "binary-test",
        &[
            "`server-unresponsive`",
            "BOUNDED time",
            "a_hanging_instance_ends_the_start_with_an_error_in_bounded_time",
            "a missing test is RED",
        ],
    ) && has(text, "gates", &["`server-unresponsive`", "missing = red"])
        && has(
            text,
            "lehren-pruefen",
            &["(12) the test `server-unresponsive`", "missing = red"],
        )
        && order(text, &["binary-test", "lehren-pruefen", "gates"])
}

#[test]
fn test_server_unresponsive_is_mandatory_and_missing_is_red() {
    let t = read("grundfassung");
    assert!(unresponsive(&t));
    for (from, to) in [
        ("a missing test is RED", "a missing test is ok"),
        ("BOUNDED time", "some time"),
        ("(12) the test `server-unresponsive`", "(12) x"),
        (
            "the test `server-unresponsive` is one of the tests (missing = red)",
            "the tests",
        ),
    ] {
        assert!(!unresponsive(&t.replacen(from, to, 1)), "{from}");
    }
    assert!(t.contains("(g) MANDATORY test `server-unresponsive`"));
}

/// The bundle switch sits BEFORE `release`; not complete (or no answer) goes to `buendel-sammeln`, which ends.
pub(super) fn bundle(text: &str) -> bool {
    order(
        text,
        &[
            "vor-tag-weiche",
            "buendel-frage",
            "buendel-weiche",
            "release",
        ],
    ) && has(
        text,
        "buendel-frage",
        &[
            "Bundle complete? Otherwise keep collecting.",
            "functions",
            "fixes",
            "dependencies",
            "ONE release per program and day",
            "No answer is no yes",
        ],
    ) && has(
        text,
        "buendel-weiche",
        &[
            "$context.buendel.blocker == true",
            "$context.buendel.vollstaendig == true",
            "then: buendel-sammeln",
        ],
    ) && has(
        text,
        "buendel-sammeln",
        &["then: end", "nothing is tagged", "collecting"],
    ) && has_step(text, "buendel-sammeln")
}

#[test]
fn bundle_question_comes_before_release() {
    let t = read("grundfassung");
    assert!(bundle(&t));
    assert!(!bundle(&move_after(&t, "buendel-frage", "release")));
    assert!(!bundle(&move_after(&t, "buendel-weiche", "release")));
    assert!(!bundle(&t.replace("  - buendel-frage:", "  - frage:")));
    assert!(!bundle(
        &t.replace("then: buendel-sammeln", "then: continue")
    ));
    assert!(!bundle(&t.replace(
        "Bundle complete? Otherwise keep collecting.",
        "Complete?"
    )));
    assert!(!bundle(
        &t.replace("No answer is no yes", "No answer is yes")
    ));
    // a collecting step that does not end falls through into the next step
    let (head, tail) = t.split_once("  - buendel-sammeln:\n").unwrap();
    let open = format!(
        "{head}  - buendel-sammeln:\n{}",
        tail.replacen("then: end", "then: continue", 1)
    );
    assert!(!bundle(&open));
}

#[test]
fn head_rules_slot_look_bundle_are_kept() {
    let t = read("grundfassung");
    for p in [
        "(d) SLOT RULE (hint): WRITE code, tests, documentation and picture sketches without a build slot; BUILD only after the release of a slot",
        "(e) LOOK AT ONCE",
        "(f) BUNDLE ([BUENDEL-RELEASE]",
        "single release only for a blocker",
        "nearest EXISTING component",
        "may currently JUMP",
        "version = tag",
    ] {
        assert!(t.contains(p), "{p}");
        assert!(!t.replace(p, "x").contains(p), "{p}");
    }
    assert!(has(
        &t,
        "lehren-pruefen",
        &["(13) the slot rule is kept", "(11) main view present"]
    ));
    assert!(has(&t, "feature-liste", &["`vorbild`", "`hauptansicht`"]));
}
