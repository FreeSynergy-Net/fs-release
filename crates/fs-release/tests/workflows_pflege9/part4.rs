use super::*;

#[test]
fn counter_request_missing_results_are_red_and_calls_are_a_fixed_list() {
    let t = read("upstream-anfrage");
    assert!(request(&t));
    let sneaky = "  - pr:\n      call: open-change:0.0.17@fs\n      metadata:\n        freesynergy.net:summary: \"opens a pull request\"\n  - record:\n";
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
