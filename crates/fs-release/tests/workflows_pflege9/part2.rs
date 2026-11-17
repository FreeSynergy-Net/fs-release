use super::*;

pub(super) fn cleanup(t: &str) -> bool {
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
pub(super) fn consents_by_people(t: &str) -> bool {
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

pub(super) fn onboarding(t: &str) -> bool {
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
