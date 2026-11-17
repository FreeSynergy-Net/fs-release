//! Pattern: none (test). Workflow maintenance run 2: the new flows `stoerung-diagnose` (read-only diagnosis, no fix in the same run),
//! `branches-aufraeumen` (list and user script, the agents delete nothing), `menschen-aufnahme` (consent only by the person) and the
//! extended `upstream-anfrage` (proof, duplicates, English text without AI words, git-bug id, no pull request, no comment).
//! Every rule has a counter-test on the real file (step struck, in the wrong place, actor changed or the key phrase changed: red).

mod support;
use support::workflows::{assert_red, in_order, read, step_body, step_names};

fn has_step(text: &str, step: &str) -> bool {
    step_names(text).iter().any(|n| n == step)
}

/// Steps in this order (a missing step is no order: false, never a panic).
fn order(text: &str, steps: &[&str]) -> bool {
    steps.iter().all(|s| has_step(text, s)) && in_order(text, steps)
}

fn has(text: &str, step: &str, phrases: &[&str]) -> bool {
    has_step(text, step) && {
        let body = step_body(text, step);
        phrases.iter().all(|p| body.contains(p))
    }
}

/// The `  - name:` blocks of a file (the layout has the steps at level two); the lines before the first step are the head.
fn split(text: &str) -> (Vec<String>, Vec<(String, Vec<String>)>) {
    let mut head = Vec::new();
    let mut blocks: Vec<(String, Vec<String>)> = Vec::new();
    for l in text.lines() {
        if l.starts_with("  - ") && l.ends_with(':') {
            blocks.push((l.to_string(), vec![l.to_string()]));
        } else if let Some(b) = blocks.last_mut() {
            b.1.push(l.to_string());
        } else {
            head.push(l.to_string());
        }
    }
    (head, blocks)
}

fn join(head: Vec<String>, blocks: Vec<(String, Vec<String>)>) -> String {
    let mut out = head;
    out.extend(blocks.into_iter().flat_map(|b| b.1));
    out.join("\n") + "\n"
}

fn without(text: &str, step: &str) -> String {
    let (head, mut blocks) = split(text);
    let at = blocks.iter().position(|b| b.0 == format!("  - {step}:"));
    blocks.remove(at.unwrap());
    join(head, blocks)
}

