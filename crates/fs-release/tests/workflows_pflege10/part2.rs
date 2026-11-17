use super::*;

#[test]
fn counter_watch_mutants_are_caught() {
    let t = read("upstream-watch");
    for step in [
        "erreichbar-weiche",
        "abruf-weiche",
        "neu-filtern",
        "karten-anlegen",
        "entwurf-pruefen",
        "freigabe-nutzer",
        "freigabe-weiche",
        "gelb-melden",
    ] {
        let bad = remove(&t, step);
        assert!(!watch(&bad), "{step}");
    }
    // `senden` before the approval of the user
    let (a, b) = (step_block(&t, "senden"), step_block(&t, "freigabe-nutzer"));
    let swapped = t.replacen(&a, "", 1).replacen(&b, &format!("{a}{b}"), 1);
    assert_ne!(swapped, t);
    assert!(!watch(&swapped));
    // the approval done by an agent
    let human = step_body(&t, "freigabe-nutzer");
    assert!(!watch(&t.replacen(
        &human,
        &human.replacen("actor: human", "actor: ai", 1),
        1
    )));
    // a write call anywhere (the whole text and the sending step)
    for w in [
        "gh api -X POST repos/x/issues/1/comments",
        "gh pr comment 73",
        "--method PATCH",
        "gh api -X DELETE x",
    ] {
        let bad = t.replacen(
            "sends nothing itself",
            &format!("sends nothing itself; {w}"),
            1,
        );
        assert!(!watch(&bad), "{w}");
    }
    // a new call that is not on the fixed list
    assert!(!watch(
        &t.replace("call: hand-to-helper:", "call: comment-issue:")
    ));
    assert!(!watch(
        &t.replace("call: fetch-upstream:", "call: post-comment:")
    ));
    // the yellow end: green instead of yellow, or no end, or a switch that no longer goes yellow
    assert!(!watch(&t.replace("status: yellow", "status: green")));
    assert!(!watch(&t.replace("Upstream nicht erreichbar", "ok")));
    assert!(!watch(&t.replace("then: gelb-melden", "then: continue")));
    assert!(!watch(
        &t.replace("$context.erreichbar.ok != true", "false")
    ));
    // the loop without the third round, the new-comment rule weakened, the board rule changed
    assert!(!watch(&t.replace(">= 3", ">= 9")));
    assert!(!watch(&t.replace("without upper/lower case", "exactly")));
    assert!(!watch(&t.replace(
        "OR its id is not in the remembered list",
        "AND nothing"
    )));
    assert!(!watch(
        &t.replace("--project bahn5-crates", "--project other")
    ));
    assert!(!watch(
        &t.replace("at most 5 sentences", "a few paragraphs")
    ));
    assert!(!watch(&t.replacen("      then: end\n", "", 1)));
}

#[test]
fn counter_due_step_mutants_are_caught() {
    for f in ["koordinator-runde", "bahnen"] {
        let t = read(f);
        assert!(due_step(&t), "{f}");
        assert!(!due_step(&remove(&t, "upstream-faellig")), "{f}");
        assert!(!due_step(&remove(&t, "upstream-watch")), "{f}");
        assert!(!due_step(&t.replace("> 7200", "> 3600")), "{f}");
        assert!(
            !due_step(&t.replace("older than 2 hours", "older than a day")),
            "{f}"
        );
        assert!(
            !due_step(&t.replace("== null or (now", "== 1 or (now")),
            "{f}"
        );
        assert!(
            !due_step(&t.replace("then: upstream-watch", "then: continue")),
            "{f}"
        );
        assert!(
            !due_step(&t.replace("Upstream nicht erreichbar", "all fine")),
            "{f}"
        );
        assert!(
            !due_step(&t.replace("name: upstream-watch", "name: other")),
            "{f}"
        );
        // a write call in the step text
        let bad = t.replacen(
            "never silently green; the result",
            "never silently green; gh api -X POST x; the result",
            1,
        );
        assert_ne!(bad, t, "{f}");
        assert!(!due_step(&bad), "{f}");
    }
}

#[test]
fn the_rule_of_the_issues_stays_in_upstream_anfrage_and_the_watch_names_it() {
    let t = read("upstream-watch");
    assert!(t.contains("no new issues while our own pull requests are open"));
    let u = read("upstream-anfrage");
    assert!(u.contains("NEVER a pull request and NEVER a comment"));
}

pub(super) fn step_block(text: &str, step: &str) -> String {
    let head = format!("  - {step}:\n");
    let start = text.find(&head).unwrap();
    let rest = &text[start + head.len()..];
    let end = rest.find("\n  - ").map_or(rest.len(), |i| i + 1);
    format!("{head}{}", &rest[..end])
}

pub(super) fn remove(text: &str, step: &str) -> String {
    text.replacen(&step_block(text, step), "", 1)
}
