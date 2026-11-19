//! Pattern: none (test). `fs-release lift` for the V1+ lift: pins never lower, the pin tag is an input, identity and footer of the commit,
//! libraries by name (`--repo NAME=TAG`, fs-types moves to its own repo).

mod support;

use fs_release::lift::{self, Report};
use fs_release_chain::sys::SystemRunner;
use std::fs;
use support::{git, World};

fn step<'a>(r: &'a Report, name: &str) -> &'a fs_release::task::Record {
    r.steps
        .iter()
        .find(|s| s.name == name)
        .unwrap_or_else(|| panic!("no step {name}"))
}

fn run(w: &World, o: &lift::LiftOpts) -> (Report, i32) {
    let _ = w;
    lift::run(o, &SystemRunner::default())
}

/// A repo with workflows on `@main`, an old tag and a newer tag, committed in the world's work tree.
fn world_with_workflows() -> World {
    let w = World::new();
    let wf = w.work().join(".github/workflows");
    fs::create_dir_all(&wf).unwrap();
    fs::write(
        wf.join("check.yml"),
        "jobs:\n  c:\n    uses: FreeSynergy/fs-release/.github/workflows/check.yml@v0.0.9\n",
    )
    .unwrap();
    fs::write(wf.join("release.yml"), "jobs:\n  r:\n    uses: FreeSynergy/fs-release/.github/workflows/release.yml@v0.0.15\n  s:\n    uses: FreeSynergy/fs-release/actions/sbom@main\n").unwrap();
    git(&w.work(), &["add", "-A"]);
    git(&w.work(), &["commit", "--quiet", "-m", "workflows"]);
    w
}

#[test]
fn pins_never_lowers_a_line_that_is_on_a_higher_tag() {
    let w = world_with_workflows();
    let (r, _) = run(&w, &w.opts());
    assert_eq!(step(&r, "pins").status, "ok", "{}", r.markdown());
    let check = w.read(".github/workflows/check.yml");
    let release = w.read(".github/workflows/release.yml");
    assert!(check.contains("check.yml@v0.0.15\n"), "{check}");
    assert!(release.contains("release.yml@v0.0.15\n"), "{release}");
    assert!(release.contains("actions/sbom@v0.0.15\n"), "{release}");
}

#[test]
fn the_pin_tag_is_an_input_not_a_constant() {
    let w = world_with_workflows();
    let mut o = w.opts();
    o.env.push(("FS_LIFT_PIN_TAG".into(), "v0.0.15".into()));
    let (r, _) = run(&w, &o);
    assert_eq!(step(&r, "pins").status, "ok", "{}", r.markdown());
    let all = format!(
        "{}{}",
        w.read(".github/workflows/check.yml"),
        w.read(".github/workflows/release.yml")
    );
    assert_eq!(all.matches("@v0.0.15\n").count(), 3, "{all}");
    // Not a tag: ignored, the default is the version of this crate (the release tag).
    let w = world_with_workflows();
    let mut o = w.opts();
    o.env.push(("FS_LIFT_PIN_TAG".into(), "main".into()));
    let (r, _) = run(&w, &o);
    assert_eq!(step(&r, "pins").status, "ok", "{}", r.markdown());
    let want = format!("check.yml@v{}\n", env!("CARGO_PKG_VERSION"));
    assert!(w.read(".github/workflows/check.yml").contains(&want));
}

/// Identity and message of HEAD: `author | committer | whole message`.
fn head_ident(w: &World) -> (String, String) {
    (
        git(&w.work(), &["log", "-1", "--format=%an <%ae> | %cn <%ce>"]),
        git(&w.work(), &["log", "-1", "--format=%B"]),
    )
}

const HOUSE: &str = "FreeSynergy <support@freesynergy.net> | FreeSynergy <support@freesynergy.net>";

#[test]
fn a_lift_commit_has_the_house_identity_and_the_footer_even_against_an_identity_in_the_environment()
{
    let w = World::new();
    let mut o = w.opts(); // the test environment carries GIT_AUTHOR_NAME=t and friends: they must not win
    o.commit = true;
    let (r, code) = run(&w, &o);
    assert_eq!(code, 0, "{}", r.markdown());
    let (who, msg) = head_ident(&w);
    assert_eq!(who, HOUSE);
    assert!(
        msg.starts_with("lift to fs-components v0.2.0, fs-interfaces v0.2.0\n"),
        "{msg}"
    );
    assert!(
        msg.trim_end()
            .ends_with("\n\nCo-Authored-By: Claude Sonnet 5.5 <noreply@anthropic.com>"),
        "{msg}"
    );
}

#[test]
fn a_lift_commit_has_the_house_identity_without_any_identity_in_the_environment_or_the_git_config()
{
    let w = World::new();
    let mut o = w.opts();
    o.env
        .retain(|(k, _)| !k.starts_with("GIT_AUTHOR") && !k.starts_with("GIT_COMMITTER"));
    o.env.push(("GIT_CONFIG_GLOBAL".into(), "/dev/null".into()));
    o.env.push(("GIT_CONFIG_NOSYSTEM".into(), "1".into()));
    o.commit = true;
    let (r, code) = run(&w, &o);
    assert_eq!(code, 0, "{}", r.markdown());
    assert_eq!(head_ident(&w).0, HOUSE);
}

