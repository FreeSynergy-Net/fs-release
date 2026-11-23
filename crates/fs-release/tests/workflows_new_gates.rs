//! Pattern: none (test). The new gates of `upstream-anfrage` (sender, preview, first contact, AI stance, send only by a human),
//! each with counter-tests on changed copies. The flow order and the older rules are in `workflows_new.rs`.

mod support;
use support::workflows::{read, step_body, step_names};

fn position(text: &str, step: &str) -> usize {
    step_names(text)
        .iter()
        .position(|n| n == step)
        .unwrap_or_else(|| panic!("no step {step}"))
}

// ------------------------------------------------------------------ upstream-anfrage: the gates of the texts to the outside

fn has_all(body: &str, needles: &[&str]) -> bool {
    needles.iter().all(|n| body.contains(n))
}

/// `absender-weiche`: the bot is refused at the first contact, at Kanidm, at a target that refuses AI and without `ziel.bot`; before the draft.
fn sender_gate(t: &str) -> bool {
    has_all(
        &step_body(t, "absender-weiche"),
        &[
            ".input.absender == \"bot\"",
            "$context.erstkontakt.vorhanden != true",
            ".input.upstream == \"kanidm\"",
            "$context.ziel.ki == \"abgelehnt\"",
            "$context.ziel.bot != \"erlaubt\"",
            "then: abgelehnt",
        ],
    ) && position(t, "erstkontakt-lesen") < position(t, "absender-weiche")
        && position(t, "absender-weiche") < position(t, "draft")
}

/// `vorschau-weiche`: without a checked preview nothing goes on; it stands between the copy block and every way out.
fn preview_gate(t: &str) -> bool {
    has_all(
        &step_body(t, "vorschau-weiche"),
        &["$context.block.vorschau_ok != true", "then: abgelehnt"],
    ) && position(t, "kopierblock") < position(t, "vorschau-weiche")
        && position(t, "vorschau-weiche") < position(t, "approval")
        && position(t, "vorschau-weiche") < position(t, "send")
}

/// `erstkontakt-lesen` before the AI-note decision and the draft; `erstkontakt-vermerken` only after BOTH ways to send.
fn first_contact_memory(t: &str) -> bool {
    position(t, "erstkontakt-lesen") < position(t, "ki-haltung-weiche")
        && position(t, "ki-haltung-weiche") < position(t, "ki-hinweis")
        && position(t, "ki-hinweis") < position(t, "draft")
        && position(t, "send") < position(t, "erstkontakt-vermerken")
        && position(t, "hand-text") < position(t, "erstkontakt-vermerken")
        && step_body(t, "erstkontakt-lesen").contains("actor: machine")
        && step_body(t, "erstkontakt-vermerken").contains("actor: machine")
}

/// `ki-haltung-weiche`: unread rules or an unknown value stop; a target that refuses AI does NOT stop here (the draft is prepared).
fn ai_stance_gate(t: &str) -> bool {
    let b = step_body(t, "ki-haltung-weiche");
    has_all(
        &b,
        &[
            "$context.ziel.fundstellen == null",
            "($context.ziel.fundstellen | length) == 0",
            "then: ki-abgelehnt",
        ],
    ) && !b.contains("nutzerwort_ki")
        && !b.contains("$context.ziel.ki == \"abgelehnt\"")
}

/// `send` is blocked by `if:` for Kanidm, targets that refuse AI and a human sender without the YES; `hand-text` takes over; the approval
/// is skipped only for the bot at Blinc, and a human at Blinc keeps approval and the channel choice.
fn send_only_by_a_human_where_refused(t: &str) -> bool {
    has_all(
        &step_body(t, "send"),
        &[
            "if: '${ $context.channel != \"direct\"",
            ".input.upstream != \"kanidm\"",
            "$context.ziel.ki != \"abgelehnt\"",
            ".input.absender == \"bot\" or $context.freigabe.ja == true",
        ],
    ) && has_all(
        &step_body(t, "hand-text"),
        &[
            ".input.upstream == \"kanidm\"",
            "$context.ziel.ki == \"abgelehnt\"",
        ],
    ) && has_all(
        &step_body(t, "approval"),
        &[
            "(.input.upstream == \"blinc\" and .input.absender == \"bot\") | not",
            "$context + { freigabe: . }",
        ],
    ) && step_body(t, "choose-channel")
        .contains("(.input.upstream == \"blinc\" and .input.absender == \"bot\") | not")
        && step_body(t, "blinc-kanal")
            .contains(".input.upstream == \"blinc\" and .input.absender == \"bot\"")
}

/// Blinc has a line in the first-contact list, otherwise the bot path of rule 98 would always be refused.
fn blinc_in_the_list() -> bool {
    std::fs::read_to_string(support::workflows::root().join("workflows/upstream-erstkontakt.txt"))
        .unwrap()
        .lines()
        .any(|l| l.starts_with("project-blinc/Blinc ") && l.contains("erstkontakt="))
}

#[test]
fn upstream_anfrage_new_gates_hold() {
    let t = read("upstream-anfrage");
    assert!(sender_gate(&t));
    assert!(preview_gate(&t));
    assert!(first_contact_memory(&t));
    assert!(ai_stance_gate(&t));
    assert!(send_only_by_a_human_where_refused(&t));
    assert!(blinc_in_the_list());
}

fn mutant(t: &str, from: &str, to: &str) -> String {
    assert!(t.contains(from), "{from}");
    let m = t.replacen(from, to, 1);
    assert_ne!(m, t);
    m
}

