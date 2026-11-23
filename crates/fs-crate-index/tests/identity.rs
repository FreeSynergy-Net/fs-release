//! Pattern: none (integration test, r4 of K-2: the identity of an entry (name, version, file name) comes from the plan and the allow list, never from the content of the `.crate`; table in docs/crate-index.md).

mod support;

use std::fs;

use fs_crate_index::{apply, scan};
use support::{ok, run, World};

fn packaged(w: &mut World) -> fs_crate_index::ScanArgs {
    w.release("0.0.7", 42);
    let s = w.scan_args();
    let r = scan(&s).unwrap();
    assert!(r.error.is_none() && r.entries == 3, "{r:?}");
    s
}

/// Repacks `demo-base-0.0.7.crate` of the scan output: the file name stays, the name in the inner `Cargo.toml` becomes `inner`.
fn relabel_inner_name(w: &World, s: &fs_crate_index::ScanArgs, inner: &str) {
    let file = s.out.join("demo-base-0.0.7.crate");
    let t = w.root.join("t-relabel");
    let _ = fs::remove_dir_all(&t);
    fs::create_dir_all(&t).unwrap();
    ok(&run(&t, "tar", &["xzf", file.to_str().unwrap()], &[]));
    let manifest = t.join("demo-base-0.0.7/Cargo.toml");
    let text = fs::read_to_string(&manifest).unwrap();
    assert!(text.contains("name = \"demo-base\""), "{text}");
    fs::write(
        &manifest,
        text.replace("name = \"demo-base\"", &format!("name = \"{inner}\"")),
    )
    .unwrap();
    ok(&run(
        &t,
        "tar",
        &["czf", file.to_str().unwrap(), "demo-base-0.0.7"],
        &[],
    ));
}

#[test]
fn a_crate_whose_inner_name_differs_from_the_plan_line_is_red_and_nothing_is_entered() {
    // job `package` is foreign code: the file name and the plan say `demo-base`, the `.crate` says another name
    for inner in ["not-on-any-list", "demo-lib"] {
        let mut w = World::new();
        let s = packaged(&mut w);
        relabel_inner_name(&w, &s, inner);
        let before = w.head();
        let e = format!("{:#}", apply(&w.apply_args(&s)).unwrap_err());
        assert!(
            e.contains(&format!("contains the crate `{inner}`")),
            "{inner}: {e}"
        );
        assert_eq!(w.head(), before, "{inner}: no commit");
        assert!(
            !w.reg.join("crates").exists() && !w.reg.join("no").exists(),
            "{inner}: no file"
        );
    }
}

#[test]
fn a_plan_line_whose_version_is_not_the_version_of_its_tag_is_red_and_nothing_is_entered() {
    let mut w = World::new();
    let s = packaged(&mut w);
    // the plan (a file of job `package`) says 0.0.8 for the tag v0.0.7; file name and plan are consistent with each other
    fs::copy(
        s.out.join("demo-base-0.0.7.crate"),
        s.out.join("demo-base-0.0.8.crate"),
    )
    .unwrap();
    let plan = fs::read_to_string(s.out.join("plan.txt")).unwrap();
    let mut f: Vec<String> = plan
        .lines()
        .next()
        .unwrap()
        .split('\t')
        .map(Into::into)
        .collect();
    f[5] = "0.0.8".into(); // version
    f[6] = "demo-base-0.0.8.crate".into(); // file
    let first = f.join("\t");
    let rest: Vec<&str> = plan.lines().skip(1).collect();
    fs::write(
        s.out.join("plan.txt"),
        format!("{first}\n{}\n", rest.join("\n")),
    )
    .unwrap();
    let before = w.head();
    let e = format!("{:#}", apply(&w.apply_args(&s)).unwrap_err());
    assert!(e.contains("is not version 0.0.8"), "{e}");
    assert_eq!(w.head(), before, "no commit");
}

#[test]
fn a_plan_line_with_another_file_name_than_package_and_version_is_red() {
    let mut w = World::new();
    let s = packaged(&mut w);
    // a valid crate under a name the plan line must not use (also a path that leaves the hand-over directory)
    fs::copy(
        s.out.join("demo-base-0.0.7.crate"),
        w.root.join("escape.crate"),
    )
    .unwrap();
    let plan = fs::read_to_string(s.out.join("plan.txt")).unwrap();
    let mut f: Vec<String> = plan
        .lines()
        .next()
        .unwrap()
        .split('\t')
        .map(Into::into)
        .collect();
    f[6] = "../escape.crate".into();
    let rest: Vec<&str> = plan.lines().skip(1).collect();
    fs::write(
        s.out.join("plan.txt"),
        format!("{}\n{}\n", f.join("\t"), rest.join("\n")),
    )
    .unwrap();
    let before = w.head();
    let e = format!("{:#}", apply(&w.apply_args(&s)).unwrap_err());
    assert!(e.contains("unexpected file name"), "{e}");
    assert_eq!(w.head(), before, "no commit");
}
