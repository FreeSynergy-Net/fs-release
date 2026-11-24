//! Test support for the workflow files in `workflows/`: the list of files, reading, step bodies, and mutated copies.

use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

use super::{have_yaml, python};

pub const FILES: [&str; 64] = [
    "adr-write",
    "adresse-wechseln",
    "agent-run",
    "anbieter-neuinstallation",
    "ausnahme-gespraech",
    "bahnen",
    "bausatz-neu",
    "branch-neu-schreiben",
    "branches-aufraeumen",
    "bugfix",
    "catalog-bump",
    "cleanup-targets",
    "coordination-round",
    "cost-reconcile",
    "doku-aufraeumen",
    "doku-mitziehen",
    "dossier",
    "download-readonly",
    "entscheidung",
    "feature-liste",
    "ff-und-tag",
    "form-neu",
    "frontend-beschreiben",
    "frontend-round",
    "frontend-umbau",
    "geruest-ausbau",
    "geruest-heben",
    "geruest-lift",
    "grundfassung",
    "handoff-update",
    "icon-round",
    "katalog",
    "koordinator-runde",
    "lib-umzug",
    "lift",
    "lift-v1plus",
    "menschen-aufnahme",
    "modul-neu",
    "picture-acceptance",
    "pin-kaskade",
    "programm-grundgeruest",
    "programm-neu",
    "pruefung",
    "publish-doc",
    "release-flow",
    "release-umgebung-pruefen",
    "report-to-strategy",
    "repos-abgleichen",
    "restart-handoff",
    "review",
    "server-einrichtung",
    "slot-check",
    "stil-neu",
    "stoerung-diagnose",
    "testlauf",
    "trait-delta",
    "umbenennen",
    "upstream-anfrage",
    "upstream-update",
    "upstream-watch",
    "user-decision",
    "wip-continue",
    "workflow-pflege",
    "wrapper-einbindung",
];

pub fn root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../..")
}

pub fn script() -> PathBuf {
    root().join("scripts/check_workflows.py")
}

pub fn read(name: &str) -> String {
    fs::read_to_string(root().join("workflows").join(format!("{name}.yaml"))).unwrap()
}

/// The step names of a file (`  - name:` lines of the `do` list; the files keep that layout).
pub fn step_names(text: &str) -> Vec<String> {
    text.lines()
        .filter_map(|l| l.strip_prefix("  - ")?.strip_suffix(':'))
        .map(str::to_string)
        .collect()
}

/// The text of one step of a file (from its `  - name:` line to the next step).
pub fn step_body(text: &str, step: &str) -> String {
    let body = text.split(&format!("  - {step}:\n")).nth(1).unwrap();
    body.split("\n  - ").next().unwrap().to_string()
}

/// The run rule set `workflows/run/rules.yaml` (the ONE place of the states of a run).
pub fn rules() -> String {
    fs::read_to_string(root().join("workflows/run/rules.yaml")).unwrap()
}

/// A copy of `workflows/` in a temp folder with `file` changed by `edit` and the rule set changed by `edit_rules`; returns exit code and
/// output of the check.
pub fn mutated_both(
    file: &str,
    edit: impl Fn(String) -> String,
    edit_rules: impl Fn(String) -> String,
) -> (i32, String) {
    let tmp = tempfile::tempdir().unwrap();
    for f in FILES {
        let text = read(f);
        let text = if f == file { edit(text) } else { text };
        fs::write(tmp.path().join(format!("{f}.yaml")), text).unwrap();
    }
    fs::create_dir(tmp.path().join("run")).unwrap();
    fs::write(tmp.path().join("run/rules.yaml"), edit_rules(rules())).unwrap();
    python(&script(), &[tmp.path()])
}

/// A copy of `workflows/` in a temp folder with `file` changed by `edit`; returns exit code and output of the check.
pub fn mutated(file: &str, edit: impl Fn(String) -> String) -> (i32, String) {
    mutated_both(file, edit, |r| r)
}

/// The rule set changed by `edit`, the flows as they are.
pub fn assert_rules_red(needle: &str, edit: impl Fn(String) -> String) {
    let (code, out) = mutated_both("", |t| t, edit);
    check_red(code, &out, needle);
}

/// The flows as they are, but without the rule set file.
pub fn assert_red_without_rules(needle: &str) {
    let tmp = tempfile::tempdir().unwrap();
    for f in FILES {
        fs::write(tmp.path().join(format!("{f}.yaml")), read(f)).unwrap();
    }
    let (code, out) = python(&script(), &[tmp.path()]);
    check_red(code, &out, needle);
}

/// One state of a run checked against the real rule set (`check_run_rules.py <rules> <state> [reason]`).
pub fn run_state(state: &str, reason: Option<&str>) -> (i32, String) {
    let rules = root().join("workflows/run/rules.yaml");
    let state = PathBuf::from(state);
    let reason = reason.map(PathBuf::from);
    let mut args: Vec<&Path> = vec![&rules, &state];
    if let Some(r) = &reason {
        args.push(r);
    }
    python(&root().join("scripts/check_run_rules.py"), &args)
}

/// A change that must stay green (a valid use of the profile).
pub fn assert_green(file: &str, edit: impl Fn(String) -> String) {
    let (code, out) = mutated(file, edit);
    assert_eq!(code, if have_yaml() { 0 } else { 2 }, "{out}");
}

fn check_red(code: i32, out: &str, needle: &str) {
    if have_yaml() {
        assert_eq!(code, 1, "{out}");
        assert!(out.contains(needle), "wanted {needle:?} in:\n{out}");
    } else {
        assert_eq!(code, 2, "{out}");
    }
}

/// A mutation must turn the check red and say `needle`; without `PyYAML` the check says `skipped` (exit 2) instead.
pub fn assert_red(file: &str, needle: &str, edit: impl Fn(String) -> String) {
    let (code, out) = mutated(file, edit);
    check_red(code, &out, needle);
}

pub fn pos(text: &str, step: &str) -> usize {
    step_names(text)
        .iter()
        .position(|n| n == step)
        .unwrap_or_else(|| panic!("no step {step}"))
}

pub fn in_order(text: &str, order: &[&str]) -> bool {
    order.windows(2).all(|w| pos(text, w[0]) < pos(text, w[1]))
}

pub fn run_self_test(name: &str) {
    // no silent skip: without python3 the self test cannot run, and a test that did not run is not green
    assert!(
        Command::new("python3").arg("--version").output().is_ok(),
        "python3 is missing: the self test {name} was NOT run"
    );
    let out = Command::new("bash")
        .arg(root().join("scripts").join(name))
        .output()
        .expect("bash");
    let text = String::from_utf8_lossy(&out.stdout);
    assert!(
        out.status.success() && text.contains("all ok"),
        "{text}\n{}",
        String::from_utf8_lossy(&out.stderr)
    );
}
