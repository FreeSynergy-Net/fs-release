//! Pattern: none (test). `fs-release dag-check`: the graph of `pin-kaskade` is read from Cargo.toml files (fixtures in a temp folder), the header is proven against it and
//! generated from it. Counter-tests: every wrong header (edge removed, wrong order, stale rev pin, missing Glied) is red.

use std::fs;
use std::path::Path;

use fs_release::dag::{check, emit, foreign_repos, git_owners, parse_header, scan};

fn dep(repo: &str, extra: &str) -> String {
    format!("{repo} = {{ git = \"https://github.com/FreeSynergy/{repo}\", {extra} }}\n")
}

/// types <- jscalendar <- interfaces <- rpc <- app; components <- types; rpc <- components; app has a rev pin and a dev dependency; foreign Blinc is skipped.
fn fixture(root: &Path) {
    let w = |repo: &str, sub: &str, body: &str| {
        let d = root.join(repo).join(sub);
        fs::create_dir_all(&d).unwrap();
        fs::write(
            d.join("Cargo.toml"),
            format!("[package]\nname = \"{repo}\"\n{body}"),
        )
        .unwrap();
    };
    w("fs-types", "", "");
    w(
        "fs-jscalendar",
        "",
        &format!("[dependencies]\n{}", dep("fs-types", "tag = \"v0.0.6\"")),
    );
    w(
        "fs-interfaces",
        "crates/fs-events",
        &format!(
            "[dependencies]\n{}{}",
            dep("fs-types", "tag = \"v0.0.6\""),
            dep("fs-jscalendar", "tag = \"v0.0.6\"")
        ),
    );
    w(
        "fs-components",
        "",
        &format!(
            "[dependencies]\n{}[patch.\"x\"]\n{}",
            dep("fs-types", "rev = \"2f45110417cf\""),
            dep("fs-rpc", "branch = \"main\"")
        ),
    );
    w(
        "fs-rpc",
        "crates/a",
        &format!(
            "[dependencies]\n{}{}",
            dep("fs-interfaces", "tag = \"v1\""),
            dep("fs-components", "tag = \"v1\"")
        ),
    );
    w("fs-app", "", &format!("[dependencies]\n{}[dev-dependencies]\n{}Blinc = {{ git = \"https://github.com/FreeSynergy/Blinc\", tag = \"x\" }}\n", dep("fs-rpc", "tag = \"v1\""), dep("fs-types", "tag = \"v1\"")));
}

const GOOD: &str = "# GLIEDER-ORDER: fs-jscalendar, fs-interfaces, fs-components, fs-rpc, fs-app
# DAG: fs-jscalendar <- fs-types
# DAG: fs-interfaces <- fs-jscalendar, fs-types
# DAG: fs-components <- fs-types
# DAG: fs-rpc <- fs-components, fs-interfaces
# DAG: fs-app <- fs-rpc, fs-types
# REV-PIN: fs-components -> fs-types rev 2f451104
";

fn red(header: &str) -> Vec<String> {
    let d = tempfile::tempdir().unwrap();
    fixture(d.path());
    let foreign = foreign_repos("FreeSynergy/Blinc  # wrapped\n");
    let g = scan(d.path(), &foreign).unwrap();
    check(&g, &parse_header(header), "fs-types")
}

#[test]
fn git_owners_reads_urls_revs_and_skips_patch_sections() {
    let t = "[dependencies]\na = { git = \"https://github.com/FreeSynergy/fs-types.git\", rev = \"0123456789ab\" }\nb = { git = \"https://github.com/other/x\" }\n[patch.\"u\"]\nc = { git = \"https://github.com/FreeSynergy/fs-rpc\" }\n";
    assert_eq!(
        git_owners(t),
        vec![("fs-types".to_string(), Some("01234567".to_string()))]
    );
}

#[test]
fn the_good_header_is_green() {
    assert_eq!(red(GOOD), Vec::<String>::new());
}

