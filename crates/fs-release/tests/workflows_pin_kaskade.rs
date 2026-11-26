//! Pattern: none (test). `pin-kaskade` (specification): ONE cascade after a new fs-types tag, Glied by Glied in DAG order. Per Glied: the existing `lift`,
//! strict lock check, gates (`CARGO_BUILD_JOBS=4`, `BUILD_PRIORITY=0`), branch CI, Opus review, `ff-und-tag` as the only way to main and tag, THEN the next Glied.
//! A red Glied aborts (no skipping, no repeat), resume with `ab`; the end checks one fs-types version in every lock. Counter-tests on changed copies.

mod support;
use support::kaskade::{cases, graph_ok, switches_ok, with_then, SWITCHES};
use support::kaskade_rules::lock_rules_ok;
use support::workflows::{in_order, read, step_body};

fn ok(t: &str) -> bool {
    let to = |s: &str, target: &str| {
        let b = step_body(t, s);
        b.contains(&format!("then: {target}"))
    };
    in_order(
        t,
        &[
            "vorhanden-pruefen",
            "karte",
            "lage",
            "plan",
            "glied-weiche",
            "glied-start",
            "rev-pins",
            "rev-pins-weiter",
            "nachbar-pins",
            "nachbar-pins-weiter",
            "pin-lift",
            "ein-stand",
            "ein-stand-weiter",
            "besonderes",
            "windows-aufrufer",
            "windows-weiter",
            "lock-strikt",
            "gates",
            "gates-weiter",
            "push-zweig",
            "zweig-ci",
            "ci-weiter",
            "review",
            "review-weiter",
            "ff-tag",
            "ff-weiter",
            "naechstes",
            "lock-ende",
            "lock-ende-weiter",
            "bericht",
            "eskalation",
        ],
    ) && step_body(t, "pin-lift").contains("name: lift\n")
        && step_body(t, "ff-tag").contains("name: ff-und-tag\n")
        && !t.contains("name: release-flow")
        && !t.contains("tag-annotated")
        && step_body(t, "gates").contains("BUILD_PRIORITY=0")
        && step_body(t, "gates").contains("CARGO_BUILD_JOBS=4")
        && step_body(t, "gates").contains("[GATE-PIPEFAIL]")
        && t.contains("[KEINE-KALTEN-FLACHPROBEN-BIS-AUFRAEUMEN]")
        && step_body(t, "lock-strikt").contains("strict = true")
        && step_body(t, "gates-weiter").contains("ABORT")
        && step_body(t, "gates-weiter").contains("then: eskalation")
        && !step_body(t, "gates-weiter").contains("then: pin-lift")
        && to("naechstes", "glied-weiche")
        && step_body(t, "review").contains("gate: opus-ja")
        && step_body(t, "eskalation").contains("actor: human")
        && step_body(t, "eskalation").contains("`ab`")
        && step_body(t, "lock-ende").contains("SAME fs-types version")
        && step_body(t, "lock-ende").contains("`fs-conformance`")
        && step_body(t, "rev-pins").contains("`rev` pins")
        && step_body(t, "rev-pins").contains("fs-people-directory")
        && step_body(t, "rev-pins").contains("fs-conformance")
        && step_body(t, "rev-pins-weiter").contains("then: eskalation")
        && !step_body(t, "rev-pins-weiter").contains("then: pin-lift")
        && step_body(t, "ein-stand").contains("DEV-DEPENDENCIES FIRST")
        && step_body(t, "ein-stand").contains("cargo tree -d")
        && step_body(t, "ein-stand").contains("exactly ONE state")
        && step_body(t, "ein-stand").contains("cargo tree -d --workspace --target all")
        && step_body(t, "nachbar-pins").contains("[DEV-DEPS-ZUERST]")
        && step_body(t, "nachbar-pins-weiter").contains("then: eskalation")
        && !step_body(t, "nachbar-pins-weiter").contains("then: pin-lift")
        && step_body(t, "windows-aufrufer").contains("[PIN-REFS-DIR]")
        && step_body(t, "windows-aufrufer").contains("[PIN-INNEN-BIS-RELEASE]")
        && step_body(t, "windows-aufrufer").contains("pin-refs.sh --dir")
        && [
            "[DEV-DEPS-ZUERST]",
            "[PIN-REFS-DIR]",
            "[PIN-INNEN-BIS-RELEASE]",
        ]
        .iter()
        .all(|r| t.contains(r))
        && step_body(t, "ein-stand-weiter").contains("then: eskalation")
        && !step_body(t, "ein-stand-weiter").contains("then: besonderes")
        && step_body(t, "plan").contains("REAL dependencies")
        && graph_ok(t)
        && switches_ok(t)
        && step_body(t, "lock-strikt").contains("FS_LOCK_ONLY_FROM=1")
        && step_body(t, "lock-strikt").contains("FS_LOCK_STRICT=1")
        && step_body(t, "lock-ende").contains("FS_LOCK_ONLY_FROM=1")
        && lock_rules_ok(t)
        && !t.contains("one source per fs crate")
        && step_body(t, "review").contains("delta: ")
        && step_body(t, "review").contains("tests: ")
        && step_body(t, "push-zweig").contains("branch-prefix: ")
        && !t.contains("shared target")
        && [
            "[EIGENES-TARGET]",
            "[GATE-VOLLSTAENDIG-LAUFEN]",
            "[GATE-CI-UMGEBUNG]",
            "[GATE-FLACHER-KLON-OHNE-TAGS]",
            "[GATE-AUSGABE-NIE-KUERZEN]",
        ]
        .iter()
        .all(|r| step_body(t, "gates").contains(r))
        && step_body(t, "eskalation").contains("ab: ${ if $context.lockende != null then null")
        && step_body(t, "glied-weiche").contains("plan.offen_vorher")
        && step_body(t, "naechstes").contains("plan.offen_vorher")
        && step_body(t, "lage").contains("erledigt")
        && t.contains(".input[\"ausloeser-tag\"]")
        && t.contains(".input.glieder")
        && t.contains(".input.ab")
}

