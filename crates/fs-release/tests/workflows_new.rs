//! Pattern: none (test). The flows `frontend-umbau`, `upstream-anfrage`, `wrapper-einbindung`: who does which step, the order of
//! the steps, and that every sub-workflow a flow runs exists at the crate version. Each rule has a counter-test on a changed copy.
//! The 0.0.5 rules (one review, data flow, control flow, channel, license) are in `workflows_pflege5.rs`.

mod support;
use support::workflows::{read, step_body, step_names, FILES};

fn position(text: &str, step: &str) -> usize {
    step_names(text)
        .iter()
        .position(|n| n == step)
        .unwrap_or_else(|| panic!("no step {step}"))
}

/// The person approves (`actor: human`, `gate: human`) before the script sends; the rules check comes before the approval.
fn approval_before_send(text: &str) -> bool {
    let approval = step_body(text, "approval");
    approval.contains("actor: human")
        && approval.contains("gate: human")
        && position(text, "rules") < position(text, "approval")
        && position(text, "approval") < position(text, "send")
}

/// The counts, the image renders and the gates are scripts (`actor: machine`), never the agent's word;
/// `same-pictures` is a picture comparison with a verdict (picture-acceptance, a person decides).
fn counts_are_scripts(text: &str) -> bool {
    ["count", "recount", "before", "after", "gates"]
        .iter()
        .all(|s| step_body(text, s).contains("actor: machine"))
        && step_body(text, "same-pictures").contains("name: picture-acceptance")
}

/// The license step is in the flow before the gates and names the GitHub CI (cargo-deny runs there, not locally).
fn license_gate_in_ci(text: &str) -> bool {
    position(text, "pin") < position(text, "wrapper")
        && position(text, "wrapper") < position(text, "license")
        && position(text, "license") < position(text, "gates")
        && step_body(text, "license").contains("GitHub CI")
}

#[test]
fn upstream_anfrage_sends_only_after_the_rules_and_the_person_approved() {
    let text = read("upstream-anfrage");
    assert!(approval_before_send(&text));
    assert!(step_body(&text, "send").contains("actor: machine"));
    assert!(step_body(&text, "rules").contains("CLAUDE.md"));
}

#[test]
fn counter_upstream_anfrage_an_agent_approval_or_a_swapped_order_is_caught() {
    let text = read("upstream-anfrage");
    let ai = text.replacen(
        "      export:\n        as: '${ $context + { freigabe: . } }'\n      metadata:\n        freesynergy.net:actor: human",
        "      export:\n        as: '${ $context + { freigabe: . } }'\n      metadata:\n        freesynergy.net:actor: ai",
        1,
    );
    assert_ne!(ai, text);
    assert!(!approval_before_send(&ai));
    let swapped = text
        .replacen("  - send:", "  - tmp:", 1)
        .replacen("  - approval:", "  - send:", 1)
        .replacen("  - tmp:", "  - approval:", 1);
    assert!(!approval_before_send(&swapped));
}

#[test]
fn frontend_umbau_counts_and_compares_by_script() {
    let text = read("frontend-umbau");
    assert!(counts_are_scripts(&text));
    let release = step_body(&text, "release");
    assert!(release.contains("actor: human") && release.contains("gate: human"));
    assert!(position(&text, "recount") > position(&text, "convert"));
    assert!(position(&text, "same-pictures") > position(&text, "convert"));
}

#[test]
fn counter_frontend_umbau_a_count_by_the_agent_is_caught() {
    let text = read("frontend-umbau");
    let changed = text.replacen(
        "  - recount:\n      call: count-fixed-values:0.0.17@fs\n      with:\n        group: ${ .input.group }\n      export:\n        as: '${ $context + { recount: . } }'\n      metadata:\n        freesynergy.net:actor: machine\n        freesynergy.net:actor-type: Service",
        "  - recount:\n      call: count-fixed-values:0.0.17@fs\n      with:\n        group: ${ .input.group }\n      export:\n        as: '${ $context + { recount: . } }'\n      metadata:\n        freesynergy.net:actor: ai\n        freesynergy.net:actor-type: Application",
        1,
    );
    assert_ne!(changed, text);
    assert!(!counts_are_scripts(&changed));
}

#[test]
fn wrapper_einbindung_orders_pin_wrapper_license_gates_and_names_the_ci() {
    let text = read("wrapper-einbindung");
    assert!(license_gate_in_ci(&text));
    assert!(step_body(&text, "lock").contains("Cargo.lock"));
}

#[test]
fn counter_wrapper_einbindung_a_local_deny_run_or_a_late_license_step_is_caught() {
    let text = read("wrapper-einbindung");
    let local = text.replace("GitHub CI", "local run");
    assert_ne!(local, text);
    assert!(!license_gate_in_ci(&local));
    let late = text
        .replacen("  - license:", "  - tmp:", 1)
        .replacen("  - gates:", "  - license:", 1)
        .replacen("  - tmp:", "  - gates:", 1);
    assert!(!license_gate_in_ci(&late));
}

/// Every `name:` under a `workflow:` key of a file, with its version line.
fn sub_workflows(text: &str) -> Vec<(String, String)> {
    let lines: Vec<&str> = text.lines().map(str::trim).collect();
    let mut out = vec![];
    for (i, l) in lines.iter().enumerate() {
        if *l == "workflow:" {
            let name = lines[i + 2].strip_prefix("name: ").unwrap();
            let version = lines[i + 3].strip_prefix("version: ").unwrap();
            out.push((name.to_string(), version.to_string()));
        }
    }
    out
}

fn all_sub_workflows_exist(read_file: impl Fn(&str) -> String) -> bool {
    let version = env!("CARGO_PKG_VERSION");
    FILES.iter().all(|f| {
        sub_workflows(&read_file(f))
            .iter()
            .all(|(n, v)| v == version && FILES.contains(&n.as_str()))
    })
}

#[test]
fn every_sub_workflow_exists_at_the_crate_version() {
    assert!(all_sub_workflows_exist(read));
    let seen: usize = FILES.iter().map(|f| sub_workflows(&read(f)).len()).sum();
    assert!(seen >= 8, "found only {seen} sub-workflow references");
}

#[test]
fn counter_a_missing_sub_workflow_is_caught() {
    let broken = |f: &str| read(f).replace("name: review\n", "name: no-such-flow\n");
    assert!(!all_sub_workflows_exist(broken));
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
