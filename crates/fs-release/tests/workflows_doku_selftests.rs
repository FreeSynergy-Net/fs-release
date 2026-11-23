//! Pattern: none (test). Self tests of the scripts around the release flows (doku gate, rename, background test, checklists, pin refs, library release,
//! signing environment, pattern kill, mkdocs quiet) and the rule [PROZESSE] in the flows.

mod support;
use std::process::Command;
use support::workflows::{read, root, run_self_test, step_names};

#[test]
fn the_doku_gate_script_passes_its_self_test() {
    run_self_test("test_doku_mitziehen.sh");
}

#[test]
fn released_changelog_sections_equal_their_tags() {
    run_self_test("test_changelog_released.sh");
}

#[test]
fn internal_pins_are_one_sha_and_no_main() {
    run_self_test("test_pins_consistent.sh");
}

#[test]
fn the_topmost_changelog_section_is_unreleased_or_the_crate_version() {
    run_self_test("test_changelog_top.sh");
}

#[test]
fn the_rename_script_passes_its_self_test() {
    run_self_test("test_umbenennen.sh");
}

#[test]
fn the_background_test_script_passes_its_self_test() {
    run_self_test("test_workspace_test_background.sh");
}

#[test]
fn the_checklist_reader_passes_its_self_test() {
    run_self_test("test_read_checklists.sh");
}

#[test]
fn pin_refs_passes_its_self_test() {
    run_self_test("test_pin_refs.sh");
}

#[test]
fn library_release_passes_its_self_test() {
    run_self_test("test_library_release.sh");
}

#[test]
fn tag_on_main_passes_its_self_test() {
    run_self_test("test_tag_on_main.sh");
}

#[test]
fn the_signing_environment_check_passes_its_self_test() {
    run_self_test("test_check_signing_env.sh");
}

#[test]
fn the_pattern_kill_check_passes_its_self_test() {
    run_self_test("test_check_no_pkill.sh");
}

/// The rule [PROZESSE] (no `pkill` / `killall`, only the own PID of the own build-slot.sh process) stands in `testlauf`, `agent-run` and
/// `koordinator-runde`, and `testlauf` has the check step; the real tree has no pattern kill.
#[test]
fn the_process_rule_is_in_the_flows_and_the_tree_is_clean() {
    for f in ["testlauf", "agent-run", "koordinator-runde"] {
        let t = read(f);
        assert!(
            t.contains("pkill") && t.contains("killall") && t.contains("PID"),
            "{f}: rule missing"
        );
    }
    let t = read("testlauf");
    assert!(step_names(&t).first().is_some_and(|n| n == "prozess-regel"));
    assert!(t.contains("scripts/check-no-pkill") && t.contains("[PROZESSE]"));
    let out = Command::new("python3")
        .arg("-I")
        .arg(root().join("scripts/check-no-pkill"))
        .output()
        .expect("python3");
    assert!(
        out.status.success(),
        "{}",
        String::from_utf8_lossy(&out.stdout)
    );
}

/// B1: the CHANGELOG test is a real gate in the OWN CI (`lint.yml`, full clone with tags) and the reusable `ci.yml` of the calling repos keeps its flat clone.
fn changelog_gate_is_wired(lint: &str, ci: &str) -> bool {
    let job = lint
        .split("\n  changelog-released:")
        .nth(1)
        .map_or("", |t| t.split("\n  packaging-tests:").next().unwrap_or(""));
    job.contains("fetch-depth: 0")
        && job.contains("fetch-tags: true")
        && job.contains("bash scripts/test_changelog_released.sh")
        && !ci.contains("fetch-depth")
        && !ci.contains("fetch-tags")
}

#[test]
fn the_changelog_test_runs_in_the_own_ci_and_not_in_the_reusable_ci() {
    let dir = root().join(".github/workflows");
    let lint = std::fs::read_to_string(dir.join("lint.yml")).unwrap();
    let ci = std::fs::read_to_string(dir.join("ci.yml")).unwrap();
    assert!(changelog_gate_is_wired(&lint, &ci));
    assert!(!changelog_gate_is_wired(
        &lint.replace("fetch-depth: 0", "fetch-depth: 1"),
        &ci
    ));
    assert!(!changelog_gate_is_wired(
        &lint.replace("fetch-tags: true", ""),
        &ci
    ));
    assert!(!changelog_gate_is_wired(
        &lint.replace("bash scripts/test_changelog_released.sh", "true"),
        &ci
    ));
    assert!(!changelog_gate_is_wired(
        &lint,
        &format!("{ci}\n          fetch-depth: 0\n")
    ));
    assert!(!changelog_gate_is_wired(
        &lint,
        &format!("{ci}\n          fetch-tags: true\n")
    ));
}

/// The own branch CI runs `cargo test`, which includes the CHANGELOG test: its checkout needs the full history with tags (a flat clone is red).
#[test]
fn the_branch_ci_checks_out_the_full_history_with_tags() {
    let ci = std::fs::read_to_string(root().join(".github/workflows/branch-ci.yml")).unwrap();
    assert!(
        ci.contains("fetch-depth: 0"),
        "branch-ci.yml needs fetch-depth: 0 (tags + history for the CHANGELOG test)"
    );
}
