use super::*;

#[test]
fn the_red_loops_of_dossier_and_frontend_round_stop_where_they_have_a_counter() {
    // dossier: the block assignment (rc 3 of the script); frontend-round: only the style analysis has a counter, the loops
    // `gates`, `menue-vergleich` and `acceptance` have none yet (open point, see CHANGELOG), so they are named as skipped
    let dossier = read("dossier");
    assert!(red_loops_escalate(&dossier, "eingabe-fehler", &[]));
    let case = "        - runden:\n            when: '${ $context.bausteine.red and ($context.bausteine.runde // 0) >= 3 }'\n            then: eingabe-fehler\n";
    assert!(dossier.contains(case));
    assert!(!red_loops_escalate(
        &dossier.replacen(case, "", 1),
        "eingabe-fehler",
        &[]
    ));
    assert!(!red_loops_escalate(
        &dossier.replacen(">= 3 }'", ">= 99 }'", 1),
        "eingabe-fehler",
        &[]
    ));
    let round = read("frontend-round");
    let open = [
        "picture-again",
        "gates-again",
        "menue-vergleich-again",
        "acceptance-again",
    ];
    assert!(red_loops_escalate(&round, "stil-eskalation", &open));
    let case = "        - runden:\n            when: '${ $context.stil.red and ($context.stil.runde // 0) >= 3 }'\n            then: stil-eskalation\n";
    assert!(round.contains(case));
    assert!(!red_loops_escalate(
        &round.replacen(case, "", 1),
        "stil-eskalation",
        &open
    ));
    assert!(!red_loops_escalate(
        &round.replacen(">= 3 }'", ">= 99 }'", 1),
        "stil-eskalation",
        &open
    ));
    // the open loops are exactly those without a counter: with no skip list the check is red
    assert!(!red_loops_escalate(&round, "stil-eskalation", &[]));
}

#[test]
fn security_packages_release_server_run_come_in_this_order() {
    let text = read(FLOW);
    assert!(release_comes_last(&text));
    // counter-tests: the old order, the round releasing by itself
    assert!(!release_comes_last(&swapped(
        &text,
        "release",
        "sicherheit"
    )));
    assert!(!release_comes_last(&swapped(
        &text,
        "server-lauf",
        "release"
    )));
    assert!(!release_comes_last(&text.replacen(
        "          release: false\n",
        "          release: true\n",
        1
    )));
}

#[test]
fn step_5_checks_formats_only_and_the_round_trip_is_checked_in_9a() {
    let text = read(FLOW);
    assert!(formats_before_commands(&text));
    let old = text.replacen(
        "NO import or export command and NO round trip here",
        "for every named format an import and an export command exist and a round trip over the fixtures keeps the data",
        1,
    );
    assert_ne!(old, text);
    assert!(!formats_before_commands(&old));
    // counter-tests: step 5 calls the boundary check of step 9a (or the old name), step 9a calls the format check
    assert!(!formats_before_commands(&text.replacen(
        "call: format-check:",
        "call: roundtrip-check:",
        1
    )));
    assert!(!formats_before_commands(&text.replacen(
        "call: boundary-check:",
        "call: format-check:",
        1
    )));
}

#[test]
fn an_own_core_can_pass_the_check_and_the_exits_of_p0_are_reported() {
    let text = read(FLOW);
    assert!(core_outcome_is_checkable(&text));
    assert!(!core_outcome_is_checkable(
        &text.replace("no pin is asked for", "a pin exists")
    ));
    assert!(p0_exits_are_reported(&text));
    assert!(!p0_exits_are_reported(&text.replacen(
        "then: vorhanden-bericht",
        "then: end",
        1
    )));
}