#[test]
fn pin_kaskade_follows_the_order_and_calls_the_existing_flows() {
    assert!(ok(&read("pin-kaskade")));
}

#[test]
fn counter_own_tag_way_skipped_abort_or_changed_order_are_caught() {
    let t = read("pin-kaskade");
    let bad = |x: String| x != t && !std::panic::catch_unwind(|| ok(&x)).unwrap_or(false);
    assert!(bad(t.replace("name: ff-und-tag\n", "name: release-flow\n")));
    assert!(bad(t.replace("name: lift\n", "name: lift-v2\n")));
    assert!(bad(t.replace("BUILD_PRIORITY=0", "BUILD_PRIORITY=5")));
    assert!(bad(t.replace("strict = true", "strict = false")));
    assert!(bad(t.replace("then: glied-weiche", "then: lock-ende")));
    assert!(bad(t.replace("  - lock-strikt:", "  - lock-x:")));
    assert!(bad(t.replacen("gate: opus-ja", "gate: none", 1)));
    assert!(bad(format!("{t}\n# tag-annotated\n")));
    // graph: jscalendar dropped, fs-rpc before fs-objects, fs-tasks before fs-people, a changed edge, the rev-pin step skipped
    let order = "# GLIEDER-ORDER: fs-interfaces, fs-objects, fs-jscalendar, fs-rpc, fs-components, fs-people, fs-tasks, fs-calendars";
    assert!(t.contains(order));
    assert!(bad(t.replace(order, "# GLIEDER-ORDER: fs-interfaces, fs-objects, fs-rpc, fs-components, fs-people, fs-tasks, fs-calendars")));
    assert!(bad(t.replace(order, "# GLIEDER-ORDER: fs-interfaces, fs-rpc, fs-objects, fs-jscalendar, fs-components, fs-people, fs-tasks, fs-calendars")));
    assert!(bad(t.replace(order, "# GLIEDER-ORDER: fs-interfaces, fs-objects, fs-jscalendar, fs-rpc, fs-components, fs-tasks, fs-people, fs-calendars")));
    assert!(bad(t.replace(
        "# DAG: fs-tasks <- fs-interfaces, fs-objects, fs-components, fs-jscalendar, fs-people",
        "# DAG: fs-tasks <- fs-interfaces, fs-objects, fs-components, fs-jscalendar"
    )));
    assert!(bad(t.replace(
        "# DAG: fs-rpc <- fs-types, fs-interfaces, fs-objects, fs-conformance",
        "# DAG: fs-rpc <- fs-types, fs-interfaces, fs-objects"
    )));
    assert!(bad(t.replace("  - rev-pins-weiter:", "  - rev-x:")));
    assert!(bad(t.replace("  - rev-pins:", "  - rev-x:")));
    assert!(bad(t.replace("  - ein-stand:", "  - ein-x:")));
    assert!(bad(t.replace("  - ein-stand-weiter:", "  - ein-x:")));
    assert!(bad(t.replace("  - nachbar-pins:", "  - nachbar-x:")));
    assert!(bad(t.replace("  - nachbar-pins-weiter:", "  - nachbar-x:")));
    assert!(bad(t.replace(
        "cargo tree -d --workspace --target all",
        "cargo tree -d"
    )));
    assert!(bad(t.replace("pin-refs.sh --dir", "pin-refs.sh")));
    assert!(bad(t.replace("[PIN-INNEN-BIS-RELEASE]", "[X]")));
    assert!(bad(
        t.replace("DEV-DEPENDENCIES FIRST", "DEV-DEPENDENCIES LATER")
    ));
    assert!(bad(t.replace("cargo tree -d", "cargo tree")));
    // A2: every case of every switch, one by one (M4 gates-weiter red -> continue, M5 round case, M7 ff-weiter `sonst`)
    for (step, want) in SWITCHES {
        for (case, then) in *want {
            let new = if *then == "eskalation" {
                "naechstes"
            } else {
                "eskalation"
            };
            assert!(
                bad(with_then(&t, step, case, new)),
                "{step}/{case} -> {new} must be caught"
            );
        }
        let first = cases(&t, step).remove(0).0;
        let cut = t.replacen(
            &format!("        - {first}:\n"),
            "        - umbenannt:\n",
            1,
        );
        assert!(
            bad(cut),
            "{step}: first case renamed/removed must be caught"
        );
    }
    assert!(bad(t.replacen(">= 3", ">= 4", 1)));
    assert!(bad(t.replace(
        "lockcheck.red == false and",
        "lockcheck.red != true and"
    )));
    assert!(bad(t.replace("\"JA\"", "\"NEIN\"")));
    // A4-A8
    assert!(bad(t.replace("FS_LOCK_ONLY_FROM=1", "")));

    assert!(bad(t.replace(
        "          delta: ${ $context.zweig.basis }..${ $context.zweig.head }\n",
        ""
    )));
    assert!(bad(t.replace("branch-prefix: ", "branch: ")));
    assert!(bad(t.replace(
        "OWN target of the worktree",
        "shared target of the worktree"
    )));
    assert!(bad(t.replace(
        "        ab: ${ if $context.lockende != null then null else",
        "        ab: ${ if false then null else"
    )));
    assert!(bad(t.replace("plan.offen_vorher", "plan.x")));
}

