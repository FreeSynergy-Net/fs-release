//! The CI of a generated data package: without a `Cargo.toml` the blueprint check (`--drift`) is the CI, with one the gates of fs-release stay.
mod support;
use fs_blueprint::Blueprint;
use support::{generate, FakeCatalog};

const PROBE: &str = include_str!("data/probe/scaffold.toml");

fn workflow(path: &str) -> String {
    let bp = Blueprint::from_toml(PROBE).unwrap();
    generate(&bp, &FakeCatalog)
        .unwrap()
        .into_iter()
        .find(|a| a.path == path)
        .unwrap_or_else(|| panic!("no {path}"))
        .content
}

/// The text of one job of the workflow (from its name up to the next job at the same indent).
fn job<'a>(text: &'a str, name: &str) -> &'a str {
    let start = text
        .find(&format!("\n  {name}:\n"))
        .unwrap_or_else(|| panic!("no job {name}"))
        + 1;
    let first = text[start..]
        .find('\n')
        .map_or(text.len(), |i| start + i + 1);
    let mut end = text.len();
    let mut at = first;
    for line in text[first..].split_inclusive('\n') {
        if line.starts_with("  ") && !line.starts_with("   ") && line.trim_end().ends_with(':') {
            end = at;
            break;
        }
        at += line.len();
    }
    &text[start..end]
}

#[test]
fn a_package_without_cargo_toml_gets_the_blueprint_check_and_no_cargo_gate_in_the_windows_caller() {
    let text = workflow(".github/workflows/windows.yml");
    let kind = job(&text, "kind");
    assert!(kind.contains("[ -f Cargo.toml ]"));
    let package = job(&text, "package");
    assert!(package.contains("needs.kind.outputs.code != 'true'"));
    assert!(package.contains("fs-blueprint\" check") && package.contains("--drift"));
    // the reusable workflow of fs-release runs cargo fmt/clippy/test: not in the package job
    assert!(!package.contains("uses: FreeSynergy/fs-release"));
    let code = job(&text, "windows");
    assert!(code.contains("needs.kind.outputs.code == 'true'"));
}

/// ONE caller for both kinds: the reusable `check.yml` of fs-release has the data mode (release-inputs, plan probe, `--drift`), so the generated caller installs nothing by itself.
#[test]
fn the_check_caller_is_one_call_of_the_reusable_workflow_for_code_and_data_alike() {
    let text = workflow(".github/workflows/check.yml");
    let tag = fs_blueprint::emit::skeleton::RELEASE_TAG;
    assert!(
        text.contains(&format!(
            "uses: FreeSynergy-Net/fs-release/.github/workflows/check.yml@{tag}\n"
        )),
        "{text}"
    );
    assert!(
        !text.contains("cargo install") && !text.contains("kind:"),
        "{text}"
    );
    assert!(
        !text.contains("FreeSynergy/fs-release"),
        "old owner: {text}"
    );
}

#[test]
fn the_windows_package_job_installs_the_command_from_the_pinned_tag() {
    let text = workflow(".github/workflows/windows.yml");
    let package = job(&text, "package");
    let tag = fs_blueprint::emit::skeleton::RELEASE_TAG;
    assert!(
        package.contains(&format!(
            "cargo install --git https://github.com/FreeSynergy/fs-release --tag {tag} --locked"
        )),
        "{package}"
    );
}

/// Every tag the package CI pins (`--tag X`, `@X` of a fs-release workflow) is the constant of the generator, none a literal of its own.
#[test]
fn no_tag_in_the_package_ci_differs_from_the_release_tag_constant() {
    let tag = fs_blueprint::emit::skeleton::RELEASE_TAG;
    for path in [
        ".github/workflows/check.yml",
        ".github/workflows/windows.yml",
    ] {
        let text = workflow(path);
        let mut seen = 0;
        for line in text.lines().filter(|l| l.contains("/fs-release")) {
            let pinned = if let Some((_, rest)) = line.split_once("--tag ") {
                rest.split_whitespace().next()
            } else {
                line.rsplit_once('@').map(|(_, t)| t.trim())
            };
            assert_eq!(pinned, Some(tag), "{path}: {line}");
            seen += 1;
        }
        assert!(seen >= 1, "{path}: only {seen} pins found");
    }
}
