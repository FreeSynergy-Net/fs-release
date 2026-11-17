//! Pattern: none (test). The `yaml-check` step of the lift and `scripts/check_yaml.py`: a workflow file that is not YAML is red
//! with `file:line`, a valid one is green, a missing `PyYAML` is `skipped` (never green).

mod support;

use std::fs;
use std::path::{Path, PathBuf};

use fs_release::lift;
use fs_release::task::{run_all, Task, SKIPPED};
use fs_release_chain::outcome::Stop;
use support::{have_yaml, python, World};

const GOOD: &str = "name: ci\non: push\njobs:\n  a:\n    runs-on: ubuntu-latest\n    steps:\n      - name: Build (debug, the test x)\n        run: echo hi\n";
/// The finding: an unquoted `: ` inside a step name (line 7).
const BAD: &str = "name: ci\non: push\njobs:\n  a:\n    runs-on: ubuntu-latest\n    steps:\n      - name: Build (debug, the test: x)\n        run: echo hi\n";

fn script() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../../scripts/check_yaml.py")
}

fn put_workflow(root: &Path, text: &str) {
    let dir = root.join(".github/workflows");
    fs::create_dir_all(&dir).unwrap();
    fs::write(dir.join("ci.yml"), text).unwrap();
}

fn step<'a>(r: &'a lift::Report, name: &str) -> &'a fs_release::task::Record {
    r.steps.iter().find(|s| s.name == name).unwrap()
}

#[test]
fn script_green_on_a_valid_file_and_red_with_file_and_line_on_the_broken_one() {
    let tmp = tempfile::tempdir().unwrap();
    put_workflow(tmp.path(), GOOD);
    let (code, out) = python(&script(), &[tmp.path()]);
    if !have_yaml() {
        assert_eq!(code, 2, "{out}");
        return;
    }
    assert_eq!(code, 0, "{out}");
    put_workflow(tmp.path(), BAD);
    let (code, out) = python(&script(), &[tmp.path()]);
    assert_eq!(code, 1, "{out}");
    assert!(out.starts_with(".github/workflows/ci.yml:7:"), "{out}");
}

#[test]
fn script_without_the_yaml_module_is_skipped_with_a_warning_not_green() {
    let tmp = tempfile::tempdir().unwrap();
    put_workflow(tmp.path(), BAD);
    let s = script();
    let arg = Path::new("--simulate-missing-yaml");
    let (code, out) = python(&s, &[tmp.path(), arg]);
    assert_eq!(code, 2, "{out}");
    assert!(
        out.starts_with("skipped:") && out.contains("NOT checked"),
        "{out}"
    );
}

#[test]
fn a_repo_without_workflow_files_is_green() {
    let tmp = tempfile::tempdir().unwrap();
    let (code, out) = python(&script(), &[tmp.path()]);
    assert_eq!(code, if have_yaml() { 0 } else { 2 }, "{out}");
}

#[test]
fn lift_is_red_at_yaml_check_for_the_broken_file_and_green_for_the_valid_one() {
    let w = World::new();
    put_workflow(&w.work(), GOOD);
    let (r, code) = lift::run(&w.opts(), &fs_release_chain::sys::SystemRunner::default());
    assert_eq!(code, 0, "{}", r.markdown());
    let s = step(&r, "yaml-check");
    assert_eq!(
        s.status,
        if have_yaml() { "ok" } else { "skipped" },
        "{s:?}"
    );
    assert_eq!(r.steps.len(), 11);
    if !have_yaml() {
        return;
    }
    put_workflow(&w.work(), BAD);
    let mut o = w.opts();
    o.commit = true;
    let (r, code) = lift::run(&o, &fs_release_chain::sys::SystemRunner::default());
    assert_eq!(code, 1, "{}", r.markdown());
    let s = step(&r, "yaml-check");
    assert_eq!(s.status, "red");
    assert!(
        s.first_error
            .clone()
            .unwrap()
            .starts_with(".github/workflows/ci.yml:7:"),
        "{s:?}"
    );
    assert_eq!(step(&r, "tests").status, "ok");
    assert_eq!(step(&r, "commit").status, "skipped");
}

struct Unable;

impl Task<()> for Unable {
    fn name(&self) -> &'static str {
        "unable"
    }

    fn run(&self, (): &()) -> Result<String, Stop> {
        Ok(format!("{SKIPPED}no tool"))
    }
}

#[test]
fn a_step_that_returns_the_skipped_marker_is_recorded_skipped_and_does_not_stop_the_run() {
    let list: Vec<Box<dyn Task<()>>> = vec![Box::new(Unable)];
    let (records, first) = run_all(&list, &());
    assert!(first.is_none());
    assert_eq!(records[0].status, "skipped");
    assert_eq!(records[0].detail, "no tool");
    assert!(!records[0].is_ok());
}

fn repo_root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../..")
}

/// All five groups the script must read: composite actions, `FreeSynergy` flows, GitHub workflows, chains, caller examples.
fn yaml_files(root: &Path) -> Vec<PathBuf> {
    let mut v = Vec::new();
    let ext = |p: &Path| matches!(p.extension().and_then(|e| e.to_str()), Some("yml" | "yaml"));
    for e in fs::read_dir(root.join("actions")).unwrap().flatten() {
        let a = e.path().join("action.yml");
        if a.is_file() {
            v.push(a);
        }
    }
    for dir in [
        "workflows",
        ".github/workflows",
        "chains",
        "examples/callers",
    ] {
        for e in fs::read_dir(root.join(dir)).unwrap().flatten() {
            if ext(&e.path()) {
                v.push(e.path());
            }
        }
    }
    v
}

