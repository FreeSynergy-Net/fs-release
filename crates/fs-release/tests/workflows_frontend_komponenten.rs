//! Pattern: none (test). Review template frontend: the step `frontend-nur-komponenten` (only component calls; logic or a widget is a finding that
//! goes to a conversation) stands in `grundfassung`, `frontend-round` and `frontend-umbau` before the picture acceptance. Counter-tests on changed copies.

mod support;
use support::workflows::{in_order, read, step_body};

const WHERE: [(&str, &str); 3] = [
    ("grundfassung", "picture-ok"),
    ("frontend-round", "acceptance"),
    ("frontend-umbau", "release"),
];

fn ok(text: &str, anchor: &str) -> bool {
    let check = step_body(text, "frontend-nur-komponenten");
    let sw = step_body(text, "frontend-nur-komponenten-weiche");
    let talk = step_body(text, "frontend-befund-talk");
    in_order(
        text,
        &[
            "frontend-nur-komponenten",
            "frontend-nur-komponenten-weiche",
            "frontend-befund-talk",
            anchor,
        ],
    ) && check.contains("frontend_crate: ${ .input.frontend_crate }")
        && check.contains("freesynergy.net:gate: opus-ja")
        && check.contains("ONLY the plugging together")
        && check.contains("FINDING")
        && sw.contains("then: frontend-befund-talk")
        && sw.contains(&format!("then: {anchor}"))
        && sw.contains(">= 3")
        && talk.contains("freesynergy.net:actor: human")
        && talk.contains("freesynergy.net:gate: human")
        && talk.contains("then: frontend-nur-komponenten")
}

/// A changed copy that lost a step the rule names makes the rule panic: that counts as red.
fn passes(text: &str, anchor: &str) -> bool {
    std::panic::catch_unwind(|| ok(text, anchor)).unwrap_or(false)
}

#[test]
fn the_step_stands_before_the_picture_acceptance_in_all_three_flows() {
    for (f, anchor) in WHERE {
        assert!(ok(&read(f), anchor), "{f}");
    }
}

#[test]
fn counter_a_missing_step_a_wrong_order_a_skipped_talk_and_an_agent_talk_are_caught() {
    for (f, anchor) in WHERE {
        let t = read(f);
        let t1 = t.replace("  - frontend-nur-komponenten:", "  - frontend-pruefen:");
        assert!(t1 != t && !passes(&t1, anchor), "{f} renamed");
        let t2 = t.replace("frontend_crate: ${ .input.frontend_crate }", "crate: x");
        assert!(t2 != t && !passes(&t2, anchor), "{f} input");
        let talk = step_body(&t, "frontend-befund-talk");
        let t3 = t.replace(&talk, &talk.replace("actor: human", "actor: ai"));
        assert!(t3 != t && !passes(&t3, anchor), "{f} talk");
        let t4 = t.replace("then: frontend-befund-talk", "then: continue");
        assert!(t4 != t && !passes(&t4, anchor), "{f} skipped talk");
        let t5 = t.replace("gate: opus-ja\n        freesynergy.net:kind: subjective\n        freesynergy.net:summary: \"Review template", "gate: none\n        freesynergy.net:kind: subjective\n        freesynergy.net:summary: \"Review template");
        assert!(t5 != t && !passes(&t5, anchor), "{f} gate");
    }
}

#[test]
fn review_text_readme_and_usage_state_the_template() {
    for text in [
        read("review"),
        std::fs::read_to_string(support::workflows::root().join("docs/usage.md")).unwrap(),
        std::fs::read_to_string(support::workflows::root().join("workflows/README.md")).unwrap(),
    ] {
        assert!(text.contains("frontend-nur-komponenten"));
    }
    let readme =
        std::fs::read_to_string(support::workflows::root().join("workflows/README.md")).unwrap();
    assert!(
        readme.contains("Review-Vorlage Frontend") && readme.contains("Review template frontend")
    );
}
