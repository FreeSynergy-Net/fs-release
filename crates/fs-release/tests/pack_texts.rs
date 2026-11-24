//! Pattern: none (test). `fs-release pack-texts`: exit codes and the same bytes in every run (local = CI).

use std::path::{Path, PathBuf};
use std::process::Command;

fn data() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../fs-texts-pack/tests/data")
}

fn run(sources: &Path, out: &Path, extra: &[&str]) -> (i32, String) {
    let o = Command::new(env!("CARGO_BIN_EXE_fs-release"))
        .arg("pack-texts")
        .arg("--sources")
        .arg(sources)
        .arg("--out")
        .arg(out)
        .args(extra)
        .output()
        .expect("run fs-release");
    (
        o.status.code().unwrap_or(-1),
        String::from_utf8_lossy(&o.stderr).into_owned(),
    )
}

fn files(dir: &Path) -> Vec<(String, Vec<u8>)> {
    let mut out = Vec::new();
    let mut todo = vec![dir.to_path_buf()];
    while let Some(d) = todo.pop() {
        for e in std::fs::read_dir(d).unwrap().flatten() {
            let p = e.path();
            if p.is_dir() {
                todo.push(p);
            } else {
                out.push((
                    p.strip_prefix(dir).unwrap().display().to_string(),
                    std::fs::read(&p).unwrap(),
                ));
            }
        }
    }
    out.sort();
    out
}

#[test]
fn two_runs_write_the_same_files() {
    let (a, b) = (tempfile::tempdir().unwrap(), tempfile::tempdir().unwrap());
    assert_eq!(run(&data(), a.path(), &["--locales", "de,en"]).0, 0);
    assert_eq!(run(&data(), b.path(), &[]).0, 0);
    let (fa, fb) = (files(a.path()), files(b.path()));
    assert_eq!(fa.len(), 5);
    assert_eq!(fa, fb);
}

#[test]
fn only_the_asked_languages_are_written_and_the_manifest_stays() {
    let (all, en) = (tempfile::tempdir().unwrap(), tempfile::tempdir().unwrap());
    assert_eq!(run(&data(), all.path(), &[]).0, 0);
    assert_eq!(run(&data(), en.path(), &["--locales", "en"]).0, 0);
    let names: Vec<String> = files(en.path()).into_iter().map(|f| f.0).collect();
    assert_eq!(names.len(), 3, "{names:?}");
    let manifest = |d: &Path| std::fs::read(d.join("manifest.mf2m")).unwrap();
    assert_eq!(manifest(all.path()), manifest(en.path()));
}

#[test]
fn a_missing_pair_is_exit_1_with_a_clear_message() {
    let src = tempfile::tempdir().unwrap();
    std::fs::copy(data().join("en.mf2"), src.path().join("en.mf2")).unwrap();
    std::fs::write(src.path().join("de.mf2"), "hello = Hallo, {$name}!\n").unwrap();
    let out = tempfile::tempdir().unwrap();
    let (rc, err) = run(src.path(), out.path(), &[]);
    assert_eq!(rc, 1);
    assert!(err.contains("files: in en, missing in de"), "{err}");
    assert!(files(out.path()).is_empty(), "nothing written on error");
}

#[test]
fn an_unknown_language_and_an_empty_folder_are_errors() {
    let out = tempfile::tempdir().unwrap();
    assert_eq!(run(&data(), out.path(), &["--locales", "fr"]).0, 2);
    let empty = tempfile::tempdir().unwrap();
    let (rc, err) = run(empty.path(), out.path(), &[]);
    assert_eq!(rc, 1);
    assert!(err.contains("no *.mf2 file"), "{err}");
}