fn swap(t: &str, a: &str, b: &str) -> String {
    let m = t
        .replacen(&format!("  - {a}:"), "  - tmp:", 1)
        .replacen(&format!("  - {b}:"), &format!("  - {a}:"), 1)
        .replacen("  - tmp:", &format!("  - {b}:"), 1);
    assert_ne!(m, t);
    m
}

#[test]
fn counter_upstream_anfrage_sender_gate_mutants_are_caught() {
    let t = read("upstream-anfrage");
    assert!(!sender_gate(&mutant(
        &t,
        "$context.erstkontakt.vorhanden != true",
        "false"
    )));
    assert!(!sender_gate(&mutant(
        &t,
        ".input.upstream == \"kanidm\" or ",
        ""
    )));
    assert!(!sender_gate(&mutant(
        &t,
        "$context.ziel.bot != \"erlaubt\"",
        "false"
    )));
    assert!(!sender_gate(&mutant(
        &t,
        "$context.ziel.ki == \"abgelehnt\" or $context.erstkontakt",
        "$context.erstkontakt"
    )));
    assert!(!sender_gate(&swap(&t, "absender-weiche", "draft")));
    assert!(!sender_gate(&swap(
        &t,
        "erstkontakt-lesen",
        "absender-weiche"
    )));
}

#[test]
fn counter_upstream_anfrage_preview_gate_mutants_are_caught() {
    let t = read("upstream-anfrage");
    assert!(!preview_gate(&mutant(
        &t,
        "$context.block.vorschau_ok != true",
        "false"
    )));
    assert!(!preview_gate(&swap(&t, "vorschau-weiche", "approval")));
    assert!(!preview_gate(&swap(&t, "vorschau-weiche", "kopierblock")));
}

#[test]
fn counter_upstream_anfrage_first_contact_memory_mutants_are_caught() {
    let t = read("upstream-anfrage");
    assert!(!first_contact_memory(&swap(
        &t,
        "erstkontakt-vermerken",
        "send"
    )));
    assert!(!first_contact_memory(&swap(
        &t,
        "erstkontakt-vermerken",
        "hand-text"
    )));
    assert!(!first_contact_memory(&swap(
        &t,
        "erstkontakt-lesen",
        "draft"
    )));
    assert!(!first_contact_memory(&swap(&t, "ki-hinweis", "draft")));
    let ai = mutant(
        &step_body(&t, "erstkontakt-lesen"),
        "freesynergy.net:actor: machine",
        "freesynergy.net:actor: ai",
    );
    assert!(!first_contact_memory(&t.replacen(
        &step_body(&t, "erstkontakt-lesen"),
        &ai,
        1
    )));
}

#[test]
fn counter_upstream_anfrage_ai_stance_gate_mutants_are_caught() {
    let t = read("upstream-anfrage");
    assert!(!ai_stance_gate(&mutant(
        &t,
        "$context.ziel.fundstellen == null or ",
        ""
    )));
    assert!(!ai_stance_gate(&mutant(
        &t,
        "then: ki-abgelehnt",
        "then: continue"
    )));
    // a refusing target stopped here again (the old behaviour) is red
    assert!(!ai_stance_gate(&mutant(
        &t,
        ") == null }'\n            then: ki-abgelehnt",
        ") == null or $context.ziel.ki == \"abgelehnt\" }'\n            then: ki-abgelehnt"
    )));
    assert!(!ai_stance_gate(&swap(
        &t,
        "ki-haltung-weiche",
        "erstkontakt-lesen"
    )));
}

#[test]
fn counter_upstream_anfrage_send_is_blocked_for_refusing_targets_and_unread_human_texts() {
    let t = read("upstream-anfrage");
    assert!(!send_only_by_a_human_where_refused(&mutant(
        &t,
        " and .input.upstream != \"kanidm\" and $context.ziel.ki != \"abgelehnt\"",
        ""
    )));
    assert!(!send_only_by_a_human_where_refused(&mutant(
        &t,
        " and $context.ziel.ki != \"abgelehnt\"",
        ""
    )));
    assert!(!send_only_by_a_human_where_refused(&mutant(
        &t,
        "(.input.absender == \"bot\" or $context.freigabe.ja == true)",
        "true"
    )));
    assert!(!send_only_by_a_human_where_refused(&mutant(&t, " or .input.upstream == \"kanidm\" or $context.ziel.ki == \"abgelehnt\" or ($context.freigabe", " or ($context.freigabe")));
    // Blinc with a human sender may not skip the approval (the old `upstream != blinc`)
    assert!(!send_only_by_a_human_where_refused(&t.replace(
        "(.input.upstream == \"blinc\" and .input.absender == \"bot\") | not",
        ".input.upstream != \"blinc\""
    )));
    assert!(!send_only_by_a_human_where_refused(&mutant(
        &t,
        "as: '${ $context + { freigabe: . } }'",
        "as: '${ $context }'"
    )));
    // Blinc without a line in the list refuses the bot path
    assert!(!blinc_in_the_list_in(
        &std::fs::read_to_string(
            support::workflows::root().join("workflows/upstream-erstkontakt.txt")
        )
        .unwrap()
        .replace("project-blinc/Blinc ", "project-x/X ")
    ));
}

fn blinc_in_the_list_in(text: &str) -> bool {
    text.lines()
        .any(|l| l.starts_with("project-blinc/Blinc ") && l.contains("erstkontakt="))
}
