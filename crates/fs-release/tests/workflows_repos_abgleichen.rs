//! Pattern: none (test). The flow `repos-abgleichen` and its script `scripts/repos-abgleichen`: the fast-forward step is `merge --ff-only`
//! with `gate: none` (only there), set-url only for repos on main, no `--prune`, and no reset/checkout/stash/pull/push in the code of the script
//! (grep counter-check). Comment lines are not code. Each rule has a counter-test on a changed copy.

mod support;
use std::fs;
use support::workflows::{read, root, step_body};

/// The code lines of the script (no comment lines).
fn code(script: &str) -> Vec<&str> {
    script
        .lines()
        .filter(|l| !l.trim_start().starts_with('#'))
        .collect()
}

fn script_is_sound(script: &str, yaml: &str) -> bool {
    let lines = code(script);
    let joined = lines.join("\n");
    let ff = step_body(yaml, "fast-forward");
    // the fast-forward step: merge --ff-only, gate none, machine
    let ff_ok = ff.contains("gate: none")
        && ff.contains("actor: machine")
        && ff.contains("merge --ff-only")
        && joined.contains("merge --ff-only origin/main")
        // no merge without --ff-only
        && !lines
            .iter()
            .any(|l| l.contains("merge ") && !l.contains("--ff-only") && l.contains("git "));
    // no --prune in the code
    let no_prune = !joined.contains("--prune");
    // set-url only inside the branch test `main`, and before the fetch
    let set_url = joined.find("remote set-url");
    let main_test = joined.find("\"$branch\" = \"main\"");
    let fetch = joined.find("fetch --all");
    let set_url_on_main = match (set_url, main_test, fetch) {
        (Some(s), Some(m), Some(f)) => m < s && s < f,
        _ => false,
    };
    // no reset, checkout, stash, pull, push, clean or ref deletion as git commands
    let forbidden = [
        "reset",
        "checkout",
        "stash",
        "pull",
        "push",
        "clean",
        "update-ref",
        "switch",
    ];
    let no_forbidden = !lines
        .iter()
        .any(|l| l.contains("git") && l.split_whitespace().any(|t| forbidden.contains(&t)));
    ff_ok && no_prune && set_url_on_main && no_forbidden
}

#[test]
fn repos_abgleichen_merges_ff_only_never_prunes_and_switches_to_https_only_on_main() {
    let script = fs::read_to_string(root().join("scripts/repos-abgleichen")).unwrap();
    assert!(script_is_sound(&script, &read("repos-abgleichen")));
}

#[test]
fn counter_a_prune_a_plain_merge_or_a_forbidden_git_command_is_caught() {
    let script = fs::read_to_string(root().join("scripts/repos-abgleichen")).unwrap();
    let yaml = read("repos-abgleichen");
    assert!(!script_is_sound(
        &script.replace("fetch --all --tags", "fetch --all --tags --prune"),
        &yaml
    ));
    assert!(!script_is_sound(
        &script.replace("merge --ff-only origin/main", "merge origin/main"),
        &yaml
    ));
    assert!(!script_is_sound(
        &script.replace("merge --ff-only origin/main", "reset --hard origin/main"),
        &yaml
    ));
    assert!(!script_is_sound(
        &script,
        &yaml.replace("gate: none", "gate: human")
    ));
}

#[test]
fn counter_a_set_url_outside_the_main_branch_is_caught() {
    let script = fs::read_to_string(root().join("scripts/repos-abgleichen")).unwrap();
    let yaml = read("repos-abgleichen");
    // the branch test before set-url is removed: set-url would run for every repo
    let unguarded = script.replace("if [ \"$branch\" = \"main\" ]; then", "if true; then");
    assert!(!script_is_sound(&unguarded, &yaml));
}
