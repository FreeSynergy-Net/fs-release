//! Pattern: none (test). Chain gate `check-pin-chain`: an `fs-release` owner that is spelled differently or is foreign is red
//! (GitHub resolves `freesynergy-net/...` too, the chain follows only the exact spellings: before, such a pin stayed old and green).

use std::fs;
use std::path::Path;
use std::process::Command;

use fs_release::pin_chain::check;

fn git(d: &Path, args: &[&str]) -> String {
    let o = Command::new("git")
        .arg("-C")
        .arg(d)
        .args([
            "-c",
            "commit.gpgsign=false",
            "-c",
            "user.name=t",
            "-c",
            "user.email=t@t.t",
        ])
        .args(args)
        .output()
        .unwrap();
    assert!(o.status.success(), "git {args:?}");
    String::from_utf8_lossy(&o.stdout).trim().to_string()
}

fn write(d: &Path, rel: &str, text: &str) {
    let p = d.join(rel);
    fs::create_dir_all(p.parent().unwrap()).unwrap();
    fs::write(p, text).unwrap();
}

/// A repo whose start head has one correct self-pin (`FreeSynergy-Net`, to an old commit with the SAME action code) and a second `uses:` with `owner`
/// pinned to an OLD action code. Returns the red findings.
fn findings(owner: &str) -> Vec<String> {
    findings_line(&format!("- uses: {owner}/fs-release/actions/pub@{{OLD}}"))
}

/// Same, with the second step line given verbatim (`{OLD}` is replaced by the old commit).
fn findings_line(second: &str) -> Vec<String> {
    let t = tempfile::tempdir().unwrap();
    let d = t.path();
    git(d, &["init", "-q", "-b", "main"]);
    write(d, "actions/pub/pub.sh", "publish v1");
    git(d, &["add", "-A"]);
    git(d, &["commit", "-q", "-m", "old"]);
    let old = git(d, &["rev-parse", "HEAD"]);
    write(d, "actions/pub/pub.sh", "publish v2");
    let wf = format!(
        "jobs:\n  a:\n    steps:\n      - uses: FreeSynergy-Net/fs-release/actions/pub@{old}\n      {}\n", second.replace("{OLD}", &old)
    );
    write(d, ".github/workflows/x.yml", &wf);
    git(d, &["add", "-A"]);
    git(d, &["commit", "-q", "-m", "start"]);
    check(d, "HEAD").unwrap()
}

#[test]
fn a_lowercase_owner_pinned_to_old_code_is_red() {
    let found = findings("freesynergy-net");
    assert!(
        found.iter().any(|f| f.contains("unknown owner spelling")),
        "{found:?}"
    );
}

#[test]
fn a_foreign_owner_is_red() {
    let found = findings("Evil");
    assert!(
        found.iter().any(|f| f.contains("unknown owner spelling")),
        "{found:?}"
    );
}

#[test]
fn the_exact_owners_are_not_flagged_as_spelling() {
    for o in ["FreeSynergy-Net", "FreeSynergy"] {
        let found = findings(o);
        assert!(
            !found.iter().any(|f| f.contains("unknown owner spelling")),
            "{found:?}"
        );
        assert!(
            found
                .iter()
                .any(|f| f.contains("differs from the start head")),
            "{found:?}"
        );
    }
}

fn is_red_spelling(second: &str) -> bool {
    findings_line(second)
        .iter()
        .any(|f| f.contains("unknown owner spelling"))
}

/// M5: GitHub resolves the repo name without regard to case, too.
#[test]
fn a_differently_cased_repo_name_pinned_to_old_code_is_red() {
    for name in ["FS-Release", "Fs-release", "FS-RELEASE"] {
        assert!(
            is_red_spelling(&format!(
                "- uses: FreeSynergy-Net/{name}/actions/pub@{{OLD}}"
            )),
            "{name}"
        );
    }
}

/// S1: a `uses:` in a form the parser does not read is red, never silently green.
#[test]
fn unread_uses_forms_are_red() {
    for line in [
        "- {uses: FreeSynergy-Net/fs-release/actions/pub@{OLD}}",
        "- uses : FreeSynergy-Net/fs-release/actions/pub@{OLD}",
        "- \"uses\": FreeSynergy-Net/fs-release/actions/pub@{OLD}",
        "- uses: FreeSynergy-Net/fs-release/actions/pub",
    ] {
        assert!(is_red_spelling(line), "{line}");
    }
}

/// S3: the flow forms behind a key (`steps: [ {uses: ...} ]`, `job: {uses: ...}`) are red too, one test per form.
#[test]
fn flow_sequence_form_is_red() {
    for line in [
        "steps: [ {uses: FreeSynergy-Net/fs-release/actions/pub@{OLD}} ]",
        "steps: [{ \"uses\": \"FreeSynergy-Net/fs-release/actions/pub@{OLD}\" }]",
        "steps: [ {name: x, uses: FreeSynergy-Net/fs-release/actions/pub@{OLD}} ]",
    ] {
        assert!(is_red_spelling(line), "{line}");
    }
}

#[test]
fn flow_mapping_form_behind_a_job_key_is_red() {
    for line in [
        "b: {uses: FreeSynergy-Net/fs-release/.github/workflows/x.yml@{OLD}}",
        "b: { 'uses': FreeSynergy-Net/fs-release/.github/workflows/x.yml@{OLD} }",
    ] {
        assert!(is_red_spelling(line), "{line}");
    }
}

/// A `run:` text that says "uses" and `fs-release/` in prose is no flow form.
#[test]
fn prose_in_a_run_line_is_not_a_flow_form() {
    assert!(!is_red_spelling(
        "run: echo done, uses fs-release/actions/pub and {braces}"
    ));
}

/// S4: the parser call in `foreign_uses` is needed: `- - uses:` (a sequence in a sequence) is read by the parser but is no form `unread_form` sees.
#[test]
fn a_nested_sequence_uses_with_a_foreign_owner_is_red() {
    assert!(is_red_spelling(
        "- - uses: Evil/fs-release/actions/pub@{OLD}"
    ));
}
