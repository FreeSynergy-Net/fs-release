//! Pattern: none (test). `pin-kaskade`, conditions B1-B4: the lock switches in the INPUT of both lock steps (mutant M11), the rev-pin edges and foreign sources, the build rules, the
//! Windows caller without exception; the seven scripts exist and their shell self tests are green. Counter-tests on changed copies.

mod support;
use support::kaskade::SCRIPTS;
use support::kaskade_rules::{lock_rules_ok, ENV_LINE};
use support::workflows::{read, root};

#[test]
fn counter_lock_env_foreign_sources_build_rules_and_windows_caller_are_caught() {
    let t = read("pin-kaskade");
    let bad =
        |x: String| x != t && !std::panic::catch_unwind(|| lock_rules_ok(&x)).unwrap_or(false);
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
}

#[test]
fn the_seven_scripts_exist_are_called_by_the_workflow_and_are_executable() {
    use std::os::unix::fs::PermissionsExt;
    let t = read("pin-kaskade");
    for name in SCRIPTS {
        let path = root().join("scripts").join(name);
        let mode = std::fs::metadata(&path)
            .unwrap_or_else(|_| panic!("scripts/{name} is missing"))
            .permissions()
            .mode();
        assert!(mode & 0o111 != 0, "scripts/{name} is not executable");
        assert!(
            t.contains(&format!("call: {name}:0.0.17@fs")),
            "the workflow does not call {name}"
        );
        assert!(
            t.contains(&format!("`scripts/{name}`")),
            "the summary does not name scripts/{name}"
        );
    }
    let list = std::fs::read_to_string(root().join("workflows/kaskade-fremdquellen.txt")).unwrap();
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
            .arg(root().join("scripts").join(t))
            .output()
            .unwrap();
        assert!(
            out.status.success(),
            "{t}: {}",
            String::from_utf8_lossy(&out.stdout)
        );
    }
}
