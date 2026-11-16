//! Pattern: none (test). Wiring rules of the bundle: `resolve-head` output form (K4), [THEME-WAECHTER], [FRISCH] / [RC]. Counter-checks on changed copies.

mod support;
use support::workflows::{read, root, run_self_test, step_body, step_names};

/// K4: `resolve-head` has ONE output form, `{ head: <sha> }`; every caller keeps it as `kopf` and reads `$context.kopf.head`.
fn resolve_head_paths_are_sound(text: &str) -> bool {
    !text.contains("resolve-head")
        || (text.matches("{ kopf: . }").count() >= text.matches("call: resolve-head").count()
            && !text.contains("$context.head")
            && !text.contains("{ head: . }")
            // an expression that ends right after `kopf` (no `.head`) hands over the whole object
            && !text.contains("$context.kopf }")
            && !text.contains("$context.kopf\""))
}

#[test]
fn every_resolve_head_caller_reads_the_same_output_form() {
    let dir = root().join("workflows");
    let mut seen = 0;
    for e in std::fs::read_dir(&dir).unwrap() {
        let p = e.unwrap().path();
        if p.extension().is_some_and(|x| x == "yaml") {
            let t = std::fs::read_to_string(&p).unwrap();
            if t.contains("call: resolve-head") {
                seen += 1;
                assert!(resolve_head_paths_are_sound(&t), "{}", p.display());
            }
        }
    }
    assert!(seen >= 5, "the five callers must be found, got {seen}");
}

#[test]
fn counter_resolve_head_paths_of_the_other_form_are_caught() {
    let t = read("umbenennen");
    assert!(resolve_head_paths_are_sound(&t));
    assert!(!resolve_head_paths_are_sound(
        &t.replace("{ kopf: . }", "{ head: . }")
    ));
    assert!(!resolve_head_paths_are_sound(
        &t.replace("$context.kopf.head", "$context.head")
    ));
    // the whole object instead of its field `head`
    assert!(!resolve_head_paths_are_sound(
        &t.replace("${ $context.kopf.head }", "${ $context.kopf }")
    ));
}

/// [THEME-WAECHTER]: `testlauf` runs the theme guard after the targeted tests and a guard that is red or missing counts as red; `programm-grundgeruest` shows
/// pictures to the picture gate only after a green guard (red goes back, no result escalates); the acceptance flow carries the guard. The guard script does not
/// exist yet: the steps say so (specification, not runnable). Each part has a counter-check on a changed copy.
fn theme_guard_is_wired(testlauf: &str, belt: &str, accept: &str) -> bool {
    let tl = step_names(testlauf);
    let pos = |n: &str| tl.iter().position(|x| x == n);
    let again = step_body(testlauf, "gezielt-again");
    let bl = step_names(belt);
    let bpos = |n: &str| bl.iter().position(|x| x == n);
    let weiche = step_body(belt, "waechter-weiche");
    pos("gezielt")
        .zip(pos("theme-waechter"))
        .zip(pos("gezielt-again"))
        .is_some_and(|((a, b), c)| a < b && b < c)
        && again.contains("waechter.red == true }'\n            then: fix")
        && again
            .find("waechter.red")
            .zip(again.find("gezielt.red == false"))
            .is_some_and(|(x, y)| x < y)
        && step_body(testlauf, "theme-waechter").contains("SPECIFICATION, NOT RUNNABLE")
        && bpos("bilder-erzeugen")
            .zip(bpos("bilder-waechter"))
            .zip(bpos("waechter-weiche"))
            .zip(bpos("bilder"))
            .is_some_and(|(((a, b), c), d)| a < b && b < c && c < d)
        && weiche.contains("waechter.red == false")
        && weiche.contains("then: bilder")
        && weiche.contains("then: stufe-3")
        && weiche.contains("then: eskalation")
        && step_body(belt, "bilder").contains("waechter: ${ $context.waechter }")
        && accept.contains("[THEME-WAECHTER]")
        && accept.contains("waechter: ${ .input.waechter }")
}

#[test]
fn the_theme_guard_is_wired_into_testlauf_and_the_picture_gate() {
    let (t, b, a) = (
        read("testlauf"),
        read("programm-grundgeruest"),
        read("picture-acceptance"),
    );
    assert!(theme_guard_is_wired(&t, &b, &a));
    // counter-checks: every change that lets pictures count without a green guard is caught
    let cases: [(&str, String, String, String); 7] = [
        (
            "guard step renamed",
            t.replace("theme-waechter:", "theme-x:"),
            b.clone(),
            a.clone(),
        ),
        (
            "red guard not red",
            t.replace(
                "waechter.red == true }'\n            then: fix",
                "waechter.red == 7 }'\n            then: fix",
            ),
            b.clone(),
            a.clone(),
        ),
        (
            "guard result ignored",
            t.replace(
                "waechter.red == true }'\n            then: fix",
                "waechter.red == true }'\n            then: kopf",
            ),
            b.clone(),
            a.clone(),
        ),
        (
            "pictures skip the guard",
            t.clone(),
            b.replace("then: bilder\n        - rot", "then: fertig\n        - rot"),
            a.clone(),
        ),
        (
            "red guard goes on",
            t.clone(),
            b.replace("waechter.red == false", "waechter.red != true"),
            a.clone(),
        ),
        (
            "no result is not an escalation",
            t.clone(),
            b.replacen("then: eskalation", "then: bilder", 99),
            a.clone(),
        ),
        (
            "acceptance without the guard",
            t.clone(),
            b.clone(),
            a.replace("waechter: ${ .input.waechter }", ""),
        ),
    ];
    for (name, t2, b2, a2) in cases {
        assert!(
            !theme_guard_is_wired(&t2, &b2, &a2),
            "{name} was not caught"
        );
    }
}

