//! `fs-blueprint fmt` (ADR 0099 point 14): the canonical form is a fixed point that keeps the meaning (the blueprint hash), is written atomically, and a file
//! with a comment is refused instead of losing it.
mod support;
use fs_blueprint::cli::run;
use fs_blueprint::cli_fmt::{canonical, fmt_file, has_comment};
use fs_blueprint::emit::header::blueprint_hash;
use fs_blueprint::Blueprint;
use std::path::{Path, PathBuf};
use support::TASKS;

fn scratch(name: &str) -> PathBuf {
    let d =
        Path::new(env!("CARGO_TARGET_TMPDIR")).join(format!("fmt-{name}-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&d);
    std::fs::create_dir_all(&d).unwrap();
    d
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

fn canon() -> String {
    canonical(&Blueprint::from_toml(TASKS).unwrap()).unwrap()
}

#[test]
fn the_canonical_form_is_a_fixed_point_and_keeps_the_meaning() {
    let bp = Blueprint::from_toml(TASKS).unwrap();
    let text = canonical(&bp).unwrap();
    let again = Blueprint::from_toml(&text).unwrap();
    assert_eq!(again, bp);
    assert_eq!(
        blueprint_hash(&again).unwrap(),
        blueprint_hash(&bp).unwrap()
    );
    assert_eq!(canonical(&again).unwrap(), text);
    assert!(text.ends_with("\n") && !text.ends_with("\n\n") && !text.contains('\r'));
    assert!(text.starts_with("schema = 2\n"), "{}", &text[..40]);
}

#[test]
fn a_file_that_is_not_canonical_is_written_once_and_then_left_alone() {
    let d = scratch("write");
    let f = d.join("scaffold.toml");
    std::fs::write(&f, format!("{}\n\n\n", canon())).unwrap();
    let (code, out, err) = cli(&["fmt", f.to_str().unwrap()]);
    assert_eq!((code, out.as_str(), err.as_str()), (0, "", ""));
    assert_eq!(std::fs::read_to_string(&f).unwrap(), canon());
    let before = std::fs::metadata(&f).unwrap().modified().unwrap();
    assert_eq!(cli(&["fmt", f.to_str().unwrap()]).0, 0);
    assert_eq!(std::fs::metadata(&f).unwrap().modified().unwrap(), before);
    let names: Vec<_> = std::fs::read_dir(&d)
        .unwrap()
        .map(|e| e.unwrap().file_name())
        .collect();
    assert_eq!(names.len(), 1, "no temporary file stays: {names:?}");
}

#[test]
fn check_says_whether_the_file_is_canonical_and_writes_nothing() {
    let d = scratch("check");
    let f = d.join("scaffold.toml");
    std::fs::write(&f, canon()).unwrap();
    assert_eq!(cli(&["fmt", "--check", f.to_str().unwrap()]).0, 0);
    let loose = format!("{}\n\n", canon());
    std::fs::write(&f, &loose).unwrap();
    let (code, out, _) = cli(&["fmt", "--check", f.to_str().unwrap()]);
    assert_eq!(code, 1);
    assert!(out.contains("fmt-changed{path="), "{out}");
    assert_eq!(std::fs::read_to_string(&f).unwrap(), loose);
    let (_, de, _) = cli(&["fmt", "--check", "--lang", "de", f.to_str().unwrap()]);
    assert!(de.contains("nicht in der kanonischen Form"), "{de}");
}

#[test]
fn a_file_with_a_comment_is_refused_and_not_changed() {
    let d = scratch("comment");
    let f = d.join("scaffold.toml");
    let text = format!("# my note\n{}", canon());
    std::fs::write(&f, &text).unwrap();
    for words in [
        vec!["fmt", f.to_str().unwrap()],
        vec!["fmt", "--check", f.to_str().unwrap()],
    ] {
        let (code, out, _) = cli(&words);
        assert_eq!(code, 1, "{words:?}");
        assert!(out.contains("fmt-comments{path="), "{out}");
    }
    assert_eq!(std::fs::read_to_string(&f).unwrap(), text);
}

#[test]
fn a_blueprint_with_an_error_and_a_missing_file_are_not_formatted() {
    let d = scratch("bad");
    let f = d.join("scaffold.toml");
    std::fs::write(&f, "schema = 1\n").unwrap();
    let (code, out, _) = cli(&["fmt", f.to_str().unwrap()]);
    assert_eq!(code, 1);
    assert!(out.contains("schema-unsupported"), "{out}");
    assert_eq!(std::fs::read_to_string(&f).unwrap(), "schema = 1\n");
    let (code, _, err) = cli(&["fmt", d.join("none.toml").to_str().unwrap()]);
    assert_eq!(code, 2);
    assert!(err.contains("cannot read"), "{err}");
    assert_eq!(cli(&["fmt", "a", "b"]).0, 2);
    assert_eq!(cli(&["fmt", "--nonsense"]).0, 2);
}

#[test]
fn a_failure_while_moving_into_place_keeps_the_old_file_and_leaves_no_temporary_file() {
    let d = scratch("atomic");
    let f = d.join("scaffold.toml");
    let old = format!("{}\n\n", canon());
    std::fs::write(&f, &old).unwrap();
    let mut out = Vec::new();
    let r = fmt_file(
        &f,
        false,
        "en",
        |_, _| Err(std::io::Error::other("stop")),
        &mut out,
    );
    assert!(r.unwrap_err().contains("cannot write"));
    assert_eq!(std::fs::read_to_string(&f).unwrap(), old);
    let names: Vec<_> = std::fs::read_dir(&d)
        .unwrap()
        .map(|e| e.unwrap().file_name())
        .collect();
    assert_eq!(names.len(), 1, "{names:?}");
}

#[test]
fn a_comment_is_found_only_outside_of_strings() {
    for (src, want) in [
        ("a = 1 # c\n", true),
        ("# c\n", true),
        ("a = \"x # y\"\n", false),
        ("a = 'x # y'\n", false),
        ("a = \"x \\\" # y\"\n", false),
        ("a = \"\"\"\nx # y\n\"\"\"\n", false),
        ("a = '''\nx # y\n'''\n", false),
        ("a = \"\"\"x\"\"\" # c\n", true),
        ("a = [\"#\", 1] # c\n", true),
        ("a = [\"#\", 1]\n", false),
        ("", false),
    ] {
        assert_eq!(has_comment(src), want, "{src:?}");
    }
}
