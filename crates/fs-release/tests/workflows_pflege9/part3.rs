use super::*;

#[test]
fn counter_onboarding_mutants_are_caught() {
    let t = read("menschen-aufnahme");
    for step in [
        "einwilligung-daten",
        "datenschutz-hinweis",
        "abstimmung",
        "einladung-senden",
    ] {
        let body = step_body(&t, step);
        let ai = body.replacen("actor: human", "actor: ai", 1);
        assert_ne!(ai, body);
        assert!(!onboarding(&t.replacen(&body, &ai, 1)), "{step}");
    }
    assert!(!onboarding(&without(&t, "einwilligung-weiche")));
    assert!(!onboarding(&without(&t, "zustimmung-pruefen")));
    assert!(!onboarding(&move_after(
        &t,
        "datenschutz-hinweis",
        "einladung-senden"
    )));
    assert!(!onboarding(&move_after(
        &t,
        "einwilligung-daten",
        "daten-pruefen"
    )));
    assert!(!onboarding(&t.replace("NOT dropped silently", "dropped")));
    assert!(!onboarding(
        &t.replace("then: ohne-einwilligung", "then: continue")
    ));
    assert!(!onboarding(
        &t.replace("[KEINE-ZUSTIMMUNG-IM-NAMEN-DES-NUTZERS]", "[X]")
    ));
    assert!(!onboarding(&t.replace(">= 3", ">= 7")));
}

// ---------------------------------------------------------------- upstream-anfrage (extended)

pub(super) fn request(t: &str) -> bool {
    order(
        t,
        &[
            "art-pruefen",
            "art-weiche",
            "belegen",
            "belegt-weiche",
            "dubletten",
            "dubletten-weiche",
            "ticket",
            "draft",
            "rules",
            "text-gate",
            "back-to-draft",
            "blinc-kanal",
            "approval",
            "choose-channel",
            "send",
            "hand-text",
            "link-ins-ticket",
            "record",
            "abgelehnt",
            "eskalation",
        ],
    ) && has(
        t,
        "art-pruefen",
        &["only an `issue`", "pull request", "comment"],
    ) && has(t, "art-weiche", &["then: abgelehnt"])
        && has(t, "belegen", &["PROVES", "reproduction", "source"])
        && has(t, "belegt-weiche", &["then: abgelehnt", "belegt != true"])
        && has(t, "dubletten", &["READ ONLY", "no comment", "no new issue"])
        && has(
            t,
            "dubletten-weiche",
            &[
                "then: abgelehnt",
                "no comment is written",
                "$context.dubletten.treffer == null",
            ],
        )
        && has(t, "ticket", &["git-bug", "OUR repo", "`id`"])
        && has(
            t,
            "draft",
            &["$context.ticket.id", "English", "NO AI words"],
        )
        && has(
            t,
            "text-gate",
            &["ENGLISH", "NO AI words", "e-mail address", "git-bug id"],
        )
        && has(
            t,
            "back-to-draft",
            &[
                "textcheck.hits",
                "$context.rules.hits == null",
                "$context.textcheck.hits == null",
                "- runden:",
                ">= 3",
                "then: eskalation",
                "then: draft",
            ],
        )
        && has(
            t,
            "blinc-kanal",
            &[
                "(98)",
                "`git-bug`",
                "no pull request",
                "no comment",
                "upstream == \"blinc\"",
            ],
        )
        && has(t, "approval", &["actor: human", "upstream != \"blinc\""])
        && has(t, "choose-channel", &["upstream != \"blinc\""])
        && has(t, "send", &["helper of the strategy", "never calls gh"])
        && has(t, "send", &["without a YES, by rule 98"])
        && t.contains("parameters upstream, evidence, art`")
        && [
            "art",
            "belegt",
            "dubletten",
            "ticket",
            "textcheck",
            "channel",
        ]
        .iter()
        .all(|k| t.contains(&format!("{{ {k}: . }}")))
        && actor(t, "send") == "machine"
        && ends(t, "record")
        && ends(t, "abgelehnt")
        && ends(t, "eskalation")
        && no_pr_calls(t)
}

