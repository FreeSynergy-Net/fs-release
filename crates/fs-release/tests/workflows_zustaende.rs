//! Pattern: none (test). The eight workflow states and the transition rules (`workflows/STATES.md`): every flow carries them,
//! and each rule of `scripts/check_workflows.py` has a counter-test on a changed copy.

mod support;
use std::fs;
use support::workflows::{assert_red, read, root, FILES};

const STATES_LINE: &str = "    freesynergy.net:states: [offen, wartet-auf-entscheidung, auf-anfrage, in-prüfung, pausiert, abgebrochen, erledigt, archiviert]\n";

fn drop_line(t: &str, start: &str) -> String {
    let mut gone = false;
    let mut out = String::new();
    for l in t.lines() {
        if !gone && l.starts_with(start) {
            gone = true;
            continue;
        }
        out.push_str(l);
        out.push('\n');
    }
    out
}

#[test]
fn every_workflow_carries_the_eight_states_and_the_transition_rules() {
    for f in FILES {
        let t = read(f);
        assert!(
            t.contains(STATES_LINE),
            "{f}.yaml: the eight states (workflows/STATES.md)"
        );
        for rule in [
            "      pausiert: [",
            "      archiviert: []",
            "      abgebrochen: [offen, archiviert]",
        ] {
            assert!(t.contains(rule), "{f}.yaml: transition rule {rule:?}");
        }
    }
}

#[test]
fn the_state_standard_document_names_every_state_and_rule() {
    let doc = fs::read_to_string(root().join("workflows/STATES.md")).unwrap();
    for s in [
        "offen",
        "wartet-auf-entscheidung",
        "auf-anfrage",
        "in-prüfung",
        "pausiert",
        "abgebrochen",
        "erledigt",
        "archiviert",
        "stationen",
    ] {
        assert!(
            doc.contains(&format!("`{s}`")) || doc.contains(s),
            "STATES.md: {s}"
        );
        assert!(STATES_LINE.contains(s) || s == "stationen", "{s}");
    }
}

#[test]
fn mutation_state_deleted_from_any_workflow_is_red() {
    for f in FILES {
        assert_red(f, "state 'wartet-auf-entscheidung' is missing", |t| {
            t.replacen("wartet-auf-entscheidung, auf-anfrage, in-prüfung, pausiert, abgebrochen, erledigt, archiviert]", "auf-anfrage, in-prüfung, pausiert, abgebrochen, erledigt, archiviert]", 1)
        });
    }
}

#[test]
fn mutation_states_block_removed_from_any_workflow_is_red() {
    for f in FILES {
        assert_red(f, &format!("{f}.yaml:"), |t| {
            drop_line(&t, STATES_LINE.trim_end())
        });
    }
}

#[test]
fn mutation_state_renamed_is_red() {
    assert_red("lift", "state 'pausiert' is missing", |t| {
        t.replacen(
            "in-prüfung, pausiert, abgebrochen, erledigt, archiviert]",
            "in-prüfung, angehalten, abgebrochen, erledigt, archiviert]",
            1,
        )
    });
    assert_red("lift", "unknown state 'angehalten'", |t| {
        t.replacen(
            "in-prüfung, pausiert, abgebrochen, erledigt, archiviert]",
            "in-prüfung, angehalten, abgebrochen, erledigt, archiviert]",
            1,
        )
    });
}

#[test]
fn mutation_transition_rule_missing_or_incomplete_is_red() {
    assert_red("bugfix", "transition rule of 'pausiert' is missing", |t| {
        drop_line(&t, "      pausiert: [")
    });
    assert_red(
        "bugfix",
        "transition 'abgebrochen' -> 'offen' is missing",
        |t| {
            t.replacen(
                "abgebrochen: [offen, archiviert]",
                "abgebrochen: [archiviert]",
                1,
            )
        },
    );
    assert_red(
        "bugfix",
        "transition 'offen' -> 'pausiert' is missing",
        |t| {
            t.replacen(
                "offen: [stationen, wartet-auf-entscheidung, auf-anfrage, pausiert, abgebrochen]",
                "offen: [stationen, wartet-auf-entscheidung, auf-anfrage, abgebrochen]",
                1,
            )
        },
    );
    assert_red("bugfix", "freesynergy.net:transitions is missing", |t| {
        t.replacen(
            "freesynergy.net:transitions:",
            "freesynergy.net:uebergaenge:",
            1,
        )
    });
}

#[test]
fn mutation_archiviert_only_from_erledigt_or_abgebrochen_and_without_way_out() {
    assert_red("bugfix", "'pausiert' -> 'archiviert' is not allowed", |t| {
        t.replacen(
            "in-prüfung, abgebrochen]\n",
            "in-prüfung, abgebrochen, archiviert]\n",
            1,
        )
    });
    assert_red("bugfix", "'archiviert' has no way out", |t| {
        t.replacen("archiviert: []", "archiviert: [offen]", 1)
    });
}

#[test]
fn mutation_a_step_named_like_a_state_is_red() {
    assert_red("lift", "carries the name of a state", |t| {
        t.replacen("  - lock:\n", "  - pausiert:\n", 1)
    });
}

#[test]
fn mutation_pausiert_and_abgebrochen_must_be_reachable_from_every_live_state() {
    let stationen = "      stationen: [stationen, wartet-auf-entscheidung, auf-anfrage, in-prüfung, pausiert, abgebrochen, erledigt]";
    assert_red(
        "bugfix",
        "transition 'stationen' -> 'pausiert' is missing",
        |t| t.replacen(stationen, &stationen.replace("pausiert, ", ""), 1),
    );
    assert_red(
        "bugfix",
        "transition 'stationen' -> 'abgebrochen' is missing",
        |t| t.replacen(stationen, &stationen.replace("abgebrochen, ", ""), 1),
    );
    assert_red(
        "bugfix",
        "transition 'offen' -> 'abgebrochen' is missing",
        |t| {
            t.replacen(
                "offen: [stationen, wartet-auf-entscheidung, auf-anfrage, pausiert, abgebrochen]",
                "offen: [stationen, wartet-auf-entscheidung, auf-anfrage, pausiert]",
                1,
            )
        },
    );
    assert_red(
        "bugfix",
        "transition 'in-prüfung' -> 'pausiert' is missing",
        |t| {
            t.replacen(
                "auf-anfrage, pausiert, abgebrochen, erledigt]\n      pausiert",
                "auf-anfrage, abgebrochen, erledigt]\n      pausiert",
                1,
            )
        },
    );
}
