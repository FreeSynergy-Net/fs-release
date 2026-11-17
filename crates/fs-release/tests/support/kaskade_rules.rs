//! Test support for `pin-kaskade`: the lock env inputs, the rev-pin edges, the foreign sources, the build rules and the Windows caller (B1-B4).

use super::kaskade::{list_lines, REV_PINS};
use super::workflows::step_body;

/// B1 (M11): the `env:` INPUT of `lock-strikt` and `lock-ende` carries both switches; the words in the summary text alone do not count.
pub const ENV_LINE: &str = "        env: FS_LOCK_STRICT=1 FS_LOCK_ONLY_FROM=1\n";

/// B2-B4: the rev-pin edges and the foreign sources in the header, the build rules and the no-exception Windows caller step.
fn rules_ok(t: &str) -> bool {
    let head = t.split("document:").next().unwrap_or("");
    let pins = list_lines(head, "# REV-PIN: ");
    REV_PINS
        .iter()
        .all(|p| pins.iter().any(|l| l.starts_with(p)))
        && pins.len() == REV_PINS.len()
        && head.lines().any(|l| {
            l.starts_with("# FREMDQUELLEN: workflows/kaskade-fremdquellen.txt")
                && l.contains("FreeSynergy/blinc-app-fix")
                && l.contains("FreeSynergy/blinc-layout-fix")
        })
        && step_body(t, "glied-start").contains(".fs-lock-sources.toml")
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
        && step_body(t, "windows-aufrufer").contains("        aufrufer: FreeSynergy/fs-release/.github/workflows/_windows-check.yml@v0.0.11\n")
        && step_body(t, "windows-aufrufer").contains("NO exceptions")
        && step_body(t, "windows-weiter").contains("then: eskalation")
        && step_body(t, "windows-aufrufer").contains("examples/callers/windows.yml")
        && step_body(t, "zweig-ci").contains("`windows`")
}

/// Both lock steps carry both switches in their INPUT, and the B2-B4 rules hold.
pub fn lock_rules_ok(t: &str) -> bool {
    step_body(t, "lock-strikt").contains(ENV_LINE)
        && step_body(t, "lock-ende").contains(ENV_LINE)
        && rules_ok(t)
}
