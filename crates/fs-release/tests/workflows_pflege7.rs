//! Pattern: none (test). The workflows of the maintenance run 1: `ff-und-tag`, `branch-neu-schreiben`, `katalog`, the step
//! `version-abgleich` in `release-flow`, the step `ci-lesen` and the delta mode in `review`. Each rule has a counter-test on a changed copy.

mod support;
use support::workflows::{in_order, read, step_body, step_names, FILES};

/// Swaps the names of two steps (the bodies stay): the same flow with the two steps in the other order.
fn swapped(text: &str, a: &str, b: &str) -> String {
    text.replacen(&format!("  - {a}:"), "  - zz-tmp:", 1)
        .replacen(&format!("  - {b}:"), &format!("  - {a}:"), 1)
        .replacen("  - zz-tmp:", &format!("  - {b}:"), 1)
}

const FF_ORDER: [&str; 17] = [
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
    "fertig",
    "eskalation",
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
        && ff_head_bound(text)
        && ff_ci_only_green(text)
        && ff_escalation_tidies(text)
}

/// The approval and the push are bound to the head of the order (a JA for another head is void).
fn ff_head_bound(text: &str) -> bool {
    step_body(text, "ja-fuer-kopf").contains("head: ${ .input.head }")
        && step_body(text, "tag").contains("head: ${ .input.head }")
        && step_body(text, "ja-fuer-kopf").contains(
            "No tag as long as a delta or re-look result for this head has an open condition",
        )
        && step_body(text, "ja-fuer-kopf")
            .contains("line 1 of the Opus result file must be `JA` for EXACTLY this head")
        && step_body(text, "zweig-ci").contains("head: ${ .input.head }")
}

/// ONLY `status == green` passes the CI check: the case `gruen` has its `when`, every other status goes to the escalation.
fn ff_ci_only_green(text: &str) -> bool {
    let wait = step_body(text, "ci-weiter");
    let green = wait.split("- gruen:").nth(1).unwrap_or("");
    let unknown = wait.split("- unbekannt:").nth(1).unwrap_or("");
    green.contains("when: '${ $context.ci.status == \"green\" }'")
        && unknown.contains("then: eskalation")
        && wait.rfind("- unbekannt:") > wait.rfind("- gruen:")
}

