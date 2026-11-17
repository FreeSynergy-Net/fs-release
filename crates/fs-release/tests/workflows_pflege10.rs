//! Pattern: none (test). Workflow maintenance run 5: the fixed step `upstream-watch` (rule: always answer at the upstream). The sub-workflow
//! `upstream-watch` reads (GET only) the answers to our open Blinc pull requests and issues, makes a card and a short draft for every new
//! comment of another person and sends NOTHING itself (the helper of b7 sends after the YES of the user). The step stands in `koordinator-runde`
//! and `bahnen` with the repeat condition "older than 2 hours". No `gh`/network = YELLOW, never silently green. Counter-tests on the real files.

mod support;
use support::workflows::{in_order as order_of, read, step_body, step_names};

/// Steps in this order (a missing step is no order: false, never a panic).
fn in_order(text: &str, steps: &[&str]) -> bool {
    steps.iter().all(|s| has_step(text, s)) && order_of(text, steps)
}

fn has_step(text: &str, step: &str) -> bool {
    step_names(text).iter().any(|n| n == step)
}

fn has(text: &str, step: &str, phrases: &[&str]) -> bool {
    has_step(text, step) && {
        let body = step_body(text, step);
        phrases.iter().all(|p| body.contains(p))
    }
}

fn actor(text: &str, step: &str) -> String {
    step_body(text, step)
        .lines()
        .find_map(|l| l.trim().strip_prefix("freesynergy.net:actor: "))
        .unwrap_or("")
        .to_string()
}

fn ends(text: &str, step: &str) -> bool {
    has_step(text, step) && step_body(text, step).contains("\n      then: end\n")
}

/// Write words that must not be in the watch text (the watch reads with GET only).
const WRITE_WORDS: [&str; 14] = [
    "POST",
    "PATCH",
    "DELETE",
    "PUT ",
    "--method",
    "-X ",
    "--input",
    "gh issue comment",
    "gh pr comment",
    "gh pr review",
    "gh issue create",
    "gh pr create",
    "gh pr merge",
    "-f body",
];

fn read_only(text: &str) -> bool {
    WRITE_WORDS.iter().all(|w| !text.contains(w))
}

/// The calls of the sub-workflow are a FIXED list (a new call that writes at the upstream is red until the list is changed on purpose).
const CALLS: [&str; 11] = [
    "read-upstream-list",
    "upstream-reachable",
    "fetch-upstream",
    "filter-new-comments",
    "add-cards",
    "draft-answer-ai",
    "text-gate",
    "report-drafts",
    "approve",
    "hand-to-helper",
    "remember-check",
];

fn fixed_calls(t: &str) -> bool {
    let calls: Vec<&str> = t
        .lines()
        .filter_map(|l| l.trim().strip_prefix("call: "))
        .collect();
    calls.len() == CALLS.len()
        && calls.iter().all(|c| {
            c.strip_suffix(":0.0.10@fs")
                .is_some_and(|name| CALLS.contains(&name))
        })
}

