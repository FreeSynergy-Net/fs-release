//! `[frontend]` (ADR 0099 point 12): the generator writes the frontend manifest and the package `layout`, never a frontend crate.
mod support;
use fs_blueprint::emit::header::untouched;
use fs_blueprint::{Artifact, Blueprint, Class};
use support::synth::{blueprint, COUNT};
use support::{edit, generate, FakeCatalog, TASKS};

fn out(src: &str) -> Vec<Artifact> {
    generate(&Blueprint::from_toml(src).unwrap(), &FakeCatalog).unwrap()
}

/// The body of a file: everything after its head line.
fn body<'a>(a: &'a [Artifact], path: &str) -> &'a str {
    let f = a
        .iter()
        .find(|x| x.path == path)
        .unwrap_or_else(|| panic!("no {path}"));
    assert_eq!(f.class, Class::Generated, "{path}");
    assert!(untouched(&f.content), "{path}");
    f.content.split_once('\n').unwrap().1
}

#[test]
fn the_manifest_is_an_appstream_addon_run_by_the_forms_host_and_names_layout_cover_and_contract() {
    let a = out(TASKS);
    let want = "<component type=\"addon\">\n  <id>net.freesynergy.Tasks.board</id>\n  <extends>net.freesynergy.Tasks</extends>\n  <name>Tasks board</name>\n  \
                <summary>Tasks, dependencies and boards</summary>\n  <requires>\n    <id version=\"0.1\" compare=\"ge\">0xc04d258971543476</id>\n  </requires>\n  \
                <custom>\n    <value key=\"fs:frontend-id\">fs-tasks-board</value>\n    <value key=\"fs:exec\">fs-forms-run</value>\n    \
                <value key=\"fs:layout\">forms/board.toml</value>\n    \
                <value key=\"fs:covers\">Items.list,Items.query,Items.create,Items.editIf,Items.readiness</value>\n    \
                <value key=\"fs:contract\">0.1</value>\n  </custom>\n</component>\n";
    assert_eq!(
        body(&a, "packaging/frontends/fs-tasks-board.metainfo.xml"),
        want
    );
}

#[test]
fn the_package_layout_carries_the_exchange_file_and_what_it_covers() {
    let a = out(TASKS);
    let want = "[package]\nname = \"fs-tasks-layout\"\nprogram = \"tasks\"\nversion = \"0.0.1\"\nhost = \"fs-forms-run\"\n\n[[file]]\nrole = \"exchange\"\n\
                path = \"forms/board.toml\"\n\
                covers = \"Items.list,Items.query,Items.create,Items.editIf,Items.readiness\"\n";
    let text = body(&a, "packaging/layout/fs-tasks-layout.toml");
    assert_eq!(text, want);
    let t: toml::Table = toml::from_str(text).unwrap();
    assert_eq!(t["file"][0]["path"].as_str(), Some("forms/board.toml"));
}

#[test]
fn covers_all_is_written_as_the_word_and_a_text_with_xml_characters_is_escaped() {
    let src = edit(
        &edit(
            TASKS,
            "covers = [\"Items.list\", \"Items.query\", \"Items.create\", \"Items.editIf\", \"Items.readiness\"]",
            "covers = \"all\"",
        ),
        "en = \"Tasks, dependencies and boards\"",
        "en = \"A & B <c>\"",
    );
    let a = out(&src);
    let m = body(&a, "packaging/frontends/fs-tasks-board.metainfo.xml");
    assert!(m.contains("<value key=\"fs:covers\">all</value>"), "{m}");
    assert!(m.contains("<summary>A &amp; B &lt;c&gt;</summary>"), "{m}");
    assert!(body(&a, "packaging/layout/fs-tasks-layout.toml").contains("covers = \"all\"\n"));
}

#[test]
fn every_blueprint_gets_its_own_two_frontend_files_and_no_frontend_crate() {
    for i in 0..COUNT {
        let bp = blueprint(i);
        let list = generate(&bp, &FakeCatalog).unwrap();
        let name = &bp.program.name;
        let id = format!("fs-{name}-{}", bp.frontend.standard);
        for path in [
            format!("packaging/frontends/{id}.metainfo.xml"),
            format!("packaging/layout/fs-{name}-layout.toml"),
        ] {
            assert!(list.iter().any(|a| a.path == path), "{path}");
        }
        assert!(
            !list
                .iter()
                .any(|a| a.path.starts_with(&format!("crates/{id}"))),
            "{name}: a frontend crate"
        );
    }
}
