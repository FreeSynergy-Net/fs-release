use super::*;

#[test]
fn the_release_of_frontend_round_is_skipped_by_a_switch_and_not_by_an_if() {
    let text = read("frontend-round");
    assert!(release_is_skipped_by_a_switch(&text));
    // counter-tests (mutations): the switch case removed, the `if` back on the release step, the condition changed, the switch moved away
    let weiche = step_body(&text, "release-weiche");
    let no_case = text.replacen(&weiche, &weiche.replace("then: end", "then: continue"), 1);
    assert_ne!(no_case, text);
    assert!(!release_is_skipped_by_a_switch(&no_case));
    assert!(!release_is_skipped_by_a_switch(&text.replacen(
        "  - release:\n      then: end\n",
        "  - release:\n      if: ${ .input.release != false }\n      then: end\n",
        1
    )));
    assert!(!release_is_skipped_by_a_switch(&text.replacen(
        ".input.release == false",
        ".input.release == true",
        1
    )));
    assert!(!release_is_skipped_by_a_switch(&swapped(
        &text,
        "release-weiche",
        "release"
    )));
    // the whole step removed: the check is red (no such step), and the file check is red as well (`release` must be reached some way)
    let gone = text.replacen(&format!("  - release-weiche:\n{weiche}\n"), "", 1);
    assert_ne!(gone, text);
    assert!(
        std::panic::catch_unwind(|| release_is_skipped_by_a_switch(&gone)).map_or(true, |ok| !ok)
    );
}

#[test]
fn the_coordinator_starts_with_the_red_check_and_events_are_net_freesynergy() {
    assert_eq!(position(&read("coordination-round"), "rot-pruefen"), 0);
    assert_red(
        "coordination-round",
        "first step must be `rot-pruefen`",
        |t| swapped(&t, "rot-pruefen", "cost-log"),
    );
    assert_red(
        "release-flow",
        "does not start with net.freesynergy.",
        |t| {
            t.replacen(
                "type: net.freesynergy.release.published",
                "type: net.example.x",
                1,
            )
        },
    );
}
