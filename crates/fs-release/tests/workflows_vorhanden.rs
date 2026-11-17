//! Pattern: none (test). Rule of the user: before anything is built, look whether it exists or is only extended. The step `vorhanden-pruefen` is the FIRST
//! step of `geruest-heben`, `modul-neu`, `bausatz-neu`, `grundfassung`, `frontend-umbau` and `stil-neu`, its result `befund` needs a proof (gate) and is read by the next
//! step. Counter-tests on changed copies (not first, no proof check, `befund` not read).

mod support;
use support::flowcheck::again_ok;
use support::workflows::{in_order, read, step_body, step_names};

/// (flow, escalation, the step that reads `befund`).
const FLOWS: [(&str, &str, &str); 6] = [
    ("geruest-heben", "eskalation", "lage"),
    ("modul-neu", "eskalation", "nutzer-pruefen"),
    ("bausatz-neu", "eskalation", "nutzer-pruefen"),
    ("grundfassung", "grundfassung-eskalation", "name-pruefen"),
    ("frontend-umbau", "umbau-eskalation", "texte"),
    ("stil-neu", "eskalation", "dossier"),
];

fn ok(t: &str, esc: &str, reader: &str) -> bool {
    let names = step_names(t);
    let first = step_body(t, "vorhanden-pruefen");
    let check = step_body(t, "vorhanden-pruefen-check");
    names.first().is_some_and(|n| n == "vorhanden-pruefen")
        && in_order(
            t,
            &[
                "vorhanden-pruefen",
                "vorhanden-pruefen-check",
                "vorhanden-pruefen-again",
                reader,
            ],
        )
        && first.contains("MANDATORY FIRST STEP")
        && first.contains("INVENTUR-*.md")
        && first.contains("{ befund: . }")
        && first.contains("`neu` only WITH a reason and proof")
        && check.contains("call: befund-beleg:")
        && check.contains("without proof is RED")
        && again_ok(t, "vorhanden-pruefen-again", "vorhanden-pruefen", esc)
        && step_body(t, reader).contains("$context.befund")
}

fn passes(t: &str, esc: &str, reader: &str) -> bool {
    std::panic::catch_unwind(|| ok(t, esc, reader)).unwrap_or(false)
}

#[test]
fn vorhanden_pruefen_is_the_first_step_with_a_proof_gate_and_is_read_by_the_next_step() {
    for (f, esc, reader) in FLOWS {
        assert!(ok(&read(f), esc, reader), "{f}");
    }
}

#[test]
fn counter_not_first_no_proof_check_not_read_and_no_inventory_are_caught() {
    for (f, esc, reader) in FLOWS {
        let t = read(f);
        let bad = |x: String, why: &str| assert!(x != t && !passes(&x, esc, reader), "{f}: {why}");
        bad(
            t.replace("  - vorhanden-pruefen-check:", "  - vorhanden-belegen:"),
            "no check step",
        );
        bad(
            t.replace("without proof is RED", "is fine"),
            "proof not required",
        );
        bad(
            t.replace("call: befund-beleg:", "call: x:"),
            "no gate script",
        );
        bad(t.replace("INVENTUR-*.md", "x"), "no inventory");
        let body = step_body(&t, reader);
        bad(
            t.replace(&body, &body.replace("$context.befund", "x")),
            "befund not read",
        );
        bad(
            t.replace(
                "do:\n  - vorhanden-pruefen:",
                "do:\n  - zuerst:\n      then: continue\n  - vorhanden-pruefen:",
            ),
            "not first",
        );
    }
}
