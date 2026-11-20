//! `fs-blueprint new` writes nothing when a place for the blueprint or its forms is taken by another content: the target is byte for byte as it was (S4 review B1).
mod support;
use fs_blueprint::cli::run;
use std::path::{Path, PathBuf};

fn fixture(name: &str) -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("tests/data/repos")
        .join(name)
}

fn cli(words: &[&str]) -> (i32, String, String) {
    let words: Vec<String> = words.iter().map(|w| (*w).to_string()).collect();
    let (mut out, mut err) = (Vec::new(), Vec::new());
    let code = run(&words, &mut out, &mut err);
    (
        code,
        String::from_utf8(out).unwrap(),
        String::from_utf8(err).unwrap(),
    )
}

fn scratch(name: &str) -> PathBuf {
    let d =
        Path::new(env!("CARGO_TARGET_TMPDIR")).join(format!("cli-{name}-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&d);
    d
}

fn files(root: &Path) -> Vec<String> {
    let mut out = Vec::new();
    let mut todo = vec![root.to_path_buf()];
    while let Some(d) = todo.pop() {
        for e in std::fs::read_dir(&d).unwrap() {
            let p = e.unwrap().path();
            if p.is_dir() {
                todo.push(p);
            } else {
                out.push(
                    p.strip_prefix(root)
                        .unwrap()
                        .to_string_lossy()
                        .replace('\\', "/"),
                );
            }
        }
    }
    out.sort();
    out
}

/// Every file below `root` with its bytes: the target tree as a value.
fn snapshot(root: &Path) -> std::collections::BTreeMap<String, Vec<u8>> {
    files(root)
        .into_iter()
        .map(|f| {
            let bytes =
                std::fs::read(f.split('/').fold(root.to_path_buf(), |p, c| p.join(c))).unwrap();
            (f, bytes)
        })
        .collect()
}

#[test]
fn new_leaves_the_target_byte_for_byte_as_it_was_after_every_kind_of_conflict() {
    let bp = fixture("ok").join("scaffold.toml");
    // 1. a foreign scaffold.toml
    let t1 = scratch("conflict-scaffold");
    std::fs::create_dir_all(&t1).unwrap();
    std::fs::write(t1.join("scaffold.toml"), "# somebody else\n").unwrap();
    // 2. a foreign file in forms/ (and a hand-written file next to it)
    let t2 = scratch("conflict-forms");
    std::fs::create_dir_all(t2.join("forms")).unwrap();
    std::fs::write(t2.join("forms").join("demo.toml"), "# mine\n").unwrap();
    std::fs::write(t2.join("notes.txt"), "keep\n").unwrap();
    for t in [&t1, &t2] {
        let before = snapshot(t);
        let (code, _, err) = cli(&["new", &bp.to_string_lossy(), "--out", &t.to_string_lossy()]);
        assert_eq!(code, 2, "{err}");
        assert!(err.contains("nothing was written"), "{err}");
        assert_eq!(snapshot(t), before, "{}", t.display());
    }
}

#[test]
fn new_of_another_blueprint_into_the_repo_of_a_first_one_changes_nothing() {
    let target = scratch("conflict-other");
    let a = fixture("ok").join("scaffold.toml");
    let (code, _, _) = cli(&[
        "new",
        &a.to_string_lossy(),
        "--out",
        &target.to_string_lossy(),
    ]);
    assert_eq!(code, 0);
    // blueprint B: the same program with another summary and its own forms folder
    let b_dir = scratch("conflict-other-b");
    std::fs::create_dir_all(b_dir.join("forms")).unwrap();
    std::fs::copy(
        fixture("ok").join("forms").join("demo.toml"),
        b_dir.join("forms").join("demo.toml"),
    )
    .unwrap();
    let text = std::fs::read_to_string(&a)
        .unwrap()
        .replace("A demo program", "Another demo program");
    std::fs::write(b_dir.join("scaffold.toml"), text).unwrap();
    let before = snapshot(&target);
    let (code, _, err) = cli(&[
        "new",
        &b_dir.join("scaffold.toml").to_string_lossy(),
        "--out",
        &target.to_string_lossy(),
    ]);
    assert_eq!(code, 2, "{err}");
    assert_eq!(
        snapshot(&target),
        before,
        "the generated files of the first blueprint were replaced"
    );
    let (code, out, _) = cli(&["check", "--repo", &target.to_string_lossy(), "--drift"]);
    assert_eq!(
        code, 0,
        "the repo is still the output of the first blueprint: {out}"
    );
}

fn new_words(bp: &Path, target: &Path) -> Vec<String> {
    [
        "new",
        &bp.to_string_lossy(),
        "--out",
        &target.to_string_lossy(),
    ]
    .iter()
    .map(|s| (*s).to_string())
    .collect()
}

#[test]
fn a_folder_at_the_place_of_the_blueprint_or_of_a_forms_file_is_a_conflict_and_nothing_is_written()
{
    let bp = fixture("ok").join("scaffold.toml");
    for (name, place) in [
        ("scaffold", "scaffold.toml"),
        ("forms-file", "forms/demo.toml"),
    ] {
        let t = scratch(&format!("folder-{name}"));
        std::fs::create_dir_all(t.join(place).join("inside")).unwrap();
        std::fs::write(t.join(place).join("inside").join("x.txt"), "x").unwrap();
        let before = snapshot(&t);
        let (code, _, err) = cli(&["new", &bp.to_string_lossy(), "--out", &t.to_string_lossy()]);
        assert_eq!(code, 2, "{name}: {err}");
        assert!(err.contains("nothing was written"), "{err}");
        assert_eq!(snapshot(&t), before, "{name}");
    }
}

#[cfg(unix)]
#[test]
fn a_file_that_cannot_be_read_at_the_place_is_a_conflict_and_nothing_is_written() {
    use std::os::unix::fs::PermissionsExt;
    let bp = fixture("ok").join("scaffold.toml");
    let t = scratch("unreadable");
    std::fs::create_dir_all(&t).unwrap();
    let file = t.join("scaffold.toml");
    std::fs::write(&file, "# mine\n").unwrap();
    std::fs::set_permissions(&file, std::fs::Permissions::from_mode(0o000)).unwrap();
    if std::fs::read(&file).is_ok() {
        // running with rights that read everything: the situation cannot be made here
        return;
    }
    let before = files(&t);
    let (code, _, err) = cli(&["new", &bp.to_string_lossy(), "--out", &t.to_string_lossy()]);
    std::fs::set_permissions(&file, std::fs::Permissions::from_mode(0o644)).unwrap();
    assert_eq!(code, 2, "{err}");
    assert!(err.contains("nothing was written"), "{err}");
    assert_eq!(files(&t), before);
    assert_eq!(std::fs::read_to_string(&file).unwrap(), "# mine\n");
}

#[test]
fn a_file_that_appears_between_the_look_and_the_write_is_never_overwritten_and_the_message_says_what_was_written(
) {
    let bp = fixture("ok").join("scaffold.toml");
    let t = scratch("race");
    let late = t.join("scaffold.toml");
    let (mut out, words) = (Vec::new(), new_words(&bp, &t));
    let err = fs_blueprint::cli_new::new_with_hook(&words[1..], &mut out, &|| {
        std::fs::create_dir_all(&t).unwrap();
        std::fs::write(&late, "# appeared late\n").unwrap();
    })
    .unwrap_err();
    assert!(
        err.contains("already written") && !err.contains("nothing was written"),
        "{err}"
    );
    assert_eq!(
        std::fs::read_to_string(&late).unwrap(),
        "# appeared late\n",
        "the late file was not touched"
    );
    assert!(
        t.join(".github")
            .join("workflows")
            .join("check.yml")
            .is_file(),
        "the program files are there, as the message says"
    );
}
