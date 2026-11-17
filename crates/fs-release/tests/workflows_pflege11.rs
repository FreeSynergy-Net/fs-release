//! Pattern: none (test). Workflow maintenance run 6: the conditions of the Opus review of `upstream-watch` (own accounts as default, the WHOLE state is
//! carried on and merged, the first run is a baseline) and the three scripts of the `@fs` calls (read only). Each rule has a counter-test on a changed copy.

mod support;
use support::workflows::{read, root, run_self_test, step_body};

const ELSE_DEFAULT: &str = "if length == 0 then [\"FreeSynergy\"] else . end";
const STATE_KEYS: [&str; 4] = [
    "repo: ${ .input.repo // \"project-blinc/Blinc\" }",
    "letzte_pruefung: ${ .input.letzte_pruefung // null }",
    "gemerkt: ${ .input.gemerkt // [] }",
    "eigene: ${ (.input.eigene // []) | if length == 0 then [\"FreeSynergy\"] else . end }",
];

/// Condition 1: no input of `eigene` stays an empty list; the default and the fallback are our own account.
fn eigene_default(text: &str, steps: &[&str]) -> bool {
    steps
        .iter()
        .all(|s| step_body(text, s).contains(ELSE_DEFAULT))
        && !text.contains("eigene: ${ .input.upstream.eigene // [] }")
        && !text.contains("eigene: ${ .input.eigene }\n")
}

/// Condition 2: `gelb-melden` and `eskalation` carry the whole state from the input; `merken` returns it; the parents MERGE the result.
fn state_carried(watch: &str) -> bool {
    ["gelb-melden", "eskalation"].iter().all(|s| {
        let b = step_body(watch, s);
        STATE_KEYS.iter().all(|k| b.contains(k))
    }) && {
        let m = step_body(watch, "merken");
        m.contains("repo: ${ .input.repo")
            && m.contains("eigene: ${ (.input.eigene")
            && m.contains("`gemerkt` (= `neu.gemerkt`")
    } && !step_body(watch, "gelb-melden").contains("letzte_pruefung: null")
}

fn parent_merges(t: &str) -> bool {
    step_body(t, "upstream-watch")
        .contains("as: '${ $context + { upstream: ($context.upstream + .) } }'")
}

/// Condition 3: the baseline of the first run.
fn baseline(watch: &str) -> bool {
    let b = step_body(watch, "neu-filtern");
    b.contains("BASELINE")
        && b.contains("`neu.grundlinie`")
        && b.contains("only `gemerkt` is filled")
        && watch.contains("FIRST RUN = BASELINE")
}

#[test]
fn the_default_of_eigene_is_our_own_account() {
    let w = read("upstream-watch");
    assert!(eigene_default(
        &w,
        &["neu-filtern", "merken", "gelb-melden", "eskalation"]
    ));
    for f in ["koordinator-runde", "bahnen"] {
        assert!(eigene_default(&read(f), &["upstream-start"]), "{f}");
    }
    assert!(w.contains("accounts ending on `[bot]` are never new"));
}

#[test]
fn counter_a_default_that_is_empty_again_is_caught() {
    let w = read("upstream-watch");
    assert!(!eigene_default(
        &w.replace(ELSE_DEFAULT, "if length == 0 then [] else . end"),
        &["neu-filtern", "merken", "gelb-melden", "eskalation"]
    ));
    assert!(!eigene_default(&w.replacen("eigene: ${ (.input.eigene // []) | if length == 0 then [\"FreeSynergy\"] else . end }\n", "eigene: ${ .input.eigene }\n", 1), &["neu-filtern", "merken", "gelb-melden", "eskalation"]));
    for f in ["koordinator-runde", "bahnen"] {
        let t = read(f);
        let old = format!("{{ (.input.upstream.eigene // []) | {ELSE_DEFAULT} }}");
        assert!(t.contains(&old), "{f}");
        assert!(
            !eigene_default(
                &t.replace(&old, "{ .input.upstream.eigene // [] }"),
                &["upstream-start"]
            ),
            "{f}"
        );
    }
}

#[test]
fn the_whole_state_is_carried_on_after_yellow_and_escalation_and_the_parents_merge() {
    let w = read("upstream-watch");
    assert!(state_carried(&w));
    for f in ["koordinator-runde", "bahnen"] {
        assert!(parent_merges(&read(f)), "{f}");
    }
}

