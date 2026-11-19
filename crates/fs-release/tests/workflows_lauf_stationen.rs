//! Pattern: none (test). What a workflow file owns: no states of its own, rules only for its own stations (`freesynergy.net:station-rules`),
//! and (the safety rules of a status command are in `workflows_lauf_befehl.rs`).

mod support;
use support::workflows::{assert_green, assert_red, FILES};

const STATES: [&str; 8] = [
    "offen",
    "läuft",
    "wartet",
    "pausiert",
    "fehlgeschlagen",
    "abgebrochen",
    "erledigt",
    "archiviert",
];
const RULES: &str = "    freesynergy.net:station-rules:\n      finding:\n";

/// `bugfix` with a block in `document.metadata` (the line `  title:` is followed by it).
fn with_doc_meta(t: &str, block: &str) -> String {
    t.replacen("  title:", &format!("  metadata:\n{block}  title:"), 1)
}

fn with_rule(t: &str, body: &str) -> String {
    with_doc_meta(t, &format!("{RULES}{body}"))
}

#[test]
fn a_flow_with_valid_station_rules_and_the_rule_set_name_is_green() {
    assert_green("bugfix", |t| {
        with_rule(
            &t,
            "        allow: [maintainer]\n        from: [finding]\n        confirm: true\n        on-enter:\n          - assign-role: reviewer\n          - set-wait-reason: auf-anfrage\n          - create-decision: ship it\n          - run-status-command: finding\n",
        )
    });
    assert_green("bugfix", |t| {
        with_doc_meta(&t, "    freesynergy.net:run-rules: freesynergy\n")
    });
}

#[test]
fn mutation_a_flow_with_its_own_states_transitions_or_another_rule_set_is_red() {
    for f in FILES {
        assert_red(f, "freesynergy.net:states must not be there", |t| {
            with_doc_meta(&t, "    freesynergy.net:states: [offen]\n")
        });
    }
    assert_red(
        "bugfix",
        "freesynergy.net:transitions must not be there",
        |t| {
            with_doc_meta(
                &t,
                "    freesynergy.net:transitions:\n      offen: [läuft]\n",
            )
        },
    );
    for v in ["mine", "9", "[freesynergy]"] {
        assert_red("bugfix", "only the rule set 'freesynergy' exists", |t| {
            with_doc_meta(&t, &format!("    freesynergy.net:run-rules: {v}\n"))
        });
    }
}

#[test]
fn mutation_a_station_rule_for_an_unknown_station_or_with_bad_parts_is_red() {
    assert_red(
        "bugfix",
        "station rule for 'nowhere': no step of this flow has this name",
        |t| {
            with_doc_meta(
                &t,
                "    freesynergy.net:station-rules:\n      nowhere:\n        allow: [maintainer]\n",
            )
        },
    );
    assert_red(
        "bugfix",
        "station rule of 'finding': only allow, confirm, from, on-enter are allowed",
        |t| with_rule(&t, "        foo: [x]\n"),
    );
    assert_red("bugfix", "`allow` is a non-empty list of roles", |t| {
        with_rule(&t, "        allow: []\n")
    });
    assert_red("bugfix", "`from` is a list of steps of this flow", |t| {
        with_rule(&t, "        from: [nowhere]\n")
    });
    assert_red("bugfix", "`confirm` is true or false", |t| {
        with_rule(&t, "        confirm: yes please\n")
    });
    assert_red("bugfix", "must be a mapping station -> rule", |t| {
        with_doc_meta(&t, "    freesynergy.net:station-rules: [finding]\n")
    });
}

#[test]
fn mutation_on_enter_needs_actions_with_a_value() {
    for body in [
        "        on-enter: [assign-role]\n",
        "        on-enter:\n          - format-disk: all\n",
        "        on-enter:\n          - assign-role: x\n            create-decision: y\n",
        "        on-enter: assign-role\n",
    ] {
        assert_red("bugfix", "`on-enter` is a list of `- action: value`", |t| {
            with_rule(&t, body)
        });
    }
    for action in [
        "assign-role",
        "create-decision",
        "set-wait-reason",
        "run-status-command",
    ] {
        assert_red(
            "bugfix",
            &format!("the action '{action}' needs a value"),
            |t| {
                with_rule(
                    &t,
                    &format!("        on-enter:\n          - {action}: \"  \"\n"),
                )
            },
        );
    }
    assert_red(
        "bugfix",
        "set-wait-reason 'weil' is not one of auf-entscheidung, auf-anfrage, in-prüfung",
        |t| with_rule(&t, "        on-enter:\n          - set-wait-reason: weil\n"),
    );
}

#[test]
fn mutation_every_state_name_as_a_step_name_is_red() {
    for name in STATES {
        assert_red("bugfix", "carries the name of a state", |t| {
            t.replacen("  - finding:\n", &format!("  - {name}:\n"), 1)
        });
    }
}
