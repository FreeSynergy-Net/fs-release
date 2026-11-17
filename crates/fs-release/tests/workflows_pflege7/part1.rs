use super::*;

pub const FF_ORDER: [&str; 19] = [
    "zweig-ci",
    "secrets-pruefen",
    "erwartete-laeufe",
    "laeufe-weiter",
    "klon",
    "kopf-und-ff",
    "identitaet",
    "version",
    "tag-frei",
    "diff-pfade",
    "ja-fuer-kopf",
    "push-ff",
    "tag",
    "ls-remote-beleg",
    "release-beobachten",
    "release-weiter",
    "fertig",
    "eskalation",
    "aufraeumen",
];

/// CI of all workflows first, then checks on a fresh clone, then the approval for the head, then push and tag by a person, then proof.
pub fn ff_flow(text: &str) -> bool {
    let humans = ["push-ff", "tag"].iter().all(|s| {
        step_body(text, s).contains("actor: human") && step_body(text, s).contains("gate: human")
    });
    in_order(text, &FF_ORDER)
        && humans
        && step_body(text, "ja-fuer-kopf").contains("gate: opus-ja")
        && step_body(text, "push-ff").contains("WITHOUT force")
        && step_body(text, "tag").contains("ANNOTATED")
        && step_body(text, "zweig-ci").contains("ALL workflows")
}

#[test]
fn ff_und_tag_follows_the_checklist_and_the_person_pushes() {
    assert!(ff_flow(&read("ff-und-tag")));
}

#[test]
fn counter_ff_und_tag_rule_breaks_are_caught() {
    let text = read("ff-und-tag");
    assert!(!ff_flow(&swapped(&text, "klon", "push-ff")));
    assert!(!ff_flow(&swapped(&text, "ja-fuer-kopf", "tag")));
    assert!(!ff_flow(&text.replace("WITHOUT force", "")));
    assert!(!ff_flow(&text.replace("ANNOTATED", "light")));
    assert!(!ff_flow(&text.replace("ALL workflows", "one workflow")));
    let agent = step_body(&text, "push-ff").replace("actor: human", "actor: machine");
    assert!(!ff_flow(
        &text.replace(&step_body(&text, "push-ff"), &agent)
    ));
}

/// A hanging CI: 30 minutes, ONE restart, then escalation; the lock and version lesson and the secrets names are in the checklist.
pub fn ff_rules(text: &str) -> bool {
    let wait = step_body(text, "ci-weiter");
    let hang = step_body(text, "ci-neustart");
    wait.contains("haengt-erstmals")
        && wait.contains("then: ci-neustart")
        && wait.contains("haengt-wieder")
        && hang.contains("30 minutes")
        && hang.contains("ONCE")
        && hang.contains("then: ci-weiter")
        && step_body(text, "secrets-pruefen").contains("GPG_PRIVATE_KEY")
        && step_body(text, "secrets-pruefen").contains("GPG_PASSPHRASE")
        && step_body(text, "version").contains("Cargo.lock")
        && step_body(text, "version").contains("CHANGELOG")
        && step_body(text, "identitaet").contains("FS_ALLOWED_IDENTITY")
        && step_body(text, "ls-remote-beleg").contains("^{}")
        && step_body(text, "release-beobachten").contains("not a draft")
        && ff_head_bound(text)
        && ff_ci_only_green(text)
        && ff_escalation_tidies(text)
}

/// The approval and the push are bound to the head of the order (a JA for another head is void).
pub fn ff_head_bound(text: &str) -> bool {
    step_body(text, "ja-fuer-kopf").contains("head: ${ .input.head }")
        && step_body(text, "tag").contains("head: ${ .input.head }")
        && step_body(text, "ja-fuer-kopf").contains(
            "No tag as long as a delta or re-look result for this head has an open condition",
        )
        && step_body(text, "ja-fuer-kopf")
            .contains("line 1 of the Opus result file must be `JA` for EXACTLY this head")
        && step_body(text, "ja-fuer-kopf").contains("a later NEIN or condition blocks the tag")
        && step_body(text, "zweig-ci").contains("head: ${ .input.head }")
}

/// ONLY `status == green` passes the CI check: the case `gruen` has its `when`, every other status goes to the escalation.
pub fn ff_ci_only_green(text: &str) -> bool {
    let wait = step_body(text, "ci-weiter");
    let green = wait.split("- gruen:").nth(1).unwrap_or("");
    let unknown = wait.split("- unbekannt:").nth(1).unwrap_or("");
    green.contains("when: '${ $context.ci.status == \"green\" }'")
        && unknown.contains("then: eskalation")
        && wait.rfind("- unbekannt:") > wait.rfind("- gruen:")
}

/// The escalation ends with `eskaliert: true` and goes through the tidy-up, which removes only a clone made by this run; the output has the proof.
pub fn ff_escalation_tidies(text: &str) -> bool {
    let out = text.split("\noutput:\n").nth(1).unwrap_or("");
    step_body(text, "eskalation").contains("eskaliert: true")
        && step_body(text, "eskalation").contains("then: aufraeumen")
        && step_body(text, "fertig").contains("then: aufraeumen")
        && step_body(text, "aufraeumen").contains("created: ${ $context.klon != null }")
        && step_body(text, "klon").contains("{ klon: . }")
        && step_body(text, "ls-remote-beleg").contains("{ beleg: . }")
        && out.contains("eskaliert:")
        && out.contains("beleg:")
}

#[test]
fn ff_und_tag_has_the_hang_rule_and_the_lessons() {
    assert!(ff_rules(&read("ff-und-tag")));
}

