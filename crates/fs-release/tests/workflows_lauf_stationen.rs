//! Pattern: none (test). What a workflow file owns: no states of its own, rules only for its own stations (`freesynergy.net:station-rules`),
//! and the safety rules of a status command (`timeout`, local origin, no shell, script under `scripts/` or on the allow list).

mod support;
use support::workflows::{assert_green, assert_red, FILES};

const STATES: [&str; 10] = [
    "offen",
    "wartet-auf-entscheidung",
    "auf-anfrage",
    "in-prüfung",
    "pausiert",
    "fehlgeschlagen",
    "abgebrochen",
    "erledigt",
    "archiviert",
    "stationen",
];
const META: &str = "      metadata:\n";
const ACTOR: &str = "        freesynergy.net:actor: human\n";

/// `bugfix` with a rule block in `document.metadata` (the line `  title:` is followed by it).
fn with_doc_meta(t: &str, block: &str) -> String {
    t.replacen("  title:", &format!("  metadata:\n{block}  title:"), 1)
}

/// `bugfix` whose first step runs a status command; `timeout` is the block before `metadata`, `extra` further metadata lines.
fn with_status(t: &str, cmd: &str, extra: &str, timeout: &str) -> String {
    let t = t.replacen(META, &format!("{timeout}{META}"), 1);
    t.replacen(
        ACTOR,
        &format!("{ACTOR}        freesynergy.net:status-command: {cmd}\n{extra}"),
        1,
    )
}

const LOCAL: &str = "        freesynergy.net:origin: local\n";
const T30: &str = "      timeout:\n        after: PT30S\n";

#[test]
fn a_flow_without_states_and_with_a_valid_station_rule_and_status_command_is_green() {
    assert_green("bugfix", |t| {
        with_doc_meta(
            &t,
            "    freesynergy.net:station-rules:\n      finding:\n        allow: [maintainer]\n        on-enter: [assign-role, run-status-command]\n",
        )
    });
    assert_green("bugfix", |t| {
        with_status(&t, "scripts/check-x --fast", LOCAL, T30)
    });
    assert_green("bugfix", |t| {
        with_status(
            &t,
            "cargo test",
            &format!("{LOCAL}        freesynergy.net:allow: [cargo]\n"),
            "      timeout:\n        after:\n          minutes: 5\n",
        )
    });
}

#[test]
fn mutation_a_flow_with_its_own_states_or_transitions_is_red() {
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
                "    freesynergy.net:transitions:\n      offen: [stationen]\n",
            )
        },
    );
}

#[test]
fn mutation_a_station_rule_for_an_unknown_station_or_with_unknown_keys_is_red() {
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
        "station rule of 'finding': only `allow` and `on-enter` are allowed",
        |t| {
            with_doc_meta(
                &t,
                "    freesynergy.net:station-rules:\n      finding:\n        foo: [x]\n",
            )
        },
    );
    assert_red(
        "bugfix",
        "station rule of 'finding': `on-enter` is a list of",
        |t| {
            with_doc_meta(&t, "    freesynergy.net:station-rules:\n      finding:\n        on-enter: [format-disk]\n")
        },
    );
    assert_red(
        "bugfix",
        "station rule of 'finding': `allow` is a non-empty list of roles",
        |t| {
            with_doc_meta(
                &t,
                "    freesynergy.net:station-rules:\n      finding:\n        allow: []\n",
            )
        },
    );
    assert_red("bugfix", "must be a mapping station -> rule", |t| {
        with_doc_meta(&t, "    freesynergy.net:station-rules: [finding]\n")
    });
}

#[test]
fn mutation_a_status_command_without_timeout_or_with_a_bad_one_is_red() {
    assert_red("bugfix", "a status command needs a `timeout`", |t| {
        with_status(&t, "scripts/check-x", LOCAL, "")
    });
    for (after, why) in [("PT2H", "7200 s"), ("PT0S", "0 s"), ("PT1H1S", "3601 s")] {
        assert_red("bugfix", &format!("is {why}, allowed 1..3600"), |t| {
            with_status(
                &t,
                "scripts/check-x",
                LOCAL,
                &format!("      timeout:\n        after: {after}\n"),
            )
        });
    }
    assert_red("bugfix", "a status command needs a `timeout`", |t| {
        with_status(
            &t,
            "scripts/check-x",
            LOCAL,
            "      timeout:\n        after: soon\n",
        )
    });
}

#[test]
fn mutation_a_status_command_with_a_shell_or_a_foreign_origin_is_red() {
    assert_red("bugfix", "runs only for runs of local origin", |t| {
        with_status(&t, "scripts/check-x", "", T30)
    });
    assert_red("bugfix", "runs only for runs of local origin", |t| {
        with_status(
            &t,
            "scripts/check-x",
            "        freesynergy.net:origin: remote\n",
            T30,
        )
    });
    for cmd in ["bash scripts/check-x", "/bin/sh -c x", "env cargo test"] {
        assert_red("bugfix", "a shell is no status command", |t| {
            with_status(&t, cmd, LOCAL, T30)
        });
    }
    assert_red("bugfix", "a shell is no status command", |t| {
        with_status(
            &t,
            "cargo test",
            &format!("{LOCAL}        freesynergy.net:allow: [cargo, bash]\n"),
            T30,
        )
    });
    for cmd in [
        "scripts/check-x; rm",
        "scripts/check-x | tee",
        "scripts/check-x $(id)",
        "scripts/check-x > out",
    ] {
        assert_red("bugfix", "contains shell syntax", |t| {
            with_status(&t, cmd, LOCAL, T30)
        });
    }
}

#[test]
fn mutation_a_status_command_outside_scripts_and_the_allow_list_is_red() {
    assert_red("bugfix", "is neither a script under scripts/ nor in", |t| {
        with_status(&t, "cargo test", LOCAL, T30)
    });
    assert_red("bugfix", "is neither a script under scripts/ nor in", |t| {
        with_status(&t, "scripts/../evil", LOCAL, T30)
    });
    assert_red("bugfix", "is neither a script under scripts/ nor in", |t| {
        with_status(&t, "tools/check-x", LOCAL, T30)
    });
    assert_red("bugfix", "is a list of program names", |t| {
        with_status(
            &t,
            "cargo test",
            &format!("{LOCAL}        freesynergy.net:allow: cargo\n"),
            T30,
        )
    });
}

#[test]
fn mutation_every_state_name_and_stationen_as_a_step_name_is_red() {
    for name in STATES {
        assert_red("bugfix", "carries the name of a state", |t| {
            t.replacen("  - finding:\n", &format!("  - {name}:\n"), 1)
        });
    }
}