#[test]
fn counter_every_wrong_header_is_red() {
    let one = |from: &str, to: &str, want: &str| {
        let r = red(&GOOD.replace(from, to));
        assert!(r.iter().any(|l| l.contains(want)), "{from} -> {to}: {r:?}");
    };
    // the old errors: an edge removed (both kinds), a Glied before its dependency, a missing Glied, a stale rev pin, a surplus edge
    one(
        "# DAG: fs-interfaces <- fs-jscalendar, fs-types",
        "# DAG: fs-interfaces <- fs-types",
        "edge missing in DAG: fs-interfaces <- fs-jscalendar",
    );
    one(
        "# DAG: fs-rpc <- fs-components, fs-interfaces",
        "# DAG: fs-rpc <- fs-interfaces",
        "edge missing in DAG: fs-rpc <- fs-components",
    );
    one(
        "# DAG: fs-app <- fs-rpc, fs-types",
        "# DAG: fs-app <- fs-rpc",
        "edge missing in DAG: fs-app <- fs-types",
    );
    one(
        "fs-jscalendar, fs-interfaces, fs-components, fs-rpc, fs-app",
        "fs-interfaces, fs-jscalendar, fs-components, fs-rpc, fs-app",
        "fs-jscalendar must stand before fs-interfaces",
    );
    one(
        "fs-jscalendar, fs-interfaces, fs-components, fs-rpc, fs-app",
        "fs-jscalendar, fs-interfaces, fs-rpc, fs-components, fs-app",
        "fs-components must stand before fs-rpc",
    );
    one(
        "fs-jscalendar, fs-interfaces, fs-components, fs-rpc, fs-app",
        "fs-jscalendar, fs-interfaces, fs-components, fs-rpc",
        "Glied missing in GLIEDER-ORDER: fs-app",
    );
    one(
        "rev 2f451104",
        "rev 339aa24f",
        "REV-PIN missing: fs-components -> fs-types rev 2f451104",
    );
    one(
        "# DAG: fs-components <- fs-types",
        "# DAG: fs-components <- fs-types, fs-conformance",
        "edge surplus in DAG: fs-components <- fs-conformance",
    );
    one(
        "# REV-PIN: fs-components -> fs-types rev 2f451104\n",
        "",
        "REV-PIN missing",
    );
}

#[test]
fn emit_writes_a_header_that_is_green_and_keeps_the_current_order() {
    let d = tempfile::tempdir().unwrap();
    fixture(d.path());
    let g = scan(d.path(), &foreign_repos("FreeSynergy/Blinc")).unwrap();
    let cur: Vec<String> = [
        "fs-jscalendar",
        "fs-interfaces",
        "fs-components",
        "fs-rpc",
        "fs-app",
    ]
    .map(String::from)
    .to_vec();
    let out = emit(&g, "fs-types", &cur).unwrap();
    assert_eq!(out, GOOD);
    assert!(check(&g, &parse_header(&out), "fs-types").is_empty());
    // from a wrong current order the generated one is still topological
    let bad: Vec<String> = [
        "fs-app",
        "fs-rpc",
        "fs-interfaces",
        "fs-components",
        "fs-jscalendar",
    ]
    .map(String::from)
    .to_vec();
    let out = emit(&g, "fs-types", &bad).unwrap();
    assert!(check(&g, &parse_header(&out), "fs-types").is_empty());
}

#[test]
fn a_cycle_is_red() {
    let d = tempfile::tempdir().unwrap();
    fixture(d.path());
    // fs-jscalendar depending on fs-app closes a cycle
    let p = d.path().join("fs-jscalendar/Cargo.toml");
    let mut t = fs::read_to_string(&p).unwrap();
    t.push_str(&dep("fs-app", "tag = \"v1\""));
    fs::write(&p, t).unwrap();
    let g = scan(d.path(), &foreign_repos("")).unwrap();
    assert!(check(&g, &parse_header(GOOD), "fs-types")
        .iter()
        .any(|l| l.contains("cycle")));
    assert!(emit(&g, "fs-types", &[]).is_err());
}