fn watch(t: &str) -> bool {
    in_order(
        t,
        &[
            "liste-lesen",
            "erreichbar-pruefen",
            "erreichbar-weiche",
            "abrufen",
            "abruf-weiche",
            "neu-filtern",
            "neu-weiche",
            "karten-anlegen",
            "entwurf",
            "entwurf-pruefen",
            "entwurf-weiche",
            "melden",
            "freigabe-nutzer",
            "freigabe-weiche",
            "senden",
            "merken",
            "gelb-melden",
            "eskalation",
        ],
    ) && has(t, "liste-lesen", &["Reported upstream", "No number is invented"])
        && has(t, "erreichbar-pruefen", &["GET", "HTTP 403"])
        && has(
            t,
            "erreichbar-weiche",
            &["erreichbar.ok != true", "then: gelb-melden"],
        )
        && has(
            t,
            "abrufen",
            &["GET only", "issues/<n>/comments", "pulls/<n>/comments", "pulls/<n>/reviews", "mergeable", "No write call"],
        )
        && has(
            t,
            "abruf-weiche",
            &["$context.abruf == null", "then: gelb-melden"],
        )
        && has(
            t,
            "neu-filtern",
            &[
                "ANOTHER person",
                "without upper/lower case",
                "`created_at` is later than `letzte_pruefung` OR its id is not in the remembered list",
            ],
        )
        && has(
            t,
            "neu-weiche",
            &["$context.neu == null", "then: gelb-melden", "then: merken"],
        )
        && has(
            t,
            "karten-anlegen",
            &[
                "timeout 15 fs-tasks list",
                "no double",
                "--project bahn5-crates --as upstream-watch",
                "Blinc: Antwort auf #<n>",
            ],
        )
        && has(
            t,
            "entwurf",
            &["at most 5 sentences", "ANTWORT-<n>-<datum>.md", "sends nothing"],
        )
        && has(t, "entwurf-pruefen", &["NO AI words", "at most 5 sentences"])
        && has(
            t,
            "entwurf-weiche",
            &["- runden:", ">= 3", "then: eskalation", "then: entwurf"],
        )
        && has(t, "melden", &["b7", "Nothing is sent"])
        && actor(t, "freigabe-nutzer") == "human"
        && step_body(t, "freigabe-nutzer").contains("gate: human")
        && has(
            t,
            "freigabe-weiche",
            &["$context.freigabe.ja == true", "then: senden", "then: merken"],
        )
        && actor(t, "senden") == "human"
        && has(
            t,
            "senden",
            &["INSTRUCTION to the helper", "sends nothing itself"],
        )
        && has(
            t,
            "gelb-melden",
            &["status: yellow", "Upstream nicht erreichbar", "NEVER silently green"],
        )
        && ends(t, "gelb-melden")
        && ends(t, "merken")
        && ends(t, "eskalation")
        && read_only(t)
        && fixed_calls(t)
}

fn due_step(t: &str) -> bool {
    in_order(t, &["upstream-start", "upstream-faellig", "upstream-watch"])
        && has(
            t,
            "upstream-faellig",
            &[
                "upstream.letzte_pruefung",
                "> 7200",
                "== null",
                "then: upstream-watch",
                "older than 2 hours",
            ],
        )
        && has(
            t,
            "upstream-watch",
            &[
                "name: upstream-watch",
                "YELLOW",
                "Upstream nicht erreichbar",
                "YES of the user",
                "{ upstream: . }",
            ],
        )
        && actor(t, "upstream-watch") == "machine"
        && step_body(t, "upstream-watch").contains("      run:\n        workflow:\n")
        && read_only(&step_body(t, "upstream-faellig"))
        && read_only(&step_body(t, "upstream-watch"))
}

#[test]
fn the_watch_reads_only_makes_cards_and_drafts_and_the_helper_sends_after_the_yes() {
    let t = read("upstream-watch");
    assert!(watch(&t));
    assert!(t.contains("[FEHLENDE-DATENQUELLE-MELDEN]"));
    // the user's approval stands before the sending, the sending is the last action before the remember step
    assert!(in_order(
        &t,
        &["freigabe-nutzer", "freigabe-weiche", "senden"]
    ));
}

#[test]
fn the_step_is_in_both_round_workflows_with_the_repeat_condition_and_in_the_report() {
    for f in ["koordinator-runde", "bahnen"] {
        let t = read(f);
        assert!(due_step(&t), "{f}");
        assert!(
            has_step(&t, "upstream-watch")
                && step_names(&t)
                    .iter()
                    .filter(|n| *n == "upstream-watch")
                    .count()
                    == 1,
            "{f}"
        );
    }
    let k = read("koordinator-runde");
    assert!(step_body(&k, "bericht").contains("upstream: ${ $context.upstream }"));
    assert!(in_order(
        &k,
        &[
            "rot-pruefen",
            "upstream-faellig",
            "upstream-watch",
            "zaehler-start"
        ]
    ));
    let b = read("bahnen");
    assert!(step_body(&b, "meldung").contains("upstream: ${ $context.upstream }"));
    assert!(in_order(
        &b,
        &[
            "slots-weiche",
            "upstream-faellig",
            "upstream-watch",
            "meldung"
        ]
    ));
}