/// [FRISCH] / [RC]: `doku-mitziehen` checks a fresh state (`docs_head` and the resolved `head` are passed to the gate), `check-docs` names the rule, `testlauf` keeps
/// the exit code (`rc` in the result, green only with `rc == 0`). Counter-checks on changed copies.
fn fresh_and_rc_are_wired(doku: &str, adr: &str, testlauf: &str) -> bool {
    let gate = step_body(doku, "gate");
    let again = step_body(testlauf, "gezielt-again");
    gate.contains("docs_head: ${ .input.docs_head }")
        && gate.contains("head: ${ $context.kopf.head }")
        && doku.contains("[FRISCH]")
        && doku.contains("WARNING or ERROR")
        && step_body(adr, "check-docs").contains("FRESH worktree")
        && testlauf.contains("[RC]")
        && step_body(testlauf, "gezielt").contains("`rc`")
        && again.contains("gezielt.rc != 0")
        && again.contains("gezielt.red == false and $context.gezielt.rc == 0")
}

#[test]
fn the_fresh_worktree_and_the_exit_code_rules_are_wired() {
    let (d, a, t) = (read("doku-mitziehen"), read("adr-write"), read("testlauf"));
    assert!(fresh_and_rc_are_wired(&d, &a, &t));
    let cases = [
        (
            "gate without docs_head",
            d.replace("docs_head: ${ .input.docs_head }", ""),
            a.clone(),
            t.clone(),
        ),
        (
            "check-docs in the main checkout",
            d.clone(),
            a.replace("FRESH worktree", "checkout"),
            t.clone(),
        ),
        (
            "green without rc",
            d.clone(),
            a.clone(),
            t.replace(
                "gezielt.red == false and $context.gezielt.rc == 0",
                "gezielt.red == false",
            ),
        ),
        (
            "rc ignored for red",
            d.clone(),
            a.clone(),
            t.replace("gezielt.rc != 0", "gezielt.rc != 7"),
        ),
        (
            "no rc in the result",
            d.clone(),
            a.clone(),
            t.replacen("`rc`", "`x`", 9),
        ),
    ];
    for (name, d2, a2, t2) in cases {
        assert!(
            !fresh_and_rc_are_wired(&d2, &a2, &t2),
            "{name} was not caught"
        );
    }
}

#[test]
fn the_quiet_mkdocs_check_passes_its_self_test() {
    run_self_test("test_check_mkdocs_quiet.sh");
}

/// A5: a missing guard result (the guard has no script) goes to `eskalation`, not to `fix`; the rounds are also counted by the flow (`fix-zaehlen`,
/// `fixrunden.runde`) and the round limit looks at the guard and the exit code too, so no loop fix <-> gezielt-again runs without end.
fn guard_loop_is_closed(t: &str) -> bool {
    let again = step_body(t, "gezielt-again");
    let missing = again.find("waechter-fehlt:");
    let red = again.find("- theme:");
    let limit = again.find("- runden:");
    matches!((limit, missing, red), (Some(a), Some(b), Some(c)) if a < b && b < c)
        && again.contains("$context.waechter.red == null }'\n            then: eskalation")
        && again.contains("$context.waechter.red == true }'\n            then: fix")
        && again.contains("$context.fixrunden.runde // 0) >= 3")
        && again.contains("$context.waechter.red == true) or")
        && again.contains("$context.gezielt.rc // 0) != 0")
        && step_body(t, "zaehler-start").contains("runde: 0")
        && step_body(t, "zaehler-start").contains("{ fixrunden: . }")
        && step_body(t, "fix").contains("then: fix-zaehlen")
        && step_body(t, "fix-zaehlen").contains("$context.fixrunden.runde // 0) + 1")
        && step_body(t, "fix-zaehlen").contains("then: slot")
}

#[test]
fn a_missing_guard_result_escalates_and_every_red_kind_is_counted() {
    let t = read("testlauf");
    assert!(guard_loop_is_closed(&t));
    let cases = [
        (
            "missing guard goes to fix",
            t.replace(
                "$context.waechter.red == null }'\n            then: eskalation",
                "$context.waechter.red == null }'\n            then: fix",
            ),
        ),
        (
            "missing guard rule removed",
            t.replace("waechter-fehlt:", "waechter-egal:"),
        ),
        (
            "fix counter not called",
            t.replace("then: fix-zaehlen", "then: slot"),
        ),
        (
            "fix counter does not count",
            t.replace(
                "$context.fixrunden.runde // 0) + 1",
                "$context.fixrunden.runde // 0) + 0",
            ),
        ),
        (
            "limit ignores the flow counter",
            t.replace(
                "$context.fixrunden.runde // 0) >= 3",
                "$context.fixrunden.runde // 0) >= 99",
            ),
        ),
        (
            "limit ignores the guard",
            t.replace("$context.waechter.red == true) or", "false) or"),
        ),
        (
            "limit ignores the exit code",
            t.replace("$context.gezielt.rc // 0) != 0", "0) != 0"),
        ),
    ];
    for (name, mutated) in cases {
        assert!(!guard_loop_is_closed(&mutated), "{name} was not caught");
    }
}
