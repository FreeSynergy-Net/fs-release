use super::*;

pub(super) fn has_step(text: &str, step: &str) -> bool {
    step_names(text).iter().any(|n| n == step)
}

pub(super) fn has(text: &str, step: &str, phrases: &[&str]) -> bool {
    has_step(text, step) && {
        let body = step_body(text, step);
        phrases.iter().all(|p| body.contains(p))
    }
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

/// Write words that must not be in the watch text (the watch reads with GET only).
pub(super) const WRITE_WORDS: [&str; 14] = [
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

pub(super) fn read_only(text: &str) -> bool {
    WRITE_WORDS.iter().all(|w| !text.contains(w))
}

/// The calls of the sub-workflow are a FIXED list (a new call that writes at the upstream is red until the list is changed on purpose).
pub(super) const CALLS: [&str; 11] = [
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

pub(super) fn fixed_calls(t: &str) -> bool {
    let calls: Vec<&str> = t
        .lines()
        .filter_map(|l| l.trim().strip_prefix("call: "))
        .collect();
    calls.len() == CALLS.len()
        && calls.iter().all(|c| {
            c.strip_suffix(":0.0.11@fs")
                .is_some_and(|name| CALLS.contains(&name))
        })
}

pub(super) fn watch(t: &str) -> bool {
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

pub(super) fn due_step(t: &str) -> bool {
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
                "{ upstream: ($context.upstream + .) }",
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