#[test]
fn counter_watch_mutants_are_caught() {
    let t = read("upstream-watch");
    for step in [
        "erreichbar-weiche",
        "abruf-weiche",
        "neu-filtern",
        "karten-anlegen",
        "entwurf-pruefen",
        "freigabe-nutzer",
        "freigabe-weiche",
        "gelb-melden",
    ] {
        let bad = remove(&t, step);
        assert!(!watch(&bad), "{step}");
    }
    // `senden` before the approval of the user
    let (a, b) = (step_block(&t, "senden"), step_block(&t, "freigabe-nutzer"));
    let swapped = t.replacen(&a, "", 1).replacen(&b, &format!("{a}{b}"), 1);
    assert_ne!(swapped, t);
    assert!(!watch(&swapped));
    // the approval done by an agent
    let human = step_body(&t, "freigabe-nutzer");
    assert!(!watch(&t.replacen(
        &human,
        &human.replacen("actor: human", "actor: ai", 1),
        1
    )));
    // a write call anywhere (the whole text and the sending step)
    for w in [
        "gh api -X POST repos/x/issues/1/comments",
        "gh pr comment 73",
        "--method PATCH",
        "gh api -X DELETE x",
    ] {
        let bad = t.replacen(
            "sends nothing itself",
            &format!("sends nothing itself; {w}"),
            1,
        );
        assert!(!watch(&bad), "{w}");
    }
    // a new call that is not on the fixed list
    assert!(!watch(
        &t.replace("call: hand-to-helper:", "call: comment-issue:")
    ));
    assert!(!watch(
        &t.replace("call: fetch-upstream:", "call: post-comment:")
    ));
    // the yellow end: green instead of yellow, or no end, or a switch that no longer goes yellow
    assert!(!watch(&t.replace("status: yellow", "status: green")));
    assert!(!watch(&t.replace("Upstream nicht erreichbar", "ok")));
    assert!(!watch(&t.replace("then: gelb-melden", "then: continue")));
    assert!(!watch(
        &t.replace("$context.erreichbar.ok != true", "false")
    ));
    // the loop without the third round, the new-comment rule weakened, the board rule changed
    assert!(!watch(&t.replace(">= 3", ">= 9")));
    assert!(!watch(&t.replace("without upper/lower case", "exactly")));
    assert!(!watch(&t.replace(
        "OR its id is not in the remembered list",
        "AND nothing"
    )));
    assert!(!watch(
        &t.replace("--project bahn5-crates", "--project other")
    ));
    assert!(!watch(
        &t.replace("at most 5 sentences", "a few paragraphs")
    ));
    assert!(!watch(&t.replacen("      then: end\n", "", 1)));
}

#[test]
fn counter_due_step_mutants_are_caught() {
    for f in ["koordinator-runde", "bahnen"] {
        let t = read(f);
        assert!(due_step(&t), "{f}");
        assert!(!due_step(&remove(&t, "upstream-faellig")), "{f}");
        assert!(!due_step(&remove(&t, "upstream-watch")), "{f}");
        assert!(!due_step(&t.replace("> 7200", "> 3600")), "{f}");
        assert!(
            !due_step(&t.replace("older than 2 hours", "older than a day")),
            "{f}"
        );
        assert!(
            !due_step(&t.replace("== null or (now", "== 1 or (now")),
            "{f}"
        );
        assert!(
            !due_step(&t.replace("then: upstream-watch", "then: continue")),
            "{f}"
        );
        assert!(
            !due_step(&t.replace("Upstream nicht erreichbar", "all fine")),
            "{f}"
        );
        assert!(
            !due_step(&t.replace("name: upstream-watch", "name: other")),
            "{f}"
        );
        // a write call in the step text
        let bad = t.replacen(
            "never silently green; the result",
            "never silently green; gh api -X POST x; the result",
            1,
        );
        assert_ne!(bad, t, "{f}");
        assert!(!due_step(&bad), "{f}");
    }
}

#[test]
fn the_rule_of_the_issues_stays_in_upstream_anfrage_and_the_watch_names_it() {
    let t = read("upstream-watch");
    assert!(t.contains("no new issues while our own pull requests are open"));
    let u = read("upstream-anfrage");
    assert!(u.contains("NEVER a pull request and NEVER a comment"));
}

fn step_block(text: &str, step: &str) -> String {
    let head = format!("  - {step}:\n");
    let start = text.find(&head).unwrap();
    let rest = &text[start + head.len()..];
    let end = rest.find("\n  - ").map_or(rest.len(), |i| i + 1);
    format!("{head}{}", &rest[..end])
}

fn remove(text: &str, step: &str) -> String {
    text.replacen(&step_block(text, step), "", 1)
}