fn move_after(text: &str, step: &str, after: &str) -> String {
    let (head, mut blocks) = split(text);
    let name = |s: &str| format!("  - {s}:");
    let i = blocks.iter().position(|b| b.0 == name(step)).unwrap();
    let moved = blocks.remove(i);
    let j = blocks.iter().position(|b| b.0 == name(after)).unwrap();
    blocks.insert(j + 1, moved);
    join(head, blocks)
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

/// The loops end in the escalation: the `switch` has a case `runden` with `>= 3` that goes to `eskalation`, and a case that goes back.
fn loop_ends(text: &str, switch: &str, back: &str) -> bool {
    has(
        text,
        switch,
        &[
            "- runden:",
            ">= 3",
            "then: eskalation",
            &format!("then: {back}"),
        ],
    )
}

// ---------------------------------------------------------------- stoerung-diagnose

fn diagnose(t: &str) -> bool {
    order(
        t,
        &[
            "symptom-aufnehmen",
            "ort-pruefen",
            "ort-weiche",
            "logs-lesen",
            "hypothesen",
            "reproduktion",
            "repro-weiche",
            "ursache-belegen",
            "beleg-weiche",
            "kein-fix",
            "kein-fix-weiche",
            "bericht",
            "uebergabe",
            "nutzeranleitung",
            "eskalation",
        ],
    ) && has(
        t,
        "ort-pruefen",
        &[
            "real server",
            "session of the user account",
            "own directory",
        ],
    ) && has(t, "ort-weiche", &["then: nutzeranleitung"])
        && has(t, "logs-lesen", &["only reads", "real server"])
        && has(t, "reproduktion", &["OWN directory", "never a real server"])
        && loop_ends(t, "repro-weiche", "hypothesen")
        && has(t, "ursache-belegen", &["PROVES the cause", "belegt: false"])
        && loop_ends(t, "beleg-weiche", "hypothesen")
        && has(t, "kein-fix", &["NO FIX", "product code"])
        && has(t, "kein-fix-weiche", &["then: eskalation", "bugfix"])
        && has(t, "bericht", &["cause", "proof", "PROPOSAL"])
        && has(
            t,
            "uebergabe",
            &["NEW run", "Workflow bugfix", "never fixes"],
        )
        && ends(t, "uebergabe")
        && ends(t, "nutzeranleitung")
        && ends(t, "eskalation")
        && actor(t, "kein-fix") == "machine"
        && actor(t, "uebergabe") == "human"
        && !t.contains("\n      run:\n")
        && !t.contains("release-flow")
        && !t.contains("name: bugfix")
}

#[test]
fn diagnosis_is_read_only_in_the_own_directory_and_never_fixes() {
    let t = read("stoerung-diagnose");
    assert!(diagnose(&t));
    assert!(t.contains("KEIN-ZUGRIFF-AUF-NUTZERSITZUNG") && t.contains("KEIN-AGENT-AUF-SERVERN"));
    assert!(t.contains("BEFUND-einzelinstanz") && t.contains("REFUTED"));
}

#[test]
fn counter_diagnosis_mutants_are_caught() {
    let t = read("stoerung-diagnose");
    assert!(!diagnose(&without(&t, "kein-fix")));
    assert!(!diagnose(&without(&t, "ort-pruefen")));
    assert!(!diagnose(&move_after(&t, "kein-fix", "bericht")));
    assert!(!diagnose(&move_after(
        &t,
        "reproduktion",
        "ursache-belegen"
    )));
    assert!(!diagnose(
        &t.replace("then: nutzeranleitung", "then: continue")
    ));
    assert!(!diagnose(
        &t.replace("never a real server", "a real server")
    ));
    assert!(!diagnose(&t.replace("NO FIX", "a small fix")));
    // a fall-through into the escalation: the hand-over no longer ends
    assert!(!diagnose(&t.replacen("      then: end\n", "", 1)));
    // the loop without the third round
    assert!(!diagnose(&t.replace(">= 3", ">= 9")));
    // a fix in the same run: a sub-workflow `bugfix` is called
    assert!(!diagnose(&t.replace(
        "  - uebergabe:\n",
        "  - uebergabe:\n      run:\n        workflow:\n          name: bugfix\n"
    )));
    assert_red("stoerung-diagnose", "no step of this flow", |x| {
        x.replacen("then: nutzeranleitung", "then: nirgends", 1)
    });
}

// ---------------------------------------------------------------- branches-aufraeumen

/// Words that mean a removal; an agent or machine step (not the two that describe the user script) must contain none of them.
const REMOVAL: [&str; 13] = [
    " rm ",
    "`rm",
    "rm -",
    "delete",
    "--delete",
    "prune",
    "branch -d",
    "rmi",
    "volume rm",
    "--apply",
    "unlink",
    "shred",
    "push",
];

fn no_removal_by_agents(t: &str) -> bool {
    step_names(t).iter().all(|s| {
        let a = actor(t, s);
        let body = step_body(t, s).to_lowercase();
        let user_script = s == "skript-erzeugen" || s == "skript-pruefen";
        a == "human" || user_script || REMOVAL.iter().all(|w| !body.contains(w))
    })
}

fn cleanup(t: &str) -> bool {
    order(
        t,
        &[
            "remote-lage",
            "merged-belegen",
            "lokal-listen",
            "podman-listen",
            "bleibt-liste",
            "listen-pruefen",
            "listen-weiche",
            "skript-erzeugen",
            "skript-pruefen",
            "skript-weiche",
            "nutzer-fuehrt-aus",
            "eskalation",
        ],
    ) && has(t, "remote-lage", &["ls-remote", "read only"])
        && has(
            t,
            "merged-belegen",
            &["--is-ancestor", "git cherry", "unklar"],
        )
        && has(t, "lokal-listen", &["review-*", "wt/*", "target"])
        && has(
            t,
            "podman-listen",
            &["Anonymous volumes", "BLEIBT", "never put on a list"],
        )
        && has(
            t,
            "bleibt-liste",
            &["`weg`", "`bleibt`", "when in doubt it goes to `bleibt`"],
        )
        && has(
            t,
            "listen-pruefen",
            &["ONE complete name", "no `*`", "missing proof is red"],
        )
        && loop_ends(t, "listen-weiche", "bleibt-liste")
        && has(
            t,
            "skript-erzeugen",
            &[
                "WRITES (does not run)",
                "fuer-nutzer.md",
                "`--dry-run`",
                "no placeholder, no glob",
                "absolute path",
                "TEXT inside this script",
                "NEVER run by the agent",
            ],
        )
        && has(
            t,
            "skript-pruefen",
            &[
                "shellcheck",
                "AFTER this static check is green",
                "ONLY with `--dry-run`",
                "NEVER",
                "SANDBOX",
                "temporary HOME",
                "no SSH agent",
                "GIT_TERMINAL_PROMPT=0",
                "no push remote",
                "does not run at all",
            ],
        )
        && loop_ends(t, "skript-weiche", "skript-erzeugen")
        && actor(t, "nutzer-fuehrt-aus") == "human"
        && step_body(t, "nutzer-fuehrt-aus").contains("gate: human")
        && step_body(t, "nutzer-fuehrt-aus").contains("agents delete nothing")
        && ends(t, "nutzer-fuehrt-aus")
        && ends(t, "eskalation")
        && no_removal_by_agents(t)
}

#[test]
fn cleanup_makes_a_list_and_a_user_script_and_the_agents_delete_nothing() {
    let t = read("branches-aufraeumen");
    assert!(cleanup(&t));
    assert!(no_removal_by_agents(&t));
    assert!(t.contains("DELETE NOTHING") && t.contains("never worked around"));
}

#[test]
fn counter_cleanup_mutants_are_caught() {
    let t = read("branches-aufraeumen");
    // a removal action of an agent or a machine anywhere in the flow
    for (step, add) in [
        (
            "bleibt-liste",
            " Then it runs git push --delete on every entry.",
        ),
        (
            "podman-listen",
            " Then it runs podman volume rm on the rest.",
        ),
        ("lokal-listen", " Then it removes the folders with rm -rf."),
        ("remote-lage", " Then it deletes the merged branches."),
        ("listen-pruefen", " Then it runs the script with --apply."),
    ] {
        let body = step_body(&t, step);
        let mut mutated = body.clone();
        mutated.insert_str(mutated.rfind('"').unwrap(), add);
        let bad = t.replacen(&body, &mutated, 1);
        assert_ne!(bad, t, "{step}");
        assert!(!no_removal_by_agents(&bad), "{step}");
        assert!(!cleanup(&bad), "{step}");
    }
    // the user step done by an agent
    let human = step_body(&t, "nutzer-fuehrt-aus");
    let ai = human.replacen("actor: human", "actor: ai", 1);
    assert!(!cleanup(&t.replacen(&human, &ai, 1)));
    assert!(!cleanup(&without(&t, "skript-pruefen")));
    assert!(!cleanup(&move_after(
        &t,
        "nutzer-fuehrt-aus",
        "skript-erzeugen"
    )));
    assert!(!cleanup(
        &t.replace("ONLY with `--dry-run`", "with the apply flag")
    ));
    assert!(!cleanup(
        &t.replace("no placeholder, no glob", "short names")
    ));
    assert!(!cleanup(&t.replacen("      then: end\n", "", 1)));
    assert_red("branches-aufraeumen", "no step of this flow", |x| {
        x.replacen("then: bleibt-liste", "then: nirgends", 1)
    });
}

// ---------------------------------------------------------------- menschen-aufnahme

/// Every consent, vote and release is a `human` step with `gate: human`; the agent steps only prepare and report.
fn consents_by_people(t: &str) -> bool {
    let people = [
        "einwilligung-daten",
        "rolle-klaeren",
        "abstimmung",
        "admin-freigabe",
        "datenschutz-hinweis",
        "einladung-senden",
    ];
    let ai_steps: Vec<String> = step_names(t)
        .into_iter()
        .filter(|s| actor(t, s) == "ai")
        .collect();
    people.iter().all(|s| {
        has_step(t, s) && actor(t, s) == "human" && step_body(t, s).contains("gate: human")
    }) && ai_steps == ["willkommen-vorbereiten", "aufnahme-beleg"]
}

fn onboarding(t: &str) -> bool {
    order(
        t,
        &[
            "angaben-aufnehmen",
            "einwilligung-daten",
            "einwilligung-weiche",
            "daten-pruefen",
            "angaben-weiche",
            "rolle-klaeren",
            "zugang-lesen",
            "zugang-weiche",
            "tuersteher",
            "abstimmung",
            "admin-freigabe",
            "eintritt-weiche",
            "willkommen-vorbereiten",
            "zustimmung-pruefen",
            "zustimmung-weiche",
            "datenschutz-hinweis",
            "hinweis-weiche",
            "einladung-senden",
            "aufnahme-beleg",
            "ohne-einwilligung",
            "ohne-zugang",
            "eskalation",
        ],
    ) && consents_by_people(t)
        && ["tuersteher", "abstimmung", "admin-freigabe"]
            .iter()
            .all(|s| has(t, s, &["{ eintritt: . }", "`frei`"]))
        && has(
            t,
            "eintritt-weiche",
            &[
                "$context.zugang.art != \"open\"",
                "$context.eintritt.frei != true",
                "then: ohne-zugang",
            ],
        )
        && actor(t, "ohne-zugang") == "human"
        && ends(t, "ohne-zugang")
        && has(
            t,
            "angaben-aufnehmen",
            &["themselves", "nothing is looked up"],
        )
        && has(
            t,
            "einwilligung-weiche",
            &["then: ohne-einwilligung", "gegeben != true"],
        )
        && has(
            t,
            "daten-pruefen",
            &["no labelling", "NOT dropped silently"],
        )
        && loop_ends(t, "angaben-weiche", "angaben-aufnehmen")
        && has(t, "tuersteher", &["no AI", "never an account of a person"])
        && has(
            t,
            "willkommen-vorbereiten",
            &[
                "[KEINE-ZUSTIMMUNG-IM-NAMEN-DES-NUTZERS]",
                "sends nothing",
                "STOP-AI",
            ],
        )
        && has(
            t,
            "zustimmung-pruefen",
            &["`human` as the actor", "written by an agent"],
        )
        && loop_ends(t, "zustimmung-weiche", "willkommen-vorbereiten")
        && has(t, "datenschutz-hinweis", &["themselves", "STOP-AI"])
        && has(
            t,
            "hinweis-weiche",
            &["then: ohne-einwilligung", "gesehen != true"],
        )
        && has(t, "einladung-senden", &["the agent sends nothing"])
        && ends(t, "aufnahme-beleg")
        && ends(t, "ohne-einwilligung")
        && ends(t, "eskalation")
}

#[test]
fn onboarding_takes_consent_only_from_the_person() {
    let t = read("menschen-aufnahme");
    assert!(onboarding(&t));
    assert!(t.contains("KEINE-ZUSTIMMUNG-IM-NAMEN-DES-NUTZERS") && t.contains("People first"));
}

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

fn request(t: &str) -> bool {
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
const CALLS: [&str; 14] = [
    "request-kind",
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

fn no_pr_calls(t: &str) -> bool {
    let calls: Vec<&str> = t
        .lines()
        .filter_map(|l| l.trim().strip_prefix("call: "))
        .collect();
    !calls.is_empty()
        && calls.iter().all(|c| {
            c.strip_suffix(":0.0.10@fs")
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

#[test]
fn counter_request_missing_results_are_red_and_calls_are_a_fixed_list() {
    let t = read("upstream-anfrage");
    assert!(request(&t));
    let sneaky = "  - pr:\n      call: open-change:0.0.10@fs\n      metadata:\n        freesynergy.net:summary: \"opens a pull request\"\n  - record:\n";
    assert!(!request(&t.replacen("  - record:\n", sneaky, 1)));
    assert!(!request(&t.replace("{ dubletten: . }", "{ x: . }")));
    assert!(!request(
        &t.replace("$context.dubletten.treffer == null or ", "")
    ));
    assert!(!request(&t.replace("$context.rules.hits == null or ", "")));
    assert!(!request(
        &t.replace("$context.textcheck.hits == null or ", "")
    ));
    assert!(!request(
        &t.replace("without a YES, by rule 98", "after the YES")
    ));
}
