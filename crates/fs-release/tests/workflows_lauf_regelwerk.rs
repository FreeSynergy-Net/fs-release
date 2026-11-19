//! Pattern: none (test). More counter-tests of the run rule set (`workflows/run/rules.yaml`): the end state, unknown sources and targets, the fixed
//! start and name, the mandatory reason of `wartet` and the one-state check, a missing rule set file.

mod support;
use support::workflows::{assert_red_without_rules, assert_rules_red, run_state};

const REASONS: [&str; 3] = ["auf-entscheidung", "auf-anfrage", "in-prüfung"];

#[test]
fn mutation_the_end_state_has_no_way_out_and_is_reached_only_from_archive_from() {
    assert_rules_red("exactly one end state", |r| {
        r.replacen("  archiviert: []", "  archiviert: [offen]", 1)
    });
    assert_rules_red("exactly one end state", |r| {
        r.replacen(
            "  fehlgeschlagen: [läuft, pausiert, abgebrochen, archiviert]",
            "  fehlgeschlagen: []",
            1,
        )
    });
    assert_rules_red(
        "transition 'pausiert' -> 'archiviert' is not allowed",
        |r| {
            r.replacen(
                "läuft, wartet, abgebrochen]",
                "läuft, wartet, abgebrochen, archiviert]",
                1,
            )
        },
    );
    assert_rules_red("transition 'offen' -> 'archiviert' is not allowed", |r| {
        r.replacen(
            "wartet, pausiert, abgebrochen]",
            "wartet, pausiert, abgebrochen, archiviert]",
            1,
        )
    });
}

#[test]
fn mutation_unknown_source_target_and_non_list_rules_are_red() {
    assert_rules_red("transition from unknown state 'foo'", |r| {
        format!("{r}  foo: [offen]\n")
    });
    for bad in ["foo", "{a: b}", "[x]"] {
        assert_rules_red("transitions of 'offen': a list of states", |r| {
            r.replacen("  offen: [läuft,", &format!("  offen: [{bad}, läuft,"), 1)
        });
    }
    assert_rules_red("transitions of 'pausiert': a list of states", |r| {
        r.replacen(
            "  pausiert: [offen, läuft, wartet, abgebrochen]",
            "  pausiert: abgebrochen",
            1,
        )
    });
    assert_rules_red("transitions of 'offen': a target is listed twice", |r| {
        r.replacen("  offen: [läuft,", "  offen: [läuft, läuft,", 1)
    });
}

#[test]
fn mutation_start_name_and_final_lists_are_fixed() {
    assert_rules_red("`start` is 'pausiert'", |r| {
        r.replacen("start: offen", "start: pausiert", 1)
    });
    assert_rules_red("`name` is 'mine'", |r| {
        r.replacen("name: freesynergy", "name: mine", 1)
    });
    for key in ["final", "reopen", "archive-from"] {
        assert_rules_red(&format!("`{key}` is a list of states"), |r| {
            r.replacen(&format!("{key}: ["), &format!("{key}: [foo, "), 1)
        });
    }
}

#[test]
fn mutation_wartet_always_has_a_reason() {
    assert_rules_red("only 'wartet' has reasons", |r| {
        r.replacen(
            "  wartet: [auf-entscheidung, auf-anfrage, in-prüfung]\n",
            "  wartet: []\n",
            1,
        )
    });
    assert_rules_red("only 'wartet' has reasons", |r| {
        r.replacen(
            "reasons:\n  wartet: [auf-entscheidung, auf-anfrage, in-prüfung]\n",
            "reasons: {}\n",
            1,
        )
    });
    assert_rules_red("only 'wartet' has reasons", |r| {
        r.replacen("reasons:\n", "reasons:\n  läuft: [x]\n", 1)
    });
    assert_rules_red("non-empty list of reasons", |r| {
        r.replacen(
            "in-prüfung]\ntransitions",
            "in-prüfung, in-prüfung]\ntransitions",
            1,
        )
    });
}

#[test]
fn a_run_in_wartet_needs_a_reason_out_of_the_list() {
    for reason in REASONS {
        assert_eq!(
            run_state("wartet", Some(reason)).0,
            if support::have_yaml() { 0 } else { 2 }
        );
    }
    let want = if support::have_yaml() { 1 } else { 2 };
    for (state, reason, needle) in [
        ("wartet", None, "needs a reason"),
        ("wartet", Some("weil"), "is not one of"),
        ("wartet", Some("entscheidung"), "is not one of"),
        ("läuft", Some("auf-anfrage"), "has no reason"),
        ("blubb", None, "unknown state"),
    ] {
        let (code, out) = run_state(state, reason);
        assert_eq!(code, want, "{out}");
        if want == 1 {
            assert!(out.contains(needle), "{out}");
        }
    }
    assert_eq!(
        run_state("läuft", None).0,
        if support::have_yaml() { 0 } else { 2 }
    );
}

#[test]
fn mutation_a_missing_rule_set_file_is_red() {
    assert_red_without_rules("the run rule set is not readable");
}
