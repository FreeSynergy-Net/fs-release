//! The empty program the generator makes from ONE blueprint (S3): which files, what they hold, what stays out.
mod support;
use fs_blueprint::{Blueprint, Class};
use support::synth::{blueprint, COUNT};
use support::{edit, generate, FakeCatalog};

const PROBE: &str = include_str!("data/probe/scaffold.toml");

fn probe() -> Vec<fs_blueprint::Artifact> {
    generate(&Blueprint::from_toml(PROBE).unwrap(), &FakeCatalog).unwrap()
}

fn get<'a>(a: &'a [fs_blueprint::Artifact], path: &str) -> &'a str {
    &a.iter()
        .find(|x| x.path == path)
        .unwrap_or_else(|| panic!("no {path}"))
        .content
}

#[test]
fn the_files_of_the_empty_program_are_these_with_these_classes() {
    let got: Vec<(String, Class)> = probe().into_iter().map(|a| (a.path, a.class)).collect();
    let g = Class::Generated;
    let want = [
        (".gitattributes", g),
        (".github/workflows/check.yml", g),
        (".github/workflows/windows.yml", g),
        ("packaging/frontends/fs-probe-simple.metainfo.xml", g),
        ("packaging/layout/fs-probe-layout.toml", g),
        ("packaging/net.freesynergy.Probe.desktop", g),
    ];
    assert_eq!(got, want.map(|(p, c)| (p.to_string(), c)));
}

#[test]
fn every_toml_file_is_toml_and_no_crate_manifest_is_made() {
    let a = probe();
    for f in a.iter().filter(|f| {
        std::path::Path::new(&f.path)
            .extension()
            .is_some_and(|x| x == "toml")
    }) {
        let t: toml::Table =
            toml::from_str(&f.content).unwrap_or_else(|e| panic!("{}: {e}", f.path));
        assert!(!t.is_empty());
    }
    // a manifest would point to Rust source that is not there (rule code-in-program)
    assert!(a.iter().all(|f| !f.path.ends_with("Cargo.toml")));
}

#[test]
fn the_pins_come_from_the_skeleton_table() {
    let a = probe();
    for p in [
        ".github/workflows/check.yml",
        ".github/workflows/windows.yml",
    ] {
        assert!(get(&a, p).contains("@v0.0.13"), "{p}");
    }
}

#[test]
fn an_unknown_skeleton_version_is_refused_by_generate() {
    let bp =
        Blueprint::from_toml(&edit(PROBE, "skeleton = \"0.1.0\"", "skeleton = \"9.9.9\"")).unwrap();
    assert_eq!(
        generate(&bp, &FakeCatalog).unwrap_err().to_string(),
        "skeleton-unknown{found=9.9.9}"
    );
}

#[test]
fn the_generated_caller_runs_the_blueprint_check_in_mode_warn_and_never_block() {
    // block would turn red for every program that gets a hand-written file (file-unowned: 274 in fs-tasks, 161 in fs-people when checked); warn shows it and stays green
    let a = probe();
    let check = get(&a, ".github/workflows/check.yml");
    assert!(
        check.ends_with("    with:\n      blueprint: warn\n"),
        "{check}"
    );
    assert!(!check.contains("blueprint: block") && !check.contains("blueprint-rev"));
}

#[test]
fn the_workflow_expressions_stay_as_they_are_and_no_slot_is_left_anywhere() {
    let a = probe();
    assert!(
        get(&a, ".github/workflows/check.yml").contains("${{ github.workflow }}-${{ github.ref }}")
    );
    assert!(get(&a, ".github/workflows/windows.yml")
        .contains("${{ github.event_name != 'workflow_dispatch' || inputs.cache }}"));
    for f in &a {
        for line in f.content.lines().filter(|l| !l.contains("${{")) {
            assert!(
                !line.contains("{{") && !line.contains("}}"),
                "{}: {line}",
                f.path
            );
        }
    }
}

#[test]
fn no_rust_source_is_generated_a_program_is_data() {
    for i in 0..COUNT {
        let bp = blueprint(i);
        for a in generate(&bp, &FakeCatalog).unwrap() {
            assert!(
                std::path::Path::new(&a.path)
                    .extension()
                    .is_none_or(|x| x != "rs"),
                "{}: {}",
                bp.program.name,
                a.path
            );
        }
    }
    let a = probe();
    assert!(a.iter().all(|x| !x.path.contains("/src/")));
}

#[test]
fn the_desktop_file_has_the_name_the_summary_in_both_languages_and_the_categories_on_one_line() {
    let a = probe();
    let d = get(&a, "packaging/net.freesynergy.Probe.desktop");
    assert!(
        d.contains("Name=Probe\n")
            && d.contains("Exec=fs-probe\n")
            && d.contains("Categories=Utility;\n")
    );
    assert!(d.contains("Comment=A probe program without any function\n"));
    assert!(d.contains("Comment[de]=Ein Probe-Programm ohne jede Funktion\n"));
    let odd = edit(
        PROBE,
        "A probe program without any function",
        "two\\nlines\\tand   spaces",
    );
    let bp = Blueprint::from_toml(&odd).unwrap();
    let out = generate(&bp, &FakeCatalog).unwrap();
    assert!(get(&out, "packaging/net.freesynergy.Probe.desktop")
        .contains("Comment=two lines and spaces\n"));
}

#[test]
fn the_synthetic_blueprints_all_generate_the_same_number_of_files_under_their_own_names() {
    for i in 0..COUNT {
        let bp = blueprint(i);
        let out = generate(&bp, &FakeCatalog).unwrap();
        // the five files of the program and the contract text with the interface of `provides`
        assert_eq!(out.len(), 7, "{}", bp.program.name);
        let name = &bp.program.name;
        assert!(out.iter().any(|a| a.path.contains(&format!("fs-{name}"))));
    }
}
