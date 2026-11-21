//! B3 (report of 21 Nov.): `new` makes a real forms description when the author brought none, and `check` is red when the description cannot be read by the host.
use fs_blueprint::check_layout::unreadable;
use fs_blueprint::cli::run;
use std::path::{Path, PathBuf};

fn cli(words: &[&str]) -> (i32, String) {
    let words: Vec<String> = words.iter().map(|w| (*w).to_string()).collect();
    let (mut out, mut err) = (Vec::new(), Vec::new());
    let code = run(&words, &mut out, &mut err);
    (
        code,
        String::from_utf8(out).unwrap() + &String::from_utf8(err).unwrap(),
    )
}

fn scratch(name: &str) -> PathBuf {
    let d = Path::new(env!("CARGO_TARGET_TMPDIR"))
        .join(format!("layout-{name}-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&d);
    std::fs::create_dir_all(&d).unwrap();
    d
}

fn probe_catalog(dir: &Path) -> PathBuf {
    let path = dir.join("catalog.toml");
    let text = fs_blueprint::catalog_file::BUILTIN.replacen(
        "capabilities = [",
        "capabilities = [\"probing\", ",
        1,
    );
    std::fs::write(&path, text).unwrap();
    path
}

/// The probe blueprint alone (no `forms/` folder next to it).
fn bare_probe(dir: &Path) -> PathBuf {
    let src = dir.join("src");
    std::fs::create_dir_all(&src).unwrap();
    let from = Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/data/probe/scaffold.toml");
    std::fs::copy(from, src.join("scaffold.toml")).unwrap();
    src
}

#[test]
fn new_makes_a_minimal_valid_forms_description_when_the_author_brought_none() {
    let d = scratch("made");
    let src = bare_probe(&d);
    let (cat, out) = (probe_catalog(&d), d.join("out"));
    let (code, text) = cli(&[
        "new",
        src.join("scaffold.toml").to_str().unwrap(),
        "--out",
        out.to_str().unwrap(),
        "--catalog",
        cat.to_str().unwrap(),
    ]);
    assert_eq!(code, 0, "{text}");
    let made = std::fs::read_to_string(out.join("forms/simple.toml")).unwrap();
    assert_eq!(unreadable(&made), None, "{made}");
    for needle in [
        "kind = \"frontend\"",
        "id = \"simple\"",
        "extends = \"net.freesynergy.Probe\"",
        "exec = \"fs-forms-run\"",
        "contract = \"probing >=0.1 <0.2\"",
        "lang = \"de\"",
    ] {
        assert!(made.contains(needle), "{needle} missing in\n{made}");
    }
    // the repo that was made checks clean, also against the generator (the seed is there, nothing drifts)
    let (code, text) = cli(&[
        "check",
        "--repo",
        out.to_str().unwrap(),
        "--catalog",
        cat.to_str().unwrap(),
        "--drift",
    ]);
    assert_eq!(code, 0, "{text}");
}

#[test]
fn a_description_the_author_brought_stays_and_is_not_replaced() {
    let d = scratch("kept");
    let src = bare_probe(&d);
    std::fs::create_dir_all(src.join("forms")).unwrap();
    let own = "schema = 1\nkind = \"frontend\"\nid = \"own\"\nextends = \"x\"\nexec = \"fs-forms-run\"\ntitle = \"t\"\n";
    std::fs::write(src.join("forms/simple.toml"), own).unwrap();
    let (cat, out) = (probe_catalog(&d), d.join("out"));
    let (code, text) = cli(&[
        "new",
        src.join("scaffold.toml").to_str().unwrap(),
        "--out",
        out.to_str().unwrap(),
        "--catalog",
        cat.to_str().unwrap(),
    ]);
    assert_eq!(code, 0, "{text}");
    assert_eq!(
        std::fs::read_to_string(out.join("forms/simple.toml")).unwrap(),
        own
    );
}

#[test]
fn a_description_the_host_cannot_read_is_red_and_nothing_is_written() {
    let d = scratch("red");
    let src = bare_probe(&d);
    std::fs::create_dir_all(src.join("forms")).unwrap();
    std::fs::write(src.join("forms/simple.toml"), "# only a comment\n").unwrap();
    let (cat, out) = (probe_catalog(&d), d.join("out"));
    let (code, text) = cli(&[
        "new",
        src.join("scaffold.toml").to_str().unwrap(),
        "--out",
        out.to_str().unwrap(),
        "--catalog",
        cat.to_str().unwrap(),
    ]);
    assert_eq!(code, 1, "{text}");
    assert!(
        text.contains("layout-unreadable{path=forms/simple.toml, cause=key-missing schema}"),
        "{text}"
    );
    assert!(!out.exists());
}

#[test]
fn the_content_rules_of_the_head() {
    let ok = "schema = 2\nkind = \"frontend\"\nid = \"a\"\nextends = \"b\"\nexec = \"c\"\ntitle = \"d\"\n";
    assert_eq!(unreadable(ok), None);
    let cases = [
        ("not = toml [", "toml"),
        ("# comment\n", "key-missing schema"),
        ("schema = 3\n", "schema-unsupported"),
        ("schema = 1\nkind = \"backend\"\n", "kind"),
        (
            "schema = 1\nkind = \"frontend\"\nid = \"a\"\n",
            "key-missing extends",
        ),
        (&format!("{ok}wat = 1\n"), "key-unknown wat"),
        (&ok.replace("id = \"a\"", "id = \"\""), "key-missing id"),
    ];
    for (text, cause) in cases {
        assert_eq!(unreadable(text).as_deref(), Some(cause), "{text}");
    }
}