#[test]
fn counter_a_replaced_state_is_caught() {
    // mutant: the state is replaced (the result of the step has only the status, the time and the remembered keys are lost)
    let w = read("upstream-watch");
    for key in STATE_KEYS {
        for step in ["gelb-melden", "eskalation"] {
            let body = step_body(&w, step);
            let line = format!("        {key}\n");
            assert!(body.contains(&line), "{key} in {step}");
            let bad = w.replacen(&body, &body.replacen(&line, "", 1), 1);
            assert_ne!(bad, w);
            assert!(!state_carried(&bad), "{key} missing in {step}");
        }
    }
    assert!(!state_carried(&w.replace(
        "letzte_pruefung: ${ .input.letzte_pruefung // null }",
        "letzte_pruefung: null"
    )));
    assert!(!state_carried(
        &w.replace("`gemerkt` (= `neu.gemerkt`", "`gemerkt` (= nothing")
    ));
    for f in ["koordinator-runde", "bahnen"] {
        let t = read(f);
        assert!(
            !parent_merges(&t.replace("{ upstream: ($context.upstream + .) }", "{ upstream: . }")),
            "{f}"
        );
    }
}

#[test]
fn the_first_run_is_a_baseline() {
    assert!(baseline(&read("upstream-watch")));
}

#[test]
fn counter_a_missing_baseline_is_caught() {
    let w = read("upstream-watch");
    assert!(!baseline(&w.replace("BASELINE", "START")));
    assert!(!baseline(&w.replace("`neu.grundlinie`", "`neu.x`")));
    assert!(!baseline(&w.replace("FIRST RUN = BASELINE", "FIRST RUN")));
}

const SCRIPTS: [&str; 3] = [
    "read-upstream-list",
    "fetch-upstream",
    "filter-new-comments",
];

/// Write words that must not be in the new scripts (the fetch reads with GET only; the method is never set).
const WRITE_WORDS: [&str; 16] = [
    "POST",
    "PATCH",
    "DELETE",
    "PUT ",
    "--method",
    "-X ",
    "\"-X\"",
    "--input",
    "gh issue comment",
    "gh pr comment",
    "gh pr review",
    "gh issue create",
    "gh pr create",
    "gh pr merge",
    "-f body",
    "gh issue close",
];

fn script(name: &str) -> String {
    std::fs::read_to_string(root().join("scripts").join(name)).unwrap()
}

fn read_only(src: &str) -> bool {
    WRITE_WORDS.iter().all(|w| !src.contains(w))
}

/// The one `gh` call is `gh api --paginate <endpoint>` with the endpoint checked against the fixed read pattern.
fn gh_get_only(src: &str) -> bool {
    src.matches("\"gh\"").count() == 1
        && src.contains("[\"gh\", \"api\", \"--paginate\", endpoint]")
        && src.contains("if not ENDPOINT.match(endpoint):")
        && src.contains("(issues/[0-9]+/comments|pulls/[0-9]+(/comments|/reviews)?)$")
}

#[test]
fn the_new_scripts_read_only_and_the_one_gh_call_is_a_checked_get() {
    for s in SCRIPTS {
        assert!(read_only(&script(s)), "{s}");
    }
    assert!(gh_get_only(&script("fetch-upstream")));
    assert!(script("fetch-upstream").contains("Upstream nicht erreichbar"));
    assert!(
        !script("read-upstream-list").contains("subprocess")
            && !script("filter-new-comments").contains("subprocess")
    );
}

#[test]
fn counter_a_write_call_in_a_script_is_caught() {
    let f = script("fetch-upstream");
    for w in [
        "-X POST",
        "--method PATCH",
        "gh api -X DELETE x",
        "gh pr comment 73",
        "--input x.json",
        "-f body=x",
        "gh issue close 3",
        "\"-X\", \"POST\"",
    ] {
        assert!(!read_only(&format!("{f}\n# {w}\n")), "{w}");
    }
    assert!(!gh_get_only(&f.replace(
        "[\"gh\", \"api\", \"--paginate\", endpoint]",
        "[\"gh\", \"api\", \"--paginate\", endpoint, \"x\"]"
    )));
    assert!(!gh_get_only(
        &f.replace("if not ENDPOINT.match(endpoint):", "if False:")
    ));
    assert!(!gh_get_only(&f.replace(
        "pulls/[0-9]+(/comments|/reviews)?",
        "pulls/[0-9]+(/comments|/reviews|/merge)?"
    )));
    assert!(!gh_get_only(&format!(
        "{f}\nsubprocess.run([\"gh\", \"pr\", \"merge\"])\n"
    )));
}

#[test]
fn the_self_tests_of_the_three_scripts_run() {
    for t in [
        "test_read_upstream_list.sh",
        "test_fetch_upstream.sh",
        "test_filter_new_comments.sh",
    ] {
        run_self_test(t);
    }
}
