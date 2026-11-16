//! Pattern: none (test). The flow `umbenennen` (dry run without AI, approval before the GitHub rename, name lock): order, human steps, approval bound to the
//! checked head and to the pair old -> new. Each rule has a counter-test on a changed copy.

mod support;
use support::workflows::{in_order, read, step_body};

fn rename_is_sound(text: &str) -> bool {
    let order = [
        "trockenlauf",
        "plan-red",
        "plan-gate",
        "umbau",
        "sperre",
        "doku",
        "nachher",
        "gates",
        "head",
        "review",
        "review-again",
        "freigabe",
        "merge",
        "github-umbenennen",
        "doku-veroeffentlichen",
        "weiterleitung",
        "release",
    ];
    let github = step_body(text, "github-umbenennen");
    in_order(text, &order)
        && step_body(text, "trockenlauf").contains("actor: machine")
        && step_body(text, "trockenlauf").contains("scripts/umbenennen dry-run")
        && step_body(text, "nachher").contains("scripts/umbenennen after")
        && step_body(text, "sperre").contains("retired-names.txt")
        && step_body(text, "doku").contains("name: doku-mitziehen")
        && step_body(text, "freigabe").contains("gate: strategie-ok")
        && step_body(text, "freigabe").contains("net.freesynergy.approval")
        && github.contains("actor: human")
        && github.contains("gate: strategie-ok")
        && step_body(text, "review").contains("gate: opus-ja")
        && step_body(text, "nachher-again").contains("then: eskalation")
        // B4: the approvals are bound to the checked head and to the pair old -> new
        && step_body(text, "head").contains("resolve-head")
        && step_body(text, "review").contains("head: ${ $context.kopf.head }")
        && !text.contains(".input.head")
        && step_body(text, "freigabe").contains("subject: ${ $context.kopf.head }")
        && step_body(text, "plan-gate").contains("subject: ${ .input.old + \"->\" + .input.new }")
        // B5: the red rounds of `nachher` are counted by the script
        && step_body(text, "nachher-again").contains(">= 3")
        // B6: a NO of the review goes back, from the third NO the person decides
        && step_body(text, "review-again").contains("then: umbau")
        && step_body(text, "review-again").contains("then: eskalation")
        && step_body(text, "review-again").contains(">= 3")
        && step_body(text, "plan-red").contains("$context.plan.red != false")
        && step_body(text, "merge").contains("actor: human")
        && step_body(text, "release").contains("old: ${ [.input.old] }")
}

#[test]
fn umbenennen_runs_the_dry_run_first_and_renames_only_after_the_approval() {
    assert!(rename_is_sound(&read("umbenennen")));
}

#[test]
fn counter_a_rename_without_the_gate_or_by_an_agent_is_caught() {
    let t = read("umbenennen");
    assert!(!rename_is_sound(&t.replacen(
        "gate: strategie-ok",
        "gate: none",
        2
    )));
    let github = step_body(&t, "github-umbenennen");
    let by_ai = github.replace("actor: human", "actor: ai");
    assert!(!rename_is_sound(&t.replacen(&github, &by_ai, 1)));
    assert!(!rename_is_sound(
        &t.replace("scripts/umbenennen dry-run", "an agent looks")
    ));
    assert!(!rename_is_sound(&t.replace("retired-names.txt", "nothing")));
    let swapped = t
        .replacen("  - freigabe:", "  - zz:", 1)
        .replacen("  - github-umbenennen:", "  - freigabe:", 1)
        .replacen("  - zz:", "  - github-umbenennen:", 1);
    assert!(!rename_is_sound(&swapped));
}

#[test]
fn counter_a_rename_approval_of_another_head_or_a_stale_head_or_a_missing_no_is_caught() {
    let t = read("umbenennen");
    assert!(rename_is_sound(&t));
    // B4: the approval without the subject filter, the review on a stale input head
    assert!(!rename_is_sound(
        &t.replace("subject: ${ $context.kopf.head }", "")
    ));
    assert!(!rename_is_sound(
        &t.replace("subject: ${ .input.old + \"->\" + .input.new }", "")
    ));
    assert!(!rename_is_sound(&t.replace(
        "head: ${ $context.kopf.head }",
        "head: ${ .input.head }"
    )));
    // B6: a NO that does not go back, a NO without the escalation
    assert!(!rename_is_sound(&t.replace(
        "then: umbau\n        - ja:",
        "then: continue\n        - ja:"
    )));
    assert!(!rename_is_sound(&t.replacen(
        ">= 3 }'\n            then: eskalation\n        - nein:",
        ">= 3 }'\n            then: continue\n        - nein:",
        1
    )));
    // plan-red: a run without result (rc 2) must not pass
    assert!(!rename_is_sound(
        &t.replace("$context.plan.red != false", "$context.plan.red")
    ));
    // order: the merge after the rename is caught
    let swapped = t
        .replacen("  - merge:", "  - zz:", 1)
        .replacen("  - github-umbenennen:", "  - merge:", 1)
        .replacen("  - zz:", "  - github-umbenennen:", 1);
    assert!(!rename_is_sound(&swapped));
}