#[test]
fn sub_workflows_get_every_input_they_read_and_the_dossier_is_the_models() {
    for caller in ["programm-neu", "frontend-round"] {
        assert_eq!(run_inputs_are_given(caller), Vec::<String>::new());
    }
    let text = read(FLOW);
    assert!(inputs_are_declared(&text));
    // step 11 and step 12 ask for the same dossier of the MODEL program (`vorbild`), so step 12 finds it green; the `frontend`
    // of step 12 is our own program (`programm`), never the model
    let line = |s: &str, key: &str| {
        step_body(&text, s)
            .lines()
            .find(|l| l.trim().starts_with(key))
            .map(str::to_string)
    };
    assert!(line("vorbild-dossier", "vorbild: ").is_some());
    assert_eq!(
        line("vorbild-dossier", "vorbild: "),
        line("frontend", "vorbild: ")
    );
    assert!(line("vorbild-dossier", "frontend: ").is_none());
    assert!(line("frontend", "frontend: ")
        .unwrap()
        .contains("input.programm"));
    // counter-tests: an input the callee reads dropped (every sub-workflow call), a name changed
    for key in ["vorbild: ", "blinc: ", "bestand: "] {
        let body = step_body(&text, "vorbild-dossier");
        let line = body.lines().find(|l| l.trim().starts_with(key)).unwrap();
        let dropped = text.replacen(&body, &body.replace(&format!("{line}\n"), ""), 1);
        assert_ne!(dropped, text, "{key}");
        assert!(!run_inputs_missing(FLOW, &dropped).is_empty(), "{key}");
    }
    let body = step_body(&text, "frontend");
    for key in ["frontend: ", "vorbild: ", "blinc: "] {
        let line = body.lines().find(|l| l.trim().starts_with(key)).unwrap();
        let dropped = text.replacen(&body, &body.replace(&format!("{line}\n"), ""), 1);
        assert!(
            !run_inputs_missing(FLOW, &dropped).is_empty(),
            "frontend {key}"
        );
    }
    let round = read("frontend-round");
    for key in ["vorbild: ", "blinc: "] {
        let body = step_body(&round, "dossier");
        let line = body.lines().find(|l| l.trim().starts_with(key)).unwrap();
        let dropped = round.replacen(&body, &body.replace(&format!("{line}\n"), ""), 1);
        assert!(
            !run_inputs_missing("frontend-round", &dropped).is_empty(),
            "{key}"
        );
    }
    // counter-tests: an input dropped, a name changed
    let dropped = text.replacen("          release: false\n", "", 1);
    assert!(!release_comes_last(&dropped));
    assert!(!inputs_are_declared(&text.replacen(
        "${ .input.programm }",
        "${ .input.unbekannt }",
        1
    )));
}

/// ADR 0086: after the data model comes the script check of the typed storage; red goes back to the data model, the third red round escalates.
pub fn storage_check_is_sound(text: &str) -> bool {
    let c = step_body(text, "speicher-check");
    let a = step_body(text, "speicher-again");
    position(text, "modell-again") < position(text, "speicher-check")
        && position(text, "speicher-check") < position(text, "speicher-again")
        && position(text, "speicher-again") < position(text, "schnittstelle")
        && c.contains("actor: machine")
        && c.contains("kind: objective")
        && c.contains("storage-typed-check")
        && c.contains("adr: '0086'")
        && a.contains("then: datenmodell")
        && a.contains(">= 3")
        && a.contains("then: eskalation")
}

#[test]
fn the_typed_storage_is_checked_after_the_data_model_adr_0086() {
    let text = read(FLOW);
    assert!(storage_check_is_sound(&text));
    assert!(!storage_check_is_sound(
        &text.replace("storage-typed-check", "an-agent-looks")
    ));
    assert!(!storage_check_is_sound(
        &text.replace("adr: '0086'", "adr: '0046'")
    ));
    let b = step_body(&text, "speicher-again");
    assert!(!storage_check_is_sound(&text.replacen(
        &b,
        &b.replace("then: datenmodell", "then: continue"),
        1
    )));
    assert!(!storage_check_is_sound(&swapped(
        &text,
        "speicher-check",
        "schnittstelle"
    )));
    assert!(!step_names(&text).contains(&"doku".to_string()));
}

/// Opus condition 1 (menues): a `picture-acceptance` that ends without a verdict (`dossier-menue-fehlt`, `menue-bild-fehlt`) must
/// never let `frontend-round` build; only `accepted == true` continues, everything else ends in the human step `bild-eskalation`.
pub fn never_builds_unaccepted(text: &str) -> bool {
    let body = step_body(text, "picture-again");
    let lines: Vec<&str> = body.lines().map(str::trim).collect();
    // exactly one `then: continue`, directly after the `accepted == true` condition
    let Some(i) = lines.iter().position(|l| l.contains("accepted == true")) else {
        return false;
    };
    let one_continue =
        body.matches("then: continue").count() == 1 && lines.get(i + 1) == Some(&"then: continue");
    // the case without `when` (the default) is the last one and goes to the human step
    let default_ok = lines
        .iter()
        .skip_while(|l| !l.starts_with("- no-verdict:"))
        .find(|l| l.starts_with("then:"))
        == Some(&"then: bild-eskalation");
    let esc = step_body(text, "bild-eskalation");
    one_continue && default_ok && esc.contains("then: end") && esc.contains("actor: human")
}

#[test]
fn frontend_round_never_builds_without_an_accepted_picture() {
    let text = read("frontend-round");
    assert!(never_builds_unaccepted(&text));
    assert!(step_body(&text, "picture-again").contains("then: gap-list"));
    // counter-test (mutant): the default goes back to `continue` (= build) -> red
    let mutant = text.replacen("then: bild-eskalation", "then: continue", 1);
    assert_ne!(mutant, text);
    assert!(!never_builds_unaccepted(&mutant));
}
