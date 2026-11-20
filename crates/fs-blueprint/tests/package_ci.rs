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
fn a_package_without_cargo_toml_gets_the_blueprint_check_and_no_cargo_gate() {
    for (path, code_job) in [
        (".github/workflows/check.yml", "code"),
        (".github/workflows/windows.yml", "windows"),
    ] {
        let text = workflow(path);
        let package = job(&text, "package");
        assert!(
            package.contains("needs.kind.outputs.code != 'true'"),
            "{path}"
        );
        assert!(package.contains("fs-blueprint\" check"), "{path}");
        assert!(package.contains("--drift"), "{path}");
        // the reusable workflow of fs-release runs cargo fmt/clippy/test: not in the package job
        assert!(!package.contains("uses: FreeSynergy/fs-release"), "{path}");
        let code = job(&text, code_job);
        assert!(code.contains("needs.kind.outputs.code == 'true'"), "{path}");
        assert!(
            code.contains("uses: FreeSynergy/fs-release/.github/workflows/"),
            "{path}"
        );
    }
}

#[test]
fn the_guard_looks_for_the_cargo_toml_in_the_root_of_the_repo() {
    for path in [
        ".github/workflows/check.yml",
        ".github/workflows/windows.yml",
    ] {
        let text = workflow(path);
        let kind = job(&text, "kind");
        assert!(kind.contains("[ -f Cargo.toml ]"), "{path}");
        assert!(
            kind.contains("code: ${{ steps.look.outputs.code }}"),
            "{path}"
        );
    }
}

#[test]
fn the_package_job_installs_the_command_from_the_pinned_tag() {
    let text = workflow(".github/workflows/check.yml");
    let package = job(&text, "package");
    assert!(
        package.contains(
            "cargo install --git https://github.com/FreeSynergy/fs-release --tag v0.0.13 --locked"
        ),
        "{package}"
    );
}