#[test]
fn counter_a_missing_ff_rule_is_caught() {
    let text = read("ff-und-tag");
    for phrase in [
        "haengt-erstmals",
        "30 minutes",
        "ONCE",
        "GPG_PASSPHRASE",
        "Cargo.lock",
        "FS_ALLOWED_IDENTITY",
        "not a draft",
        "head: ${ .input.head }",
        "a later NEIN or condition blocks the tag",
        "created: ${ $context.klon != null }",
        "then: aufraeumen",
        "{ beleg: . }",
        "eskaliert:",
    ] {
        assert!(!ff_rules(&text.replace(phrase, "x")), "{phrase}");
    }
    // `gruen` without its `when`: every status that is not red or hung would pass
    let open = text.replace(
        "            when: '${ $context.ci.status == \"green\" }'\n",
        "",
    );
    assert!(!ff_rules(&open), "gruen without when");
    // the approval without the head of the order
    let body = step_body(&text, "ja-fuer-kopf");
    let unbound = body.replace("        head: ${ .input.head }\n", "");
    assert!(!ff_rules(&text.replace(&body, &unbound)), "ja without head");
}

pub const BNS_ORDER: [&str; 14] = [
    "ketten-plan",
    "alter-kopf",
    "neu-schreiben",
    "rebase",
    "identitaet-nach",
    "inhaltsgleich",
    "gates",
    "lease-pruefen",
    "push-lease",
    "ls-remote-beleg",
    "neues-ja",
    "ja-weiter",
    "kette-weiter",
    "vorgaenger-ff",
];

/// Lease only (never a bare force), checked before the push, the push is a person's, equal content is checked, a new head needs a new JA.
pub fn rewrite_flow(text: &str) -> bool {
    let lease = step_body(text, "push-lease");
    in_order(text, &BNS_ORDER)
        && lease.contains("actor: human")
        && lease.contains("--force-with-lease")
        && step_body(text, "lease-pruefen").contains("unreviewed/*")
        && step_body(text, "neues-ja").contains("gate: opus-ja")
        && step_body(text, "neues-ja").contains("NEW head")
        && step_body(text, "inhaltsgleich").contains("allow")
        && step_body(text, "identitaet-nach").contains("ONE line")
        && step_body(text, "rebase").contains("--reset-author")
        && step_body(text, "rebase").contains("COUNTED again")
        && step_body(text, "rebase").contains("ONE `## Unreleased`")
        && chain_on_current(text)
}

/// Chain mode: only `ketten-plan` reads the branch of the order; every other step works on `kette.aktuell`; `ketten-naechster` sets `alt` (the lease);
/// a NEIN of the new JA goes to the escalation and never to the ff of the earlier branch.
pub fn chain_on_current(text: &str) -> bool {
    let others = text.replace(&step_body(text, "ketten-plan"), "");
    let ja = step_body(text, "ja-weiter");
    !others.contains("${ .input.branch }")
        && step_body(text, "ketten-naechster").contains("then: alter-kopf")
        && step_body(text, "lease-pruefen").contains("$context.alt.head")
        && step_body(text, "push-lease").contains("$context.kette.aktuell.branch")
        && step_body(text, "neues-ja").contains("$context.kette.aktuell.branch")
        && step_body(text, "rebase").contains("$context.kette.aktuell.branch")
        && step_body(text, "rebase").contains("lease: ${ $context.alt.head }")
        && ja.contains("then: continue")
        && ja.contains("then: eskalation")
        && !ja.contains("vorgaenger-ff")
}

#[test]
fn branch_neu_schreiben_pushes_only_with_the_lease() {
    assert!(rewrite_flow(&read("branch-neu-schreiben")));
}

#[test]
fn counter_branch_neu_schreiben_rule_breaks_are_caught() {
    let text = read("branch-neu-schreiben");
    assert!(!rewrite_flow(&swapped(
        &text,
        "lease-pruefen",
        "push-lease"
    )));
    for phrase in [
        "--force-with-lease",
        "unreviewed/*",
        "NEW head",
        "--reset-author",
        "COUNTED again",
        "ONE `## Unreleased`",
        "ONE line",
    ] {
        assert!(!rewrite_flow(&text.replace(phrase, "x")), "{phrase}");
    }
    // the chain steps on the old head or the order's branch, the NEIN that goes on: all caught
    let order = text.replace(
        "branch: ${ $context.kette.aktuell.branch }",
        "branch: ${ .input.branch }",
    );
    assert!(!rewrite_flow(&order), "order branch in the chain");
    assert!(
        !rewrite_flow(&text.replace("then: alter-kopf", "then: modus")),
        "no alt"
    );
    assert!(
        !rewrite_flow(&text.replacen(
            "        - nein:\n            then: eskalation",
            "        - nein:\n            then: vorgaenger-ff",
            1
        )),
        "NEIN goes on"
    );
}

/// A bare `--force` (not `--force-with-lease`) in a step text: allowed only as a prohibition ("`--force` alone is refused", "never --force").
pub fn bare_force_hits(text: &str) -> Vec<String> {
    let mut hits = vec![];
    for (i, _) in text.match_indices("--force") {
        let after = &text[i + 7..];
        let before = &text[..i];
        let lease = after.starts_with("-with-lease");
        let ban = after.starts_with("` alone is refused")
            || after.starts_with(" alone is refused")
            || before.ends_with("never ")
            || before.ends_with("never `");
        if !lease && !ban {
            hits.push(text[i.saturating_sub(30)..(i + 30).min(text.len())].replace('\n', " "));
        }
    }
    hits
}

#[test]
fn no_workflow_pushes_with_a_bare_force() {
    for f in FILES {
        let hits = bare_force_hits(&read(f));
        assert!(hits.is_empty(), "{f}: bare --force: {hits:?}");
    }
}
