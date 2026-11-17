use super::*;

/// The lanes round `bahnen`: fs-tasks only with `timeout 15`, never change the order of cards, hang reported, remote by `ls-remote`, max 2 builds.
pub fn lanes(text: &str) -> bool {
    order(
        text,
        &[
            "ids-lesen",
            "board-lesen",
            "haenger-weiche",
            "remote-lage",
            "karten-abgleichen",
            "abgleich-weiche",
            "slots-pruefen",
            "slots-verteilen",
            "slots-weiche",
            "meldung",
            "haenger-melden",
            "eskalation",
        ],
    ) && has(
        text,
        "board-lesen",
        &[
            "`timeout 15 fs-tasks list",
            "never without `timeout`",
            "never starts or restarts a server",
        ],
    ) && has(
        text,
        "haenger-weiche",
        &["then: haenger-melden", "a missing result is never green"],
    ) && has(
        text,
        "haenger-melden",
        &["NO agent starts or restarts a server", "then: end"],
    ) && has(text, "remote-lage", &["`git ls-remote`", "No `gh` call"])
        && has(
            text,
            "karten-abgleichen",
            &[
                "ORDER of the cards is NEVER changed",
                "ONE card per run",
                "timeout 15 fs-tasks claim",
                "carries red and runde",
            ],
        )
        && has(
            text,
            "slots-verteilen",
            &[
                "at most 2 builds",
                "SLOT RULE",
                "WRITES code",
                "night order",
            ],
        )
        && has(
            text,
            "meldung",
            &[
                "bundled",
                "per lane",
                "WORD FOR WORD",
                "file path",
                "`git ls-remote`",
            ],
        )
        && has(
            text,
            "abgleich-weiche",
            &[
                "($context.karten.runde // 0) >= 3",
                "then: eskalation",
                "then: karten-abgleichen",
            ],
        )
        && has(
            text,
            "slots-weiche",
            &[
                "($context.verteilung.runde // 0) >= 3",
                "then: eskalation",
                "then: slots-pruefen",
            ],
        )
        && has(text, "eskalation", &["then: end"])
        && has(text, "meldung", &["then: end"])
}

#[test]
fn lanes_round_is_led_in_fs_tasks_with_counted_loops() {
    let t = read("bahnen");
    assert!(lanes(&t));
    for (from, to) in [
        ("`timeout 15 fs-tasks list", "`fs-tasks list"),
        ("ORDER of the cards is NEVER changed", "order may change"),
        (
            "NO agent starts or restarts a server",
            "the agent restarts the server",
        ),
        ("`git ls-remote`", "memory"),
        ("($context.karten.runde // 0) >= 3", "false"),
        ("($context.verteilung.runde // 0) >= 3", "false"),
        ("WORD FOR WORD", "roughly"),
        ("at most 2 builds at the same time (`maximum`)", "builds"),
    ] {
        assert!(!lanes(&t.replace(from, to)), "{from}");
    }
    assert!(!lanes(&t.replace("  - slots-verteilen:", "  - verteilen:")));
    assert!(!lanes(&move_after(&t, "remote-lage", "meldung")));
    // the escalation must end, else it falls out of the file
    let (head, tail) = t.split_once("  - eskalation:\n").unwrap();
    assert!(!lanes(&format!(
        "{head}  - eskalation:\n{}",
        tail.replacen("then: end", "then: continue", 1)
    )));
}

#[test]
fn lanes_round_files_and_mutants_are_red_in_the_schema_check() {
    assert!(read("bahnen").contains("name: bahnen"));
    // a `then` that names no step is red in the schema check
    assert_red("bahnen", "no step of this flow", |t| {
        t.replacen("then: haenger-melden", "then: nirgends", 1)
    });
    assert_red("grundfassung", "no step of this flow", |t| {
        t.replacen("then: buendel-sammeln", "then: nirgends", 1)
    });
    assert_red("grundfassung", "no step of this flow", |t| {
        t.replacen(
            "$context.bildpaar.red != false }\n            then: bild-eskalation",
            "$context.bildpaar.red != false }\n            then: nirgends",
            1,
        )
    });
}

#[test]
fn chain_rebase_hint_names_the_result_fields() {
    let t = read("katalog");
    let hint = |t: &str| {
        has(
            t,
            "kette-rebase",
            &[
                "HINT for the later script `chain-rebase`",
                "`aktuell`",
                "`rest`",
                "`erster`",
                "{ kette: . }",
            ],
        )
    };
    assert!(hint(&t));
    assert!(!hint(&t.replace("`rest`", "`x`")));
    assert!(!hint(
        &t.replace("HINT for the later script `chain-rebase`", "note")
    ));
}

#[test]
fn opus_conditions_hang_pretag_and_order() {
    let b = read("bahnen");
    let hang = |t: &str| has(t, "haenger-weiche", &["$context.board.haenger != false"]);
    assert!(hang(&b));
    assert!(!hang(&b.replace(
        "$context.board.haenger != false",
        "$context.board.haenger == true"
    )));
    let g = read("grundfassung");
    let pre = |t: &str| {
        has(
            t,
            "vor-tag",
            &[
                "verdict: ${ $context.verdict }",
                "bildpaar: ${ $context.bildpaar }",
                "verdict.accepted != true",
                "bildpaar.red != false",
            ],
        )
    };
    assert!(pre(&g));
    assert!(!pre(
        &g.replace("verdict: ${ $context.verdict }", "verdict: x")
    ));
    assert!(!pre(
        &g.replace("bildpaar: ${ $context.bildpaar }", "bildpaar: x")
    ));
    assert!(!pre(&g.replace("verdict.accepted != true", "x")));
    let ord = |t: &str| {
        has(
            t,
            "slots-verteilen",
            &[
                "reihenfolge: ${ .input.reihenfolge }",
                "1. finished states (ff / tag), 2. Opus reviews, 3. gates",
            ],
        ) && t.contains("reihenfolge (ordered lane list")
    };
    assert!(ord(&b));
    assert!(!ord(&b.replace("reihenfolge: ${ .input.reihenfolge }", "x")));
    assert!(!ord(&b.replace("2. Opus reviews", "2. x")));
    let one = |t: &str| {
        t.matches("vorbild: ${ .input.vorbild }").count() == 1
            && has(
                t,
                "recherche",
                &["vorbild: ${ .input.vorbild }", "never pick one yourself"],
            )
            && has(
                t,
                "feature-liste",
                &["`features.vorbild` is the model of `.input.vorbild`"],
            )
            && has(t, "picture-ok", &["model: ${ $context.features.vorbild }"])
            && has(
                t,
                "hauptansicht",
                &["vorbild: ${ $context.features.vorbild }"],
            )
            && has(t, "bildpaar", &["vorbild: ${ $context.features.vorbild }"])
    };
    assert!(one(&g));
    assert!(!one(&g.replace(
        "model: ${ $context.features.vorbild }",
        "model: ${ .input.vorbild }"
    )));
    assert!(!one(&g.replacen(
        "        vorbild: ${ .input.vorbild }\n        dossier:",
        "        dossier:",
        1
    )));
    assert!(b.contains("lane 3 (people), lane 1 (components), lane 4, lane 5, lane 2"));
    assert!(has(&g, "bildpaar", &["ONE source"]));
}
