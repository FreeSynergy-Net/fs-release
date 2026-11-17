use super::*;

#[test]
fn counter_a_bare_force_in_any_workflow_is_caught() {
    for f in FILES {
        let text = read(f);
        let bad = format!("{text}\n# git push --force origin main\n");
        assert!(!bare_force_hits(&bad).is_empty(), "{f}");
    }
    // the allowed forms stay allowed
    assert_eq!(
        bare_force_hits("`--force-with-lease=b:sha`, `--force` alone is refused, never --force"),
        [] as [std::string::String; 0]
    );
    let ff = read("ff-und-tag");
    let mutant = ff.replacen("WITHOUT force", "git push --force", 1);
    assert_ne!(bare_force_hits(&mutant), [] as [std::string::String; 0]);
}

pub const KAT_ORDER: [&str; 18] = [
    "zaehler-start",
    "alt-kopf",
    "kette-weiche",
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
pub fn catalog_flow(text: &str) -> bool {
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
pub fn catalog_loops_and_head(text: &str) -> bool {
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
pub fn review_delta(review: &str) -> bool {
    review.contains("mode DELTA")
        && step_body(review, "verdict").contains("Delta mode - the report names")
}

/// Decision b7: scripts and logs only in the own TMPDIR, never in the shared scratchpad - in the header and in the steps `ci-lesen` and `read`.
pub fn review_own_folder(review: &str) -> bool {
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

/// The expected runs per repo are listed BEFORE the tag and every one must exist and be green for the head; a missing run is red.
pub fn ff_expected_runs(text: &str) -> bool {
    if !text.contains("  - erwartete-laeufe:\n") || !text.contains("  - laeufe-weiter:\n") {
        return false;
    }
    let list = step_body(text, "erwartete-laeufe");
    let next = step_body(text, "laeufe-weiter");
    let green = next.split("- gruen:").nth(1).unwrap_or("");
    let unknown = next.split("- unbekannt:").nth(1).unwrap_or("");
    in_order(text, &["erwartete-laeufe", "laeufe-weiter", "tag"])
        && list.contains("gh run list --commit <head>")
        && list.contains("NOT by an agent")
        && list.contains("EVERY run expected")
        && list.contains("a missing run is RED")
        && list.contains("release.yml")
        && list.contains("`msrv`")
        && list.contains("typing-test")
        && list.contains("head: ${ .input.head }")
        && list.contains("{ laeufe: . }")
        && next.contains("fehlend")
        && next.contains("then: eskalation")
        && green.contains("when: '${ $context.laeufe.status == \"green\" }'")
        && unknown.contains("then: eskalation")
        && next.rfind("- unbekannt:") > next.rfind("- gruen:")
}

#[test]
fn ff_und_tag_checks_every_expected_run_before_the_tag() {
    assert!(ff_expected_runs(&read("ff-und-tag")));
}