/// The calls of the flow are a FIXED list (a new call, e.g. one that opens a pull request or writes a comment, is red until the list is changed on purpose).
pub(super) const CALLS: [&str; 19] = [
    "request-kind",
    "target-rules",
    "first-contact-read",
    "ai-note-decide",
    "first-contact-note",
    "copy-block",
    "prove-ai",
    "search-duplicates-ai",
    "git-bug-ticket",
    "draft-issue",
    "upstream-rules",
    "text-gate",
    "blinc-channel",
    "approve",
    "choose-channel",
    "send-issue",
    "hand-text",
    "ticket-link",
    "note-history",
];

pub(super) fn no_pr_calls(t: &str) -> bool {
    let calls: Vec<&str> = t
        .lines()
        .filter_map(|l| l.trim().strip_prefix("call: "))
        .collect();
    !calls.is_empty()
        && calls.iter().all(|c| {
            c.strip_suffix(":0.0.17@fs")
                .is_some_and(|name| CALLS.contains(&name))
        })
}

#[test]
fn upstream_request_is_proven_checked_for_duplicates_and_without_pr_or_comment() {
    let t = read("upstream-anfrage");
    assert!(request(&t));
    assert!(t.contains("NEVER a pull request and NEVER a comment"));
}

#[test]
fn counter_upstream_request_mutants_are_caught() {
    let t = read("upstream-anfrage");
    assert!(!request(&without(&t, "dubletten")));
    assert!(!request(&without(&t, "belegen")));
    assert!(!request(&without(&t, "text-gate")));
    assert!(!request(&without(&t, "blinc-kanal")));
    assert!(!request(&without(&t, "ticket")));
    assert!(!request(&move_after(&t, "send", "art-pruefen")));
    assert!(!request(&move_after(&t, "text-gate", "approval")));
    assert!(!request(&t.replace("textcheck.hits", "textcheck.none")));
    assert!(!request(&t.replace("NO AI words", "short words")));
    assert!(!request(&t.replace("ENGLISH", "any language")));
    assert!(!request(&t.replacen(
        "then: abgelehnt",
        "then: continue",
        1
    )));
    assert!(!request(
        &t.replace("upstream != \"blinc\"", "upstream != \"x\"")
    ));
    // a pull request or a comment step
    assert!(!request(
        &t.replace("call: send-issue:", "call: comment-issue:")
    ));
    assert!(!request(&t.replace(
        "call: search-duplicates-ai:",
        "call: pull-request-ai:"
    )));
    assert!(!request(&t.replacen("      then: end\n", "", 1)));
    assert_red("upstream-anfrage", "no step of this flow", |x| {
        x.replacen("then: abgelehnt", "then: nirgends", 1)
    });
}

#[test]
fn every_new_flow_keeps_the_escalation_after_three_rounds() {
    for f in [
        "stoerung-diagnose",
        "branches-aufraeumen",
        "menschen-aufnahme",
        "upstream-anfrage",
    ] {
        let t = read(f);
        assert!(has_step(&t, "eskalation"), "{f}");
        assert!(t.contains(">= 3") && t.contains("runde"), "{f}");
        assert_eq!(
            step_names(&t).iter().filter(|n| *n == "eskalation").count(),
            1,
            "{f}"
        );
    }
}

#[test]
fn counter_cleanup_dry_run_only_after_the_static_check_in_a_sandbox() {
    let t = read("branches-aufraeumen");
    assert!(cleanup(&t));
    for (a, b) in [
        ("SANDBOX", "folder"),
        ("GIT_TERMINAL_PROMPT=0", "nothing"),
        ("AFTER this static check is green", "at once"),
        ("no SSH agent", "an SSH agent"),
        ("does not run at all", "runs anyway"),
    ] {
        assert!(!cleanup(&t.replace(a, b)), "{a}");
    }
}

#[test]
fn counter_onboarding_the_way_in_must_let_the_person_in() {
    let t = read("menschen-aufnahme");
    assert!(onboarding(&t));
    assert!(!onboarding(&without(&t, "eintritt-weiche")));
    assert!(!onboarding(&without(&t, "ohne-zugang")));
    assert!(!onboarding(&t.replacen("{ eintritt: . }", "{ x: . }", 1)));
    assert!(!onboarding(
        &t.replace("$context.eintritt.frei != true", "false")
    ));
    assert!(!onboarding(&move_after(
        &t,
        "eintritt-weiche",
        "einladung-senden"
    )));
    assert_red("menschen-aufnahme", "no step of this flow", |x| {
        x.replacen("then: ohne-zugang", "then: nirgends", 1)
    });
}
