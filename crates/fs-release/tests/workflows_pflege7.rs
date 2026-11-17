//! Pattern: none (test). The workflows of the maintenance run 1: `ff-und-tag`, `branch-neu-schreiben`, `katalog`, the step
//! `version-abgleich` in `release-flow`, the step `ci-lesen` and the delta mode in `review`. Each rule has a counter-test on a changed copy.

mod support;
use support::workflows::{in_order, read, step_body, step_names};

/// Swaps the names of two steps (the bodies stay): the same flow with the two steps in the other order.
fn swapped(text: &str, a: &str, b: &str) -> String {
    text.replacen(&format!("  - {a}:"), "  - zz-tmp:", 1)
        .replacen(&format!("  - {b}:"), &format!("  - {a}:"), 1)
        .replacen("  - zz-tmp:", &format!("  - {b}:"), 1)
}

const FF_ORDER: [&str; 15] = [
    "zweig-ci",
    "secrets-pruefen",
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
    "aufraeumen",
];

/// CI of all workflows first, then checks on a fresh clone, then the approval for the head, then push and tag by a person, then proof.
fn ff_flow(text: &str) -> bool {
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
fn ff_rules(text: &str) -> bool {
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
    ] {
        assert!(!ff_rules(&text.replace(phrase, "x")), "{phrase}");
    }
}

const BNS_ORDER: [&str; 12] = [
    "alter-kopf",
    "ketten-plan",
    "neu-schreiben",
    "rebase",
    "identitaet-nach",
    "inhaltsgleich",
    "gates",
    "lease-pruefen",
    "push-lease",
    "ls-remote-beleg",
    "neues-ja",
    "kette-weiter",
];

/// Lease only (never a bare force), checked before the push, the push is a person's, equal content is checked, a new head needs a new JA.
fn rewrite_flow(text: &str) -> bool {
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
    // no step may push with a bare `--force`
    assert!(!text.contains("--force "), "bare --force");
    assert!(!text.contains("--force\""), "bare --force");
}

const KAT_ORDER: [&str; 10] = [
    "eintrag",
    "asset-pruefen",
    "gen-catalog",
    "frische-pruefung",
    "ff-ueber-strategie",
    "sign-abwarten",
    "catalog-beleg",
    "raw-cache",
    "kette-weiter",
    "sign-neu",
];

/// Asset check (not for a removal), the three generator checks, the fresh review, the ff by a person, the signed proof; the raw cache rule;
/// `catalog-bump` stays the event path and is not called.
fn catalog_flow(text: &str) -> bool {
    in_order(text, &KAT_ORDER)
        && step_body(text, "asset-pruefen").contains("if: ${ .input.modus != \"entfernen\" }")
        && step_body(text, "asset-pruefen").contains("NEVER a draft")
        && ["gen_catalog.py --check", "unittest", "py_compile"]
            .iter()
            .all(|p| step_body(text, "gen-catalog").contains(p))
        && step_body(text, "frische-pruefung").contains("name: review")
        && step_body(text, "ff-ueber-strategie").contains("actor: human")
        && step_body(text, "sign-abwarten").contains("packages.toml")
        && step_body(text, "raw-cache").contains("does not match the given sha256")
        && step_body(text, "raw-cache").contains("5 minutes")
        && step_body(text, "kette-rebase").contains("BOTH `## Unreleased")
        && !text.contains("name: catalog-bump")
}

#[test]
fn katalog_checks_before_the_ff_and_proves_after_the_signing() {
    assert!(catalog_flow(&read("katalog")));
    assert!(step_names(&read("catalog-bump")).contains(&"bump".to_string()));
}

#[test]
fn counter_katalog_rule_breaks_are_caught() {
    let text = read("katalog");
    assert!(!catalog_flow(&swapped(
        &text,
        "gen-catalog",
        "ff-ueber-strategie"
    )));
    assert!(!catalog_flow(&swapped(
        &text,
        "catalog-beleg",
        "sign-abwarten"
    )));
    for phrase in [
        "NEVER a draft",
        "py_compile",
        "does not match the given sha256",
        "5 minutes",
        "BOTH `## Unreleased",
        "packages.toml or scripts",
    ] {
        assert!(!catalog_flow(&text.replace(phrase, "x")), "{phrase}");
    }
    let calls = text.replacen(
        "  - eintrag:",
        "  - x:\n      run:\n        workflow:\n          name: catalog-bump\n  - eintrag:",
        1,
    );
    assert!(!catalog_flow(&calls));
}

#[test]
fn release_flow_compares_versions_before_the_review_and_review_reads_the_ci() {
    let flow = read("release-flow");
    assert!(in_order(
        &flow,
        &[
            "lock-pruefung",
            "version-abgleich",
            "adresse-pruefen",
            "review-request"
        ]
    ));
    let body = step_body(&flow, "version-abgleich");
    for p in [
        "Cargo.toml",
        "Cargo.lock",
        "CHANGELOG.md",
        "NEWEST chain",
        "v0.0.10",
    ] {
        assert!(body.contains(p), "{p}");
    }
    let review = read("review");
    assert!(in_order(
        &review,
        &["request", "ci-lesen", "read", "verdict", "bind"]
    ));
    assert!(step_body(&review, "ci-lesen").contains("ALL workflows"));
    assert!(review.contains("mode DELTA") && review.contains("Delta mode - the report names"));
}

#[test]
fn counter_a_late_version_check_or_a_missing_ci_step_is_caught() {
    let flow = read("release-flow");
    let late = swapped(&flow, "version-abgleich", "review-request");
    assert!(!in_order(
        &late,
        &["lock-pruefung", "version-abgleich", "review-request"]
    ));
    let review = read("review");
    assert!(!in_order(
        &swapped(&review, "ci-lesen", "read"),
        &["ci-lesen", "read"]
    ));
    assert!(!review.replace("mode DELTA", "").contains("mode DELTA"));
}
