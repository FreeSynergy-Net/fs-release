//! `fs-blueprint fmt` keeps comments (S5c): tables and keys move into the order of the schema, a comment goes with the line below it, nothing else changes.
mod support;
use fs_blueprint::cli::run;
use fs_blueprint::cli_fmt::{canonical, judge_with};
use fs_blueprint::emit::header::blueprint_hash;
use fs_blueprint::Blueprint;
use std::path::{Path, PathBuf};
use support::TASKS;

fn scratch(name: &str) -> PathBuf {
    let d =
        Path::new(env!("CARGO_TARGET_TMPDIR")).join(format!("fmtc-{name}-{}", std::process::id()));
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

/// The canonical text with its blocks (a block = a table header and what follows it up to the next header) in a different order, comments on and in them.
fn scrambled() -> String {
    let canon = canon();
    let at = |h: &str| canon.find(h).unwrap_or_else(|| panic!("no {h}"));
    let (program, contract) = (at("[program]"), at("[contract]"));
    let head = &canon[..program];
    let program_block = &canon[program..contract];
    let rest = &canon[contract..];
    // the first table goes to the END with a comment above it and one at the end of its first line; a comment on top of the file and one in the middle of the rest
    let first_nl = program_block.find('\n').unwrap();
    let moved = format!(
        "# about the program\n{} # trailing note{}",
        &program_block[..first_nl],
        &program_block[first_nl..]
    );
    format!("# header of the file\n{head}# about the contract\n{rest}\n{moved}")
}

fn comment_lines(text: &str) -> Vec<String> {
    let mut v: Vec<String> = text
        .lines()
        .filter_map(|l| l.find('#').map(|i| l[i..].to_string()))
        .collect();
    v.sort();
    v
}

#[test]
fn comments_are_kept_and_move_with_the_line_below_them() {
    let d = scratch("comment");
    let f = d.join("scaffold.toml");
    let text = scrambled();
    std::fs::write(&f, &text).unwrap();
    assert_eq!(
        cli(&["fmt", "--check", f.to_str().unwrap()]).0,
        1,
        "not in order: --check says so"
    );
    let (code, out, err) = cli(&["fmt", f.to_str().unwrap()]);
    assert_eq!((code, out.as_str(), err.as_str()), (0, "", ""));
    let done = std::fs::read_to_string(&f).unwrap();
    assert_eq!(
        comment_lines(&done),
        comment_lines(&text),
        "no comment lost, none made up"
    );
    // the order is the order of the schema again, the comment above [program] came along, the trailing one stayed on its line
    assert!(done.find("[program]").unwrap() < done.find("[contract]").unwrap());
    assert!(
        done.contains("# about the program\n[program] # trailing note\n"),
        "{done}"
    );
    assert!(done.contains("# about the contract\n[contract]"), "{done}");
    assert!(
        done.starts_with("# header of the file\nschema = 2\n"),
        "{done}"
    );
    // the same blueprint, the same hash; a second run changes nothing; --check is green now
    let (a, b) = (
        Blueprint::from_toml(&text).unwrap(),
        Blueprint::from_toml(&done).unwrap(),
    );
    assert_eq!(a, b);
    assert_eq!(blueprint_hash(&a).unwrap(), blueprint_hash(&b).unwrap());
    assert_eq!(cli(&["fmt", "--check", f.to_str().unwrap()]).0, 0);
    assert_eq!(cli(&["fmt", f.to_str().unwrap()]).0, 0);
    assert_eq!(std::fs::read_to_string(&f).unwrap(), done);
}

#[test]
fn the_form_of_the_values_and_the_order_of_a_list_stay_the_authors() {
    let d = scratch("form");
    let f = d.join("scaffold.toml");
    // an inline table in another key order, a list of tables whose entries keep their order, CRLF line ends
    let text = canon()
        .replacen("[program]", "[program] # note", 1)
        .replace('\n', "\r\n");
    std::fs::write(&f, &text).unwrap();
    assert_eq!(cli(&["fmt", f.to_str().unwrap()]).0, 0);
    let done = std::fs::read_to_string(&f).unwrap();
    assert!(!done.contains('\r') && done.ends_with('\n') && !done.ends_with("\n\n"));
    assert!(done.contains("[program] # note"));
    let names = |t: &str| -> Vec<String> {
        Blueprint::from_toml(t)
            .unwrap()
            .module
            .iter()
            .map(|m| m.name.clone())
            .collect()
    };
    assert_eq!(names(&done), names(&text));
}

#[test]
fn keys_inside_a_table_are_put_in_order_and_their_comments_go_with_them() {
    let d = scratch("keys");
    let f = d.join("scaffold.toml");
    let canon = canon();
    // the first two key lines under [program] swap places; the second one has a comment above it and a trailing one
    let start = canon.find("[program]\n").unwrap() + "[program]\n".len();
    let lines: Vec<&str> = canon[start..].lines().collect();
    let swapped = format!("# first now\n{} # tail\n{}\n", lines[1], lines[0]);
    let rest = lines[2..].join("\n");
    let text = format!("{}{swapped}{rest}\n", &canon[..start]);
    std::fs::write(&f, &text).unwrap();
    assert_eq!(cli(&["fmt", f.to_str().unwrap()]).0, 0);
    let done = std::fs::read_to_string(&f).unwrap();
    let want = format!("# first now\n{} # tail\n", lines[1]);
    let at = done
        .find(&want)
        .unwrap_or_else(|| panic!("the comment went with its key: {done}"));
    assert!(
        done.find(lines[0]).unwrap() < at,
        "the key that is first in the schema comes first again: {done}"
    );
    assert_eq!(
        Blueprint::from_toml(&done).unwrap(),
        Blueprint::from_toml(&text).unwrap()
    );
}

#[test]
fn an_inline_table_gets_its_keys_in_the_order_of_the_schema() {
    let d = scratch("inline");
    let f = d.join("scaffold.toml");
    let swapped = TASKS.replace(
        "summary = { en = \"Tasks, dependencies and boards\", de = \"Aufgaben, Abhängigkeiten und Boards\" }",
        "summary = { de = \"Aufgaben, Abhängigkeiten und Boards\", en = \"Tasks, dependencies and boards\" }",
    );
    assert_ne!(swapped, TASKS);
    std::fs::write(&f, &swapped).unwrap();
    assert_eq!(cli(&["fmt", f.to_str().unwrap()]).0, 0);
    let done = std::fs::read_to_string(&f).unwrap();
    let line = done.lines().find(|l| l.starts_with("summary = ")).unwrap();
    assert!(
        line.find("en = ").unwrap() < line.find("de = ").unwrap(),
        "{line}"
    );
}

#[test]
fn carriage_returns_in_comments_are_gone_too() {
    let d = scratch("crlf");
    let f = d.join("scaffold.toml");
    std::fs::write(
        &f,
        format!("# top\r\n# second\r\n{}", canon().replace('\n', "\r\n")),
    )
    .unwrap();
    assert_eq!(cli(&["fmt", f.to_str().unwrap()]).0, 0);
    let done = std::fs::read_to_string(&f).unwrap();
    assert!(
        !done.contains('\r') && done.starts_with("# top\n# second\nschema = 2\n"),
        "{done}"
    );
}

#[test]
fn a_result_that_is_another_blueprint_or_no_toml_is_refused_and_never_written() {
    let path = Path::new("scaffold.toml");
    let other = canonical(
        &Blueprint::from_toml(&support::edit(
            TASKS,
            "name = \"tasks\"",
            "name = \"other\"",
        ))
        .unwrap(),
    )
    .unwrap();
    for broken in [Some(other), Some("not [toml".to_string()), None] {
        let e = judge_with(path, TASKS, |_, _| broken.clone()).unwrap_err();
        assert_eq!(e.to_string(), "fmt-unsafe{path=scaffold.toml}");
    }
    assert!(judge_with(path, TASKS, |t, _| Some(t.to_string())).is_ok());
}

#[test]
fn a_carriage_return_inside_a_multi_line_string_cannot_be_made_an_lf_without_changing_the_blueprint(
) {
    // LF only is the rule of the canonical form; a CR that is PART OF A VALUE (inside a multi-line string) would change the value: refused, nothing written
    let d = scratch("crlf-string");
    let f = d.join("scaffold.toml");
    let with_string = canon().replacen(
        "en = \"Tasks, dependencies and boards\"",
        "en = \"\"\"Tasks,\ndependencies and boards\"\"\"",
        1,
    );
    assert_ne!(with_string, canon());
    let text = with_string.replace('\n', "\r\n");
    std::fs::write(&f, &text).unwrap();
    let (code, out, _) = cli(&["fmt", f.to_str().unwrap()]);
    assert_eq!(code, 1, "{out}");
    assert!(out.contains("fmt-unsafe{path="), "{out}");
    assert_eq!(
        std::fs::read_to_string(&f).unwrap(),
        text,
        "nothing was written"
    );
}