#[test]
fn the_footer_value_is_configurable_and_an_invalid_one_stops_before_anything_is_committed() {
    let w = World::new();
    let mut o = w.opts();
    o.commit = true;
    o.env.push((
        "FS_LIFT_COAUTHOR".into(),
        "Other Bot <bot@example.org>".into(),
    ));
    let (r, code) = run(&w, &o);
    assert_eq!(code, 0, "{}", r.markdown());
    assert!(head_ident(&w)
        .1
        .contains("\nCo-Authored-By: Other Bot <bot@example.org>"));
    for bad in [
        "",
        "nobody",
        "Name <nomail>",
        "<a@b>",
        "N <a@b>\nSigned-off-by: x",
    ] {
        let w = World::new();
        let before = git(&w.work(), &["rev-parse", "HEAD"]);
        let mut o = w.opts();
        o.commit = true;
        o.env.push(("FS_LIFT_COAUTHOR".into(), bad.into()));
        let (r, code) = run(&w, &o);
        assert_eq!(code, 1, "{bad:?}: {}", r.markdown());
        let c = step(&r, "commit");
        assert_eq!(c.status, "red", "{bad:?}");
        assert!(
            c.first_error.clone().unwrap().contains("FS_LIFT_COAUTHOR"),
            "{bad:?}"
        );
        assert_eq!(git(&w.work(), &["rev-parse", "HEAD"]), before, "{bad:?}");
    }
}

#[test]
fn the_fs_interfaces_lift_alone_leaves_fs_types_where_it_is() {
    let w = World::new();
    w.with_types_over_interfaces();
    let (r, code) = run(&w, &w.opts()); // components v0.2.0, interfaces v0.2.0, no fs-types asked
    assert_eq!(code, 0, "{}", r.markdown());
    let a = w.read("crates/a/Cargo.toml");
    assert!(
        a.contains("fs-types = { git = \"https://github.com/FreeSynergy/fs-interfaces\", tag = \"v0.1.0\" }"),
        "{a}"
    );
    assert!(a.contains("fs-interfaces = { git = \"https://github.com/FreeSynergy/fs-interfaces\", tag = \"v0.2.0\" }"), "{a}");
}

#[test]
fn asking_for_fs_types_moves_it_to_its_own_repo_and_the_lock_holds_exactly_that_source() {
    let w = World::new();
    w.with_types_over_interfaces();
    let mut o = w.opts();
    o.set_lib_arg("fs-types=v0.2.0").unwrap();
    let (r, code) = run(&w, &o);
    assert_eq!(code, 0, "{}", r.markdown());
    let a = w.read("crates/a/Cargo.toml");
    assert!(
        a.contains(
            "fs-types = { git = \"https://github.com/FreeSynergy/fs-types\", tag = \"v0.2.0\" }"
        ),
        "{a}"
    );
    assert!(a.contains("fs-interfaces = { git = \"https://github.com/FreeSynergy/fs-interfaces\", tag = \"v0.2.0\" }"), "{a}");
    let lock = w.read("Cargo.lock");
    assert!(
        lock.contains("github.com/FreeSynergy/fs-types?tag=v0.2.0#"),
        "{lock}"
    );
    assert!(!lock.contains("fs-interfaces?tag=v0.1.0"), "{lock}");
    assert!(
        step(&r, "lock").detail.contains("fs-types v0.2.0"),
        "{}",
        r.markdown()
    );
    assert_eq!(step(&r, "lock-sources").status, "ok", "{}", r.markdown());
    assert!(r.markdown().contains("fs-types v0.2.0"));
    let json: serde_json::Value =
        serde_json::from_str(&w.read(".fs-release/lift-report.json")).unwrap();
    assert_eq!(json["repos"]["fs-types"], "v0.2.0");
}

#[test]
fn a_fs_types_tag_that_does_not_exist_is_red_at_the_lock() {
    let w = World::new();
    w.with_types_over_interfaces();
    let mut o = w.opts();
    o.set_lib_arg("fs-types=v9.9.9").unwrap();
    let (r, code) = run(&w, &o);
    assert_eq!(code, 1);
    assert_eq!(step(&r, "deps").status, "ok");
    assert_eq!(step(&r, "lock").status, "red");
    assert_eq!(step(&r, "tests").status, "skipped");
}

#[test]
fn a_program_without_fs_types_and_a_fs_types_request_alone_is_red_at_deps() {
    let w = World::new();
    let mut o = w.opts();
    o.components = None;
    o.interfaces = None;
    o.set_lib_arg("fs-types=v0.2.0").unwrap();
    let (r, code) = run(&w, &o);
    assert_eq!(code, 1);
    assert_eq!(step(&r, "deps").status, "red", "{}", r.markdown());
}

#[test]
fn the_command_line_takes_repo_name_tag_and_refuses_unknown_names_and_doubles() {
    let w = World::new();
    let go = |extra: &[&str]| {
        let mut v: Vec<String> = ["fs-release", "lift"].map(String::from).to_vec();
        v.push(w.work().display().to_string());
        v.extend(extra.iter().map(|s| (*s).to_string()));
        fs_release::cli::main_with(v)
    };
    assert_eq!(go(&["--repo", "fs-wiki=v0.0.1"]), 2);
    assert_eq!(go(&["--repo", "fs-types"]), 2);
    assert_eq!(go(&["--types", "v0.0.1", "--repo", "fs-types=v0.0.2"]), 2);
    assert_eq!(
        go(&["--interfaces", "v0.0.1", "--repo", "fs-interfaces=v0.0.2"]),
        2
    );
    assert_eq!(git(&w.work(), &["status", "--porcelain"]), "");
}
