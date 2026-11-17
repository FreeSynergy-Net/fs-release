use super::*;

#[test]
fn counter_a_missing_expected_runs_step_is_caught() {
    let text = read("ff-und-tag");
    // Mutant 1: the step is struck (both steps removed).
    let mut gone = text.clone();
    for s in ["erwartete-laeufe", "laeufe-weiter"] {
        gone = gone.replace(&step_body(&gone, s), "");
    }
    assert!(!ff_expected_runs(&gone));
    // Mutant 2: the step stands behind `tag`.
    assert!(!ff_expected_runs(&swapped(
        &text,
        "erwartete-laeufe",
        "tag"
    )));
    assert!(!ff_expected_runs(&swapped(&text, "laeufe-weiter", "tag")));
    // Mutant 3: the rules are weakened.
    assert!(!ff_expected_runs(
        &text.replace("a missing run is RED", "a missing run is fine")
    ));
    assert!(!ff_expected_runs(
        &text.replace("EVERY run expected", "one run expected")
    ));
    assert!(!ff_expected_runs(
        &text.replace("gh run list --commit <head>", "gh run view")
    ));
    assert!(!ff_expected_runs(&text.replace(
        "when: '${ $context.laeufe.status == \"green\" }'",
        ""
    )));
    // The whole-flow check catches the moved step as well.
    assert!(!ff_flow(&swapped(&text, "erwartete-laeufe", "tag")));
}

/// No step falls sequentially into `ci-neustart`: the step before it is the switch `ci-weiter`, its `gruen` goes explicitly to `secrets-pruefen`
/// (no `then: continue` anywhere in it), and `ci-neustart` is named only by `haengt-erstmals`.
pub fn ff_no_fall_into_restart(text: &str) -> bool {
    let names = step_names(text);
    let i = names.iter().position(|n| n == "ci-neustart").unwrap_or(0);
    let wait = step_body(text, "ci-weiter");
    let green = wait
        .split("- gruen:")
        .nth(1)
        .unwrap_or("")
        .split("- unbekannt:")
        .next()
        .unwrap_or("");
    i > 0
        && names[i - 1] == "ci-weiter"
        && green.contains("then: secrets-pruefen")
        && !wait.contains("then: continue")
        && text.matches("then: ci-neustart").count() == 1
        && wait
            .split("- haengt-erstmals:")
            .nth(1)
            .unwrap_or("")
            .split("- haengt-wieder:")
            .next()
            .unwrap_or("")
            .contains("then: ci-neustart")
}

#[test]
fn ff_und_tag_green_ci_never_falls_into_the_restart() {
    assert!(ff_no_fall_into_restart(&read("ff-und-tag")));
}

#[test]
fn counter_a_green_that_falls_through_is_caught() {
    let text = read("ff-und-tag");
    let open = text.replace(
        "            when: '${ $context.ci.status == \"green\" }'\n            then: secrets-pruefen\n",
        "            when: '${ $context.ci.status == \"green\" }'\n            then: continue\n",
    );
    assert_ne!(open, text);
    assert!(!ff_no_fall_into_restart(&open));
    let gone = text.replace(
        "            when: '${ $context.ci.status == \"green\" }'\n            then: secrets-pruefen\n",
        "            when: '${ $context.ci.status == \"green\" }'\n",
    );
    assert!(!ff_no_fall_into_restart(&gone));
}
