//! Pattern: none (test). The step `generieren` of `grundfassung.yaml` (S5): the program is made from the blueprint with `fs-blueprint new`, no AI scaffold step.

mod support;
use support::workflows::{in_order, read, step_names};

/// The text of one step, or `None` when there is no such step (`step_body` of the support code panics then).
fn step_text(text: &str, step: &str) -> Option<String> {
    let body = text.split(&format!("  - {step}:\n")).nth(1)?;
    Some(body.split("\n  - ").next()?.to_string())
}

/// The step `generieren` makes the program from the blueprint with the command `fs-blueprint new` (no AI scaffold step), hands over `bauplan`, is a machine step, and
/// sits between the contract and the frontend crate; no step named `geruest` and no jump to it is left.
fn generates_from_the_blueprint(text: &str) -> bool {
    let names = step_names(text);
    let Some(body) = step_text(text, "generieren") else {
        return false;
    };
    names.iter().any(|n| n == "generieren")
        && !names.iter().any(|n| n == "geruest")
        && !text.contains("then: geruest")
        && !text.contains("call: scaffold-ai")
        && body.contains("call: fs-blueprint:")
        && body.contains("@fs")
        && body.contains("bauplan: ${ .input.bauplan }")
        && body.contains("freesynergy.net:actor: machine")
        && body.contains("`fs-blueprint new <bauplan>")
        && in_order(text, &["vertrag-standard", "generieren", "frontend-crate"])
}

#[test]
fn step_generieren_makes_the_program_from_the_blueprint() {
    let t = read("grundfassung");
    assert!(generates_from_the_blueprint(&t));
    for (from, to) in [
        (
            "bauplan: ${ .input.bauplan }",
            "bauplan: ${ .input.programm }",
        ),
        ("call: fs-blueprint:", "call: scaffold-ai:"),
        (
            "freesynergy.net:actor: machine",
            "freesynergy.net:actor: ai",
        ),
        ("`fs-blueprint new <bauplan>", "`something <bauplan>"),
        ("then: generieren", "then: geruest"),
        ("  - generieren:\n", "  - geruest:\n"),
    ] {
        // a change inside the step is made inside the step (the first match of the whole file may lie in another step)
        let body = step_text(&t, "generieren").unwrap();
        let weak = if body.contains(from) {
            t.replacen(&body, &body.replacen(from, to, 1), 1)
        } else {
            t.replacen(from, to, 1)
        };
        assert_ne!(weak, t, "{from}");
        assert!(!generates_from_the_blueprint(&weak), "{from}");
    }
}
