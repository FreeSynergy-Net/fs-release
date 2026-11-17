//! Pattern: none (test). `pin-kaskade` (specification): ONE cascade after a new fs-types tag, Glied by Glied in DAG order. Per Glied: the existing `lift`,
//! strict lock check, gates (`CARGO_BUILD_JOBS=4`, `BUILD_PRIORITY=0`), branch CI, Opus review, `ff-und-tag` as the only way to main and tag, THEN the next Glied.
//! A red Glied aborts (no skipping, no repeat), resume with `ab`; the end checks one fs-types version in every lock. Counter-tests on changed copies.

mod support;
use support::kaskade::{
    cases, graph_ok, list_lines, switches_ok, with_then, REV_PINS, SCRIPTS, SWITCHES,
};
use support::workflows::{in_order, read, step_body};

/// B1 (M11): the `env:` INPUT of `lock-strikt` and `lock-ende` carries both switches; the words in the summary text alone do not count.
const ENV_LINE: &str = "        env: FS_LOCK_STRICT=1 FS_LOCK_ONLY_FROM=1\n";

/// B2-B4: the rev-pin edges and the foreign sources in the header, the build rules and the no-exception Windows caller step.
fn rules_ok(t: &str) -> bool {
    let head = t.split("document:").next().unwrap_or("");
    let pins = list_lines(head, "# REV-PIN: ");
    REV_PINS
        .iter()
        .all(|p| pins.iter().any(|l| l.starts_with(p)))
        && pins.len() == REV_PINS.len()
        && head.contains("# FREMDQUELLEN: workflows/kaskade-fremdquellen.txt")
        && head.contains("FreeSynergy/blinc-app-fix")
        && head.contains("FreeSynergy/blinc-layout-fix")
        && step_body(t, "rev-pins").contains("9950b51")
        && step_body(t, "rev-pins").contains("blinc-app-fix")
        && [
            "[BAU-ANMELDEN]",
            "nach-welle-1.md",
            "max. 2 build slots systemweit",
            "[GATES-WAECHTER]",
            "[GATE-NICHT-AENDERN]",
            "[WORKTREES-NIE-IM-HAUPT-CHECKOUT]",
        ]
        .iter()
        .all(|r| head.contains(r))
        && ["[BAU-ANMELDEN]", "[GATES-WAECHTER]", "[GATE-NICHT-AENDERN]"]
            .iter()
            .all(|r| step_body(t, "gates").contains(r))
        && head.contains("AFTER fs-release v0.0.11")
        && step_body(t, "lage").contains("fs-release-tag: v0.0.11")
        && step_body(t, "windows-aufrufer").contains("_windows-check.yml@v0.0.11")
        && step_body(t, "windows-aufrufer").contains("NO exceptions")
        && step_body(t, "windows-aufrufer").contains("examples/callers/windows.yml")
        && step_body(t, "zweig-ci").contains("`windows`")
}

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
            "pin-lift",
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
        && step_body(t, "plan").contains("REAL dependencies")
        && graph_ok(t)
        && switches_ok(t)
        && step_body(t, "lock-strikt").contains(ENV_LINE)
        && step_body(t, "lock-ende").contains(ENV_LINE)
        && step_body(t, "lock-strikt").contains("FS_LOCK_ONLY_FROM=1")
        && step_body(t, "lock-strikt").contains("FS_LOCK_STRICT=1")
        && step_body(t, "lock-ende").contains("FS_LOCK_ONLY_FROM=1")
        && rules_ok(t)
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
    // B1: M11 - the switch removed from the INPUT of ONE step only (the summary text still names it): lock-strikt, then lock-ende
    let weak = "        env: FS_LOCK_STRICT=1\n";
    assert!(ENV_LINE != weak && t.matches(ENV_LINE).count() == 2);
    assert!(
        bad(t.replacen(ENV_LINE, weak, 1)),
        "M11: lock-strikt without FS_LOCK_ONLY_FROM=1"
    );
    let at = t.rfind(ENV_LINE).unwrap();
    assert!(
        bad(format!("{}{}{}", &t[..at], weak, &t[at + ENV_LINE.len()..])),
        "M11b: lock-ende without FS_LOCK_ONLY_FROM=1"
    );
    assert!(
        bad(t.replacen(ENV_LINE, "        env: FS_LOCK_ONLY_FROM=1\n", 1)),
        "M11c: lock-strikt without FS_LOCK_STRICT=1"
    );
    // B2-B4: the rev-pin edge of fs-calendars, the foreign sources, the build rules, the Windows caller and its switch
    assert!(bad(t.replace(
        "# REV-PIN: fs-calendars -> fs-components rev 9950b51",
        "# REV-PIN: fs-calendars -> fs-components rev 0000000"
    )));
    assert!(bad(t.replace(
        "FreeSynergy/blinc-layout-fix (blinc_layout)",
        "(blinc_layout)"
    )));
    for rule in [
        "[BAU-ANMELDEN]",
        "max. 2 build slots systemweit",
        "[GATES-WAECHTER]",
        "[GATE-NICHT-AENDERN]",
        "[WORKTREES-NIE-IM-HAUPT-CHECKOUT]",
    ] {
        assert!(bad(t.replace(rule, "[X]")), "rule {rule} must be caught");
    }
    assert!(bad(t.replace("  - windows-aufrufer:", "  - windows-x:")));
    assert!(bad(t.replace("  - windows-weiter:", "  - windows-x:")));
    assert!(bad(t.replace(
        "_windows-check.yml@v0.0.11\n        vorlage",
        "_windows-check.yml@main\n        vorlage"
    )));
    assert!(bad(t.replace("        fs-release-tag: v0.0.11\n", "")));
    assert!(bad(t.replace("The workflow `windows`", "The workflow `x`")));
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

#[test]
fn the_seven_scripts_exist_are_called_by_the_workflow_and_are_executable() {
    use std::os::unix::fs::PermissionsExt;
    let t = read("pin-kaskade");
    for name in SCRIPTS {
        let path = support::workflows::root().join("scripts").join(name);
        let mode = std::fs::metadata(&path)
            .unwrap_or_else(|_| panic!("scripts/{name} is missing"))
            .permissions()
            .mode();
        assert!(mode & 0o111 != 0, "scripts/{name} is not executable");
        assert!(
            t.contains(&format!("call: {name}:0.0.10@fs")),
            "the workflow does not call {name}"
        );
        assert!(
            t.contains(&format!("`scripts/{name}`")),
            "the summary does not name scripts/{name}"
        );
    }
    let list = std::fs::read_to_string(
        support::workflows::root().join("workflows/kaskade-fremdquellen.txt"),
    )
    .unwrap();
    for r in ["FreeSynergy/blinc-app-fix", "FreeSynergy/blinc-layout-fix"] {
        assert!(
            list.lines().any(|l| l.starts_with(r)),
            "{r} must be a known foreign source"
        );
    }
}

#[test]
fn the_shell_self_tests_of_the_scripts_are_green() {
    for t in [
        "test_kaskade_plan.sh",
        "test_kaskade_pins.sh",
        "test_kaskade_gates.sh",
    ] {
        let out = std::process::Command::new("bash")
            .arg(support::workflows::root().join("scripts").join(t))
            .output()
            .unwrap();
        assert!(
            out.status.success(),
            "{t}: {}",
            String::from_utf8_lossy(&out.stdout)
        );
    }
}