#[test]
fn every_action_flow_and_github_workflow_file_of_this_repo_parses_and_the_script_reads_all_five_groups(
) {
    let root = repo_root();
    let files = yaml_files(&root);
    assert!(files.len() > 50, "{}", files.len());
    let (code, out) = python(&script(), &[&root]);
    if !have_yaml() {
        assert_eq!(code, 2, "{out}");
        return;
    }
    assert_eq!(code, 0, "{out}");
    // the script counts exactly the files found here: none of the five groups is left out
    assert_eq!(out, format!("{} YAML file(s) parse", files.len()), "{out}");
}

/// The finding of run 7: an unquoted `: ` in the description of a composite action (line 2).
const BAD_ACTION: &str =
    "name: x\ndescription: Release of a pure library: check the tag\ninputs: {}\n";

#[test]
fn a_broken_action_yml_and_a_broken_flow_yaml_are_red_with_file_and_line() {
    if !have_yaml() {
        return;
    }
    let tmp = tempfile::tempdir().unwrap();
    let a = tmp.path().join("actions/library-release");
    fs::create_dir_all(&a).unwrap();
    fs::write(a.join("action.yml"), BAD_ACTION).unwrap();
    let (code, out) = python(&script(), &[tmp.path()]);
    assert_eq!(code, 1, "{out}");
    assert!(
        out.starts_with("actions/library-release/action.yml:2:"),
        "{out}"
    );
    fs::write(
        a.join("action.yml"),
        "name: x\ndescription: \"ok: quoted\"\n",
    )
    .unwrap();
    let (code, out) = python(&script(), &[tmp.path()]);
    assert_eq!(code, 0, "{out}");
    let w = tmp.path().join("workflows");
    fs::create_dir_all(&w).unwrap();
    fs::write(
        w.join("flow.yaml"),
        "name: f\nsteps:\n  - name: a\n    note: b: c\n",
    )
    .unwrap();
    let (code, out) = python(&script(), &[tmp.path()]);
    assert_eq!(code, 1, "{out}");
    assert!(out.starts_with("workflows/flow.yaml:4:"), "{out}");
}

#[test]
fn the_real_library_release_action_with_the_old_unquoted_line_is_red() {
    if !have_yaml() {
        return;
    }
    let real = fs::read_to_string(repo_root().join("actions/library-release/action.yml")).unwrap();
    let (first, rest) = real.split_once('\n').unwrap();
    let (desc, body) = rest.split_once('\n').unwrap();
    let unquoted = desc.replacen("description: \"", "description: ", 1);
    let unquoted = unquoted.strip_suffix('"').unwrap();
    assert_ne!(unquoted, desc, "the description line must be quoted");
    let tmp = tempfile::tempdir().unwrap();
    let a = tmp.path().join("actions/library-release");
    fs::create_dir_all(&a).unwrap();
    fs::write(a.join("action.yml"), format!("{first}\n{unquoted}\n{body}")).unwrap();
    let (code, out) = python(&script(), &[tmp.path()]);
    assert_eq!(code, 1, "{out}");
    assert!(
        out.starts_with("actions/library-release/action.yml:2:"),
        "{out}"
    );
}

/// Mutant: the script with one group left out counts fewer files than the test finds (red), for each of the two new groups.
#[test]
fn a_script_that_leaves_out_the_chains_or_the_caller_examples_is_caught_by_the_count() {
    if !have_yaml() {
        return;
    }
    let root = repo_root();
    let want = format!("{} YAML file(s) parse", yaml_files(&root).len());
    let src = fs::read_to_string(script()).unwrap();
    let tmp = tempfile::tempdir().unwrap();
    for (name, drop) in [
        ("no_chains", "(\"chains\", \"*.yaml\"),"),
        ("no_callers", "(\"examples\", \"callers\", \"*.yml\"),"),
    ] {
        assert!(src.contains(drop), "{drop}");
        let m = tmp.path().join(format!("{name}.py"));
        fs::write(&m, src.replacen(drop, "", 1)).unwrap();
        let (code, out) = python(&m, &[&root]);
        assert_eq!(code, 0, "{out}");
        assert_ne!(out, want, "{name}: the count must differ");
    }
}

#[test]
fn a_broken_chain_and_a_broken_caller_example_are_red_with_file_and_line() {
    if !have_yaml() {
        return;
    }
    let tmp = tempfile::tempdir().unwrap();
    for (dir, file) in [("chains", "x.chain.yaml"), ("examples/callers", "ci.yml")] {
        let d = tmp.path().join(dir);
        fs::create_dir_all(&d).unwrap();
        fs::write(d.join(file), "name: x\nnote: a: b\n").unwrap();
        let (code, out) = python(&script(), &[tmp.path()]);
        assert_eq!(code, 1, "{out}");
        assert!(out.starts_with(&format!("{dir}/{file}:2:")), "{out}");
        fs::remove_file(d.join(file)).unwrap();
    }
}
