//! Pattern: none (test). Colour codes (`CARGO_TERM_COLOR=always`, as in the CI) never change the error line the chain shows.

use fs_release_chain::sys::{strip_ansi, Out};

const COLOURED: &str =
    "\u{1b}[1m\u{1b}[91merror\u{1b}[0m\u{1b}[1m: this could be `is_empty`\u{1b}[0m\n";

#[test]
fn strip_ansi_removes_colour_sequences_and_keeps_the_text() {
    assert_eq!(strip_ansi(COLOURED), "error: this could be `is_empty`\n");
    assert_eq!(strip_ansi("plain"), "plain");
    assert_eq!(strip_ansi("a\u{1b}[0"), "a");
}

#[test]
fn first_error_of_coloured_output_starts_with_error() {
    let o = Out {
        code: 1,
        stdout: String::new(),
        stderr: format!("\u{1b}[1m\u{1b}[92m   Compiling\u{1b}[0m x\n{COLOURED}"),
    };
    assert_eq!(o.first_error(), "error: this could be `is_empty`");
}

/// Mutant: without the removal the raw line starts with the escape byte, so the comparison of the lift test would be red.
#[test]
fn without_the_removal_the_coloured_line_does_not_start_with_error() {
    let raw = COLOURED.lines().next().unwrap();
    assert!(!raw.starts_with("error"));
    assert!(strip_ansi(raw).starts_with("error"));
}
