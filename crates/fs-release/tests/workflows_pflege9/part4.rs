use super::*;

#[test]
fn counter_request_missing_results_are_red_and_calls_are_a_fixed_list() {
    let t = read("upstream-anfrage");
    assert!(request(&t));
    let sneaky = "  - pr:\n      call: open-change:0.0.18@fs\n      metadata:\n        freesynergy.net:summary: \"opens a pull request\"\n  - record:\n";
    assert!(!request(&t.replacen("  - record:\n", sneaky, 1)));
    assert!(!request(&t.replace("{ dubletten: . }", "{ x: . }")));
    assert!(!request(
        &t.replace("$context.dubletten.treffer == null or ", "")
    ));
    assert!(!request(&t.replace("$context.rules.hits == null or ", "")));
    assert!(!request(
        &t.replace("$context.textcheck.hits == null or ", "")
    ));
    assert!(!request(
        &t.replace("without a YES, by rule 98", "after the YES")
    ));
}

#[test]
fn counter_pull_request_is_red_until_answer_or_time_limit_and_only_the_human_sends() {
    let t = read("upstream-anfrage");
    for w in ["pr-nach-antwort", "pr-weiche", "pr-rot", "frist_tage // 14"] {
        assert!(t.contains(w), "{w}");
    }
    assert!(has(&t, "pr-freigabe", &["actor: human", "gate: human"]));
    assert!(has(&t, "pr-hand-text", &["actor: human", "never call gh"]));
    assert!(has(&t, "pr-nach-antwort", &["ONE factual follow-up"]));
    assert!(!has(
        &t.replace("actor: human", "actor: ai"),
        "pr-freigabe",
        &["actor: human"]
    ));
}