/// The escalation ends with `eskaliert: true` and goes through the tidy-up, which removes only a clone made by this run; the output has the proof.
fn ff_escalation_tidies(text: &str) -> bool {
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

const BNS_ORDER: [&str; 14] = [
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
        && chain_on_current(text)
}

/// Chain mode: only `ketten-plan` reads the branch of the order; every other step works on `kette.aktuell`; `ketten-naechster` sets `alt` (the lease);
/// a NEIN of the new JA goes to the escalation and never to the ff of the earlier branch.
fn chain_on_current(text: &str) -> bool {
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
fn bare_force_hits(text: &str) -> Vec<String> {
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

#[test]
fn counter_a_bare_force_in_any_workflow_is_caught() {
    for f in FILES {
        let text = read(f);
        let bad = format!("{text}\n# git push --force origin main\n");
        assert!(!bare_force_hits(&bad).is_empty(), "{f}");
    }
    // the allowed forms stay allowed
    assert!(bare_force_hits(
        "`--force-with-lease=b:sha`, `--force` alone is refused, never --force"
    )
    .is_empty());
    let ff = read("ff-und-tag");
    let mutant = ff.replacen("WITHOUT force", "git push --force", 1);
    assert!(!bare_force_hits(&mutant).is_empty());
}

const KAT_ORDER: [&str; 17] = [
    "zaehler-start",
    "alt-kopf",
    "eintrag",
    "asset-pruefen",
    "gen-catalog",
    "frische-pruefung",
    "pruefung-weiter",
    "ff-ueber-strategie",
    "sign-abwarten",
    "catalog-beleg",
    "raw-cache",
    "kette-weiter",
    "ketten-naechster",
    "kette-rebase",
    "gen-zaehlen",
    "sign-zaehlen",
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
        && catalog_loops_and_head(text)
}

/// Every step works on `kette.aktuell` (only `ketten-plan` reads the head of the order), the review gets the NEW head, a NEIN never goes on,
/// the two loops are counted and end in the escalation which is in the output.
fn catalog_loops_and_head(text: &str) -> bool {
    let others = text.replace(&step_body(text, "ketten-plan"), "");
    let review = step_body(text, "frische-pruefung");
    let next = step_body(text, "pruefung-weiter");
    let gen = step_body(text, "gen-rot");
    let cache = step_body(text, "raw-cache");
    let out = text.split("\noutput:\n").nth(1).unwrap_or("");
    !others.contains("${ .input.head }")
        && !others.contains("${ .input.program }")
        && !others.contains("${ .input.tag }")
        && !others.contains("${ .input.branch }")
        && review.contains("head: ${ $context.kette.aktuell.head }")
        && review.contains("branch: ${ $context.kette.aktuell.branch }")
        && step_body(text, "kette-rebase").contains("{ kette: . }")
        && next.contains("then: eintrag")
        && next.contains("then: eskalation")
        && !next.contains("kette-weiter")
        && !next.contains("ff-ueber-strategie")
        && gen.contains("$context.zaehler.gen >= 3")
        && gen.contains("then: gen-zaehlen")
        && gen.contains("then: eskalation")
        && step_body(text, "gen-zaehlen").contains("$context.zaehler.gen + 1")
        && step_body(text, "gen-zaehlen").contains("then: eintrag")
        && cache.contains("$context.zaehler.sign >= 1")
        && cache.contains("then: sign-zaehlen")
        && cache.contains("then: eskalation")
        && step_body(text, "sign-zaehlen").contains("$context.zaehler.sign + 1")
        && step_body(text, "ketten-naechster").contains("then: zaehler-start")
        && step_body(text, "alt-kopf").contains("$context.kette.aktuell.branch")
        && step_body(text, "eskalation").contains("eskaliert: true")
        && step_body(text, "eskalation").contains("status: escalated")
        && out.contains("eskaliert:")
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
        "head: ${ $context.kette.aktuell.head }",
        "$context.zaehler.gen >= 3",
        "$context.zaehler.sign >= 1",
        "$context.zaehler.gen + 1",
        "then: gen-zaehlen",
        "then: sign-zaehlen",
        "eskaliert: true",
        "{ kette: . }",
    ] {
        assert!(!catalog_flow(&text.replace(phrase, "x")), "{phrase}");
    }
    // the review on the head of the order instead of the new head after the rebase; no head at all
    let review = step_body(&text, "frische-pruefung");
    let order_head = review.replace("$context.kette.aktuell.head", ".input.head");
    assert!(
        !catalog_flow(&text.replace(&review, &order_head)),
        "order head"
    );
    let none = review.replace("          head: ${ $context.kette.aktuell.head }\n", "");
    assert!(!catalog_flow(&text.replace(&review, &none)), "no head");
    // a NEIN that goes on to the ff
    let next = step_body(&text, "pruefung-weiter");
    let nein = next.replace("then: eintrag", "then: ff-ueber-strategie");
    assert!(!catalog_flow(&text.replace(&next, &nein)), "NEIN goes on");
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
    assert!(review_delta(&review));
    assert!(review_own_folder(&review));
}

/// The delta mode is described in the header AND in the step `verdict`, the CI step reads all workflows.
fn review_delta(review: &str) -> bool {
    review.contains("mode DELTA")
        && step_body(review, "verdict").contains("Delta mode - the report names")
}

/// Decision b7: scripts and logs only in the own TMPDIR, never in the shared scratchpad - in the header and in the steps `ci-lesen` and `read`.
fn review_own_folder(review: &str) -> bool {
    let head = review.split("\ndocument:").next().unwrap_or("");
    let sentence = "only in the own TMPDIR under a unique name, never in the shared scratchpad";
    head.contains("[EIGENES-SKRIPT-EIGENER-ORDNER]")
        && head.contains(sentence)
        && ["ci-lesen", "read"].iter().all(|s| {
            step_body(review, s).contains("[EIGENES-SKRIPT-EIGENER-ORDNER]")
                && step_body(review, s).contains(sentence)
        })
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
    assert!(!review_delta(&review.replace("mode DELTA", "mode X")));
    assert!(!review_delta(
        &review.replace("Delta mode - the report names", "The report names")
    ));
    assert!(!review_own_folder(
        &review.replace("[EIGENES-SKRIPT-EIGENER-ORDNER]", "[X]")
    ));
    assert!(!review_own_folder(
        &review.replace("never in the shared scratchpad", "anywhere")
    ));
}