#[test]
fn the_cascade_calls_each_other_flow_only_by_run_and_passes_the_inputs() {
    let t = read("pin-kaskade");
    let ff = step_body(&t, "ff-tag");
    for k in ["repo", "branch", "head", "tag", "allow", "workdir"] {
        assert!(
            ff.contains(&format!("          {k}: ")),
            "ff-und-tag input {k}"
        );
    }
    let lift = step_body(&t, "pin-lift");
    for k in ["repo", "commit", "libs"] {
        assert!(lift.contains(&format!("          {k}: ")), "lift input {k}");
    }
}

/// The jq of a `when: '${ ... }'` line, evaluated on `ctx` (the `$context` of the run): the real answer, not the wording.
fn when_is(t: &str, case: &str, ctx: &str) -> bool {
    use std::io::Write;
    let step = "ein-stand-weiter";
    let expr = cases(t, step)
        .into_iter()
        .find(|c| c.0 == case)
        .map(|c| c.1)
        .expect("case exists");
    let expr = expr
        .trim()
        .trim_start_matches('\'')
        .trim_end_matches('\'')
        .trim_start_matches("${")
        .trim_end_matches('}')
        .to_string();
    let mut child = std::process::Command::new("jq")
        .args(["-e", "--argjson", "context", ctx, &expr])
        .stdin(std::process::Stdio::piped())
        .stdout(std::process::Stdio::null())
        .spawn()
        .expect("jq is installed (it is on the runner and in the dev tools)");
    child.stdin.take().unwrap().write_all(b"null").unwrap();
    child.wait().unwrap().success()
}

#[test]
fn ein_stand_weiter_reaches_the_green_way_and_stops_on_red_or_no_result() {
    let t = read("pin-kaskade");
    let green = r#"{"einstand":{"red":false,"befunde":[]}}"#;
    let red = r#"{"einstand":{"red":true,"befunde":[]}}"#;
    let found = r#"{"einstand":{"red":false,"befunde":[{"crate":"fs-rpc"}]}}"#;
    let none = r"{}";
    // green: only `gruen` fires
    assert!(!when_is(&t, "rot", green) && when_is(&t, "gruen", green));
    // red, or a finding: `rot`, never `gruen`
    assert!(when_is(&t, "rot", red) && !when_is(&t, "gruen", red));
    assert!(when_is(&t, "rot", found) && !when_is(&t, "gruen", found));
    // no result: neither fires, so `ohne-ergebnis` (-> eskalation) decides
    assert!(!when_is(&t, "rot", none) && !when_is(&t, "gruen", none));
}
