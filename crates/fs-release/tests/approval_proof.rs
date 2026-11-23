//! Pattern: none (test). `JA am <SHA>` only with the proof of the runner; a hand-written verdict is refused (G6 no. 10).

use fs_release::approval::{verify, Evidence, Reject};

const SHA: &str = "0123456789abcdef0123456789abcdef01234567";
const RUN: &str = "20261123T100000-1-rev-x";

fn log(task: &str, model: &str, exit: &str) -> String {
    format!("| UTC | Aufgabe | Modell | a | b | c | USD | s | Exit | Quelle | RunID |\n| 2026-11-23T10:00:00Z | {task} | {model} | 1 | 2 | 3 | 0.1 | 5 | {exit} | claude | {RUN} |\n")
}

fn record(model: &str, cwd: &str, path: &str, body: &str) -> String {
    let init = serde_json::json!({"type":"system","subtype":"init","model":model,"cwd":cwd});
    let w = serde_json::json!({"type":"assistant","message":{"content":[{"type":"tool_use","name":"Write","input":{"file_path":path,"content":body}}]}});
    format!("{init}\n{w}\n")
}

const VERDICT: &str = "JA am 0123456789abcdef0123456789abcdef01234567\nreport body\n";
const FULL: &str = "0123456789abcdef0123456789abcdef01234567";

fn good_rec() -> String {
    record(
        "claude-opus-5-5",
        "/r/wt/rev-x",
        "/r/wt/rev-x/BERICHT.md",
        &format!("JA am {FULL}\nreport body\n"),
    )
}

fn run(verdict: &str, l: &str, recs: &[String]) -> Result<(), Reject> {
    verify(&Evidence {
        verdict,
        sha: SHA,
        run_id: RUN,
        log: l,
        records: recs,
    })
}

#[test]
fn a_verdict_with_the_runner_proof_is_accepted() {
    let l = log("rev-x", "claude-opus-5-5", "0");
    assert_eq!(run(VERDICT, &l, &[good_rec()]), Ok(()));
}

#[test]
fn a_hand_written_verdict_without_a_record_is_refused() {
    let l = log("rev-x", "claude-opus-5-5", "0");
    assert_eq!(run(VERDICT, &l, &[]), Err(Reject::NoRunRecord));
    assert_eq!(
        run(VERDICT, "", &[good_rec()]),
        Err(Reject::NoSingleLogRow(RUN.into()))
    );
}

#[test]
fn the_runner_record_must_match_the_verdict_exactly() {
    let l = log("rev-x", "claude-opus-5-5", "0");
    let changed = "JA am 0123456789abcdef0123456789abcdef01234567\nforged body\n";
    assert_eq!(
        run(changed, &l, &[good_rec()]),
        Err(Reject::NoMatchingReport)
    );
    let other = record(
        "claude-opus-5-5",
        "/r/wt/rev-x",
        "/r/wt/rev-x/BERICHT.md",
        "JA am 1111111\nreport body\n",
    );
    assert_eq!(run(VERDICT, &l, &[other]), Err(Reject::NoMatchingReport));
    let elsewhere = record(
        "claude-opus-5-5",
        "/r/wt/rev-x",
        "/tmp/BERICHT.md",
        "JA am 0123456\nreport body\n",
    );
    assert_eq!(
        run(VERDICT, &l, &[elsewhere]),
        Err(Reject::NoMatchingReport)
    );
}

#[test]
fn wrong_head_model_mode_or_exit_is_refused() {
    let ok = log("rev-x", "claude-opus-5-5", "0");
    assert!(matches!(
        run(
            "JA am 1111111111111111111111111111111111111111\nreport body\n",
            &ok,
            &[good_rec()]
        ),
        Err(Reject::FirstLine(_))
    ));
    assert!(matches!(
        run(
            "NEIN am 0123456789abcdef0123456789abcdef01234567\nx\n",
            &ok,
            &[good_rec()]
        ),
        Err(Reject::FirstLine(_))
    ));
    assert!(matches!(
        run(
            VERDICT,
            &log("rev-x", "claude-sonnet-5-5", "0"),
            &[good_rec()]
        ),
        Err(Reject::NotOpus(_))
    ));
    assert!(matches!(
        run(
            VERDICT,
            &log("build-x", "claude-opus-5-5", "0"),
            &[good_rec()]
        ),
        Err(Reject::NotReviewTask(_))
    ));
    assert!(matches!(
        run(
            VERDICT,
            &log("rev-x", "claude-opus-5-5", "1"),
            &[good_rec()]
        ),
        Err(Reject::RunFailed(_))
    ));
    let sonnet = record(
        "claude-sonnet-5-5",
        "/r/wt/rev-x",
        "/r/wt/rev-x/BERICHT.md",
        "JA am 0123456\nreport body\n",
    );
    assert!(matches!(
        run(VERDICT, &ok, &[sonnet]),
        Err(Reject::RecordModel(_))
    ));
    let build = record(
        "claude-opus-5-5",
        "/r/wt/other",
        "/r/wt/other/BERICHT.md",
        "JA am 0123456\nreport body\n",
    );
    assert!(matches!(
        run(VERDICT, &ok, &[build]),
        Err(Reject::RecordFolder(_))
    ));
}

#[test]
fn a_report_that_names_only_a_prefix_of_the_head_is_refused() {
    let l = log("rev-x", "claude-opus-5-5", "0");
    let prefix_only = record(
        "claude-opus-5-5",
        "/r/wt/rev-x",
        "/r/wt/rev-x/BERICHT.md",
        "JA am 0123456\nreport body\n",
    );
    assert_eq!(
        run(VERDICT, &l, &[prefix_only]),
        Err(Reject::NoMatchingReport)
    );
    let short = "JA am 0123456\nreport body\n";
    assert!(matches!(
        run(short, &l, &[good_rec()]),
        Err(Reject::FirstLine(_))
    ));
}