/// Lessons L1, L2, L5 of the first rename (review-umbau-activity): the lock check after the write mode, the keep list
/// in the dry run and the after-check, the docs only after the rename. Each rule has a counter-test.
fn lessons_are_wired(text: &str) -> bool {
    in_order(text, &["umbau", "lock", "lock-weiter", "sperre"])
        && step_body(text, "lock").contains("cargo metadata --locked")
        && step_body(text, "lock-weiter").contains("then: umbau")
        && step_body(text, "lock-weiter").contains("then: continue")
        && step_body(text, "trockenlauf").contains("keep: ${ .input.keep }")
        && step_body(text, "nachher").contains("keep: ${ .input.keep }")
        && in_order(text, &["github-umbenennen", "doku-veroeffentlichen"])
        && step_body(text, "doku-veroeffentlichen").contains("NACH der GitHub-Umbenennung")
        // strategy order: code merge, GitHub rename, docs merge, remotes, catalog (own last step), then the release
        && in_order(
            text,
            &["merge", "github-umbenennen", "doku-veroeffentlichen", "remotes", "pins", "weiterleitung", "katalog", "release"],
        )
        && step_body(text, "pins").contains("rename --pins")
        && step_body(text, "pins").contains("AFTER the GitHub rename")
        && step_body(text, "umbau").contains("leaves pin URLs")
        && step_body(text, "katalog").contains("fs-catalog")
        && step_body(text, "katalog").contains("replaces")
        && !step_body(text, "merge").contains("of fs-catalog")
        // the name lock is written before `nachher`, in effect at the end
        && in_order(text, &["sperre", "nachher"])
        && step_body(text, "sperre").contains("geschrieben vor nachher, wirksam am Schluss")
        // lock loop: round counter and escalation like the other flows
        && step_body(text, "lock").contains("`runde`")
        && step_body(text, "lock-weiter").contains("lock.runde")
        && step_body(text, "lock-weiter").contains(">= 3")
        && step_body(text, "lock-weiter").contains("then: eskalation")
        // keep example is valid JSON (double quotes, escaped in the YAML string), not single quotes
        && step_body(text, "trockenlauf").contains("[{\\\"pattern\\\": \\\"history.*\\\", \\\"grund\\\"")
        && !step_body(text, "trockenlauf").contains("{'pattern'")
        // parts of the script that do not exist yet are marked as specification
        && step_body(text, "lock").contains("(Spezifikation, Skript folgt")
        && step_body(text, "umbau").contains("(Spezifikation, Skript folgt")
}

#[test]
fn umbenennen_lessons_l1_l2_l5_are_wired() {
    assert!(lessons_are_wired(&read("umbenennen")));
}

#[test]
fn counter_a_lock_check_that_does_not_go_back_or_a_docs_step_before_the_rename_is_caught() {
    let t = read("umbenennen");
    assert!(!lessons_are_wired(&t.replace(
        "then: umbau\n        - gruen",
        "then: continue\n        - gruen"
    )));
    assert!(!lessons_are_wired(
        &t.replace("cargo metadata --locked", "cargo check")
    ));
    assert!(!lessons_are_wired(&t.replace(
        "NACH der GitHub-Umbenennung",
        "vor der GitHub-Umbenennung"
    )));
    assert!(!lessons_are_wired(&t.replace("keep: ${ .input.keep }", "")));
    let swapped = t
        .replacen("  - doku-veroeffentlichen:", "  - zz:", 1)
        .replacen("  - github-umbenennen:", "  - doku-veroeffentlichen:", 1)
        .replacen("  - zz:", "  - github-umbenennen:", 1);
    assert!(!lessons_are_wired(&swapped));
}

#[test]
fn counter_order_rounds_json_and_spec_marks_of_the_corrected_lessons_are_caught() {
    let t = read("umbenennen");
    // the catalog before the GitHub rename (back in `merge`), the catalog step missing, the catalog before the remotes
    assert!(!lessons_are_wired(
        &t.replace("the own last step `katalog`)", "of fs-catalog)")
            .replace("(NOT fs-catalog, that is", "(of fs-catalog, that is")
    ));
    let swapped = t
        .replacen("  - katalog:", "  - zz:", 1)
        .replacen("  - remotes:", "  - katalog:", 1)
        .replacen("  - zz:", "  - remotes:", 1);
    assert!(!lessons_are_wired(&swapped));
    // the lock loop without the round counter or the escalation
    assert!(!lessons_are_wired(&t.replace("lock.runde", "lock.rounds")));
    let no_esc = step_body(&t, "lock-weiter").replace("then: eskalation", "then: umbau");
    assert!(!lessons_are_wired(&t.replacen(
        &step_body(&t, "lock-weiter"),
        &no_esc,
        1
    )));
    // the keep example with single quotes (no valid JSON), the lock step without the spec mark, the name lock text
    assert!(!lessons_are_wired(
        &t.replace("\\\"pattern\\\"", "'pattern'")
    ));
    assert!(!lessons_are_wired(&t.replace(
        "(Spezifikation, Skript folgt - the script step",
        "(the script step"
    )));
    assert!(!lessons_are_wired(
        &t.replace("wirksam am Schluss", "sofort")
    ));
}
