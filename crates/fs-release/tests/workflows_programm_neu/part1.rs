use super::*;

pub(super) const FLOW: &str = "programm-neu";
pub(super) const MAIN: [&str; 19] = [
    "wessen-daten",
    "katalog-suche",
    "standards",
    "kern-crate",
    "umzug",
    "datenmodell",
    "schnittstelle",
    "manifest",
    "engine",
    "grenz-adapter",
    "admin-frontend",
    "vorbild-dossier",
    "frontend",
    "sprache",
    "hub-adapter",
    "sicherheit",
    "pakete",
    "release",
    "abnahme",
];

pub(super) fn position(text: &str, step: &str) -> usize {
    step_names(text)
        .iter()
        .position(|n| n == step)
        .unwrap_or_else(|| panic!("no step {step}"))
}

/// Every step names at least one ADR (`freesynergy.net:adr: '0076, 0061'`) and its phase.
pub(super) fn all_have_adr(text: &str) -> bool {
    step_names(text).iter().all(|s| {
        let b = step_body(text, s);
        b.lines().any(|l| {
            l.trim()
                .strip_prefix("freesynergy.net:adr: '")
                .is_some_and(|v| v.chars().next().is_some_and(|c| c.is_ascii_digit()))
        }) && b.contains("freesynergy.net:phase: P")
    })
}

/// Every objective agent step is followed by a machine step of kind objective (the objective check).
pub(super) fn agent_steps_are_checked(text: &str) -> bool {
    let names = step_names(text);
    names.iter().enumerate().all(|(i, n)| {
        let b = step_body(text, n);
        if !(b.contains("actor: ai") && b.contains("kind: objective")) {
            return true;
        }
        names.get(i + 1).is_some_and(|next| {
            let nb = step_body(text, next);
            nb.contains("actor: machine") && nb.contains("kind: objective")
        })
    })
}

#[test]
fn the_17_steps_come_in_the_order_of_the_phases() {
    let text = read(FLOW);
    let at: Vec<usize> = MAIN.iter().map(|s| position(&text, s)).collect();
    assert!(at.windows(2).all(|p| p[0] < p[1]), "{at:?}");
    let phase = |s: &str| {
        let b = step_body(&text, s);
        let l = b
            .lines()
            .find(|l| l.contains("freesynergy.net:phase: P"))
            .unwrap()
            .to_string();
        l.trim().chars().last().unwrap()
    };
    let phases: Vec<char> = MAIN.iter().map(|s| phase(s)).collect();
    assert!(phases.windows(2).all(|p| p[0] <= p[1]), "{phases:?}");
    assert_eq!((phases[0], phases[18]), ('0', '6'));
}

#[test]
fn the_border_adapter_step_and_the_formats_name_adr_0085() {
    let text = read(FLOW);
    for step in [
        "umzug",
        "datenmodell",
        "grenz-adapter",
        "grenz-check",
        "grenz-again",
    ] {
        assert!(step_body(&text, step).contains("0085"), "{step}");
    }
    // counter-test: the reference removed from that one step only (the other steps keep theirs), the rule then fails
    let adr = |t: &str, s: &str| {
        step_body(t, s)
            .lines()
            .any(|l| l.trim().starts_with("freesynergy.net:adr:") && l.contains("0085"))
    };
    let broken = text.replacen("'0085, 0014, 0046'", "'0014, 0046'", 1);
    assert_ne!(broken, text);
    assert!(adr(&text, "grenz-adapter") && !adr(&broken, "grenz-adapter"));
    assert!(adr(&broken, "umzug"));
}

#[test]
fn counter_two_steps_swapped_are_caught() {
    let text = read(FLOW);
    let swapped = text
        .replacen("  - engine:", "  - zz-tmp:", 1)
        .replacen("  - datenmodell:", "  - engine:", 1)
        .replacen("  - zz-tmp:", "  - datenmodell:", 1);
    let at: Vec<usize> = MAIN.iter().map(|s| position(&swapped, s)).collect();
    assert!(!at.windows(2).all(|p| p[0] < p[1]));
}

#[test]
fn every_step_names_its_adrs_and_a_kind() {
    assert!(all_have_adr(&read(FLOW)));
    assert_red(FLOW, "kind", |t| {
        t.replacen("        freesynergy.net:kind: subjective\n", "", 1)
    });
}

#[test]
fn counter_a_step_without_an_adr_reference_is_caught() {
    let text = read(FLOW);
    let broken = text.replacen("        freesynergy.net:adr: '0076, 0061'\n", "", 1);
    assert_ne!(broken, text);
    assert!(!all_have_adr(&broken));
}

#[test]
fn objective_agent_steps_have_an_objective_script_check() {
    assert!(agent_steps_are_checked(&read(FLOW)));
}

#[test]
fn counter_a_missing_check_step_is_caught() {
    let text = read(FLOW);
    let broken = text.replacen("  - standards-check:", "  - standards-note:", 1).replacen(
        "freesynergy.net:actor: machine\n        freesynergy.net:actor-type: Service\n        freesynergy.net:gate: none\n        freesynergy.net:kind: objective\n        freesynergy.net:phase: P1\n        freesynergy.net:adr: '0014'",
        "freesynergy.net:actor: human\n        freesynergy.net:actor-type: Person\n        freesynergy.net:gate: human\n        freesynergy.net:kind: subjective\n        freesynergy.net:phase: P1\n        freesynergy.net:adr: '0014'",
        1,
    );
    assert_ne!(broken, text);
    assert!(!agent_steps_are_checked(&broken));
}

#[test]
fn red_checks_jump_back_by_switch_and_the_person_runs_real_servers() {
    let text = read(FLOW);
    for (step, target) in [
        ("einordnung-again", "then: wessen-daten"),
        ("standards-again", "then: standards"),
        ("kern-again", "then: kern-crate"),
        ("modell-again", "then: datenmodell"),
        ("schnittstelle-again", "then: datenmodell"),
        ("engine-again", "then: schnittstelle"),
        ("admin-again", "then: admin-frontend"),
        ("sprache-again", "then: sprache"),
        ("adapter-again", "then: hub-adapter"),
        ("sicherheit-again", "then: sicherheit"),
        ("pakete-again", "then: pakete"),
        ("abnahme-again", "then: befunde"),
        ("umzug-again", "then: umzug"),
        ("grenz-again", "then: grenz-adapter"),
        ("grenz-again", "then: datenmodell"),
        ("manifest-again", "then: manifest"),
        ("engine-again", "then: engine"),
        ("katalog-weiter", "then: vorhanden-bericht"),
        ("katalog-weiter", "then: katalog-fehler"),
        ("einordnung-again", "then: eskalation"),
    ] {
        let b = step_body(&text, step);
        assert!(
            b.contains("switch:") && b.contains(target) && b.contains("actor: machine"),
            "{step}"
        );
    }
    let server = step_body(&text, "server-lauf");
    assert!(
        server.contains("actor: human")
            && server.contains("gate: human")
            && server.contains("no agent ever has SSH or root")
    );
    let hub = step_body(&text, "hub-adapter");
    assert!(hub.contains("actor: ai") && hub.contains("nothing on a real server"));
    let abnahme = step_body(&text, "abnahme");
    assert!(abnahme.contains("actor: human") && abnahme.contains("kind: subjective"));
    assert!(step_body(&text, "befunde").contains("name: bugfix"));
}

#[test]
fn facette_and_modul_leave_at_a_todo_stub_and_facette_anlegen_is_not_invented() {
    let text = read(FLOW);
    for stub in ["facette-abzweig", "modul-abzweig", "suite-abzweig"] {
        assert!(ends_the_flow(&text, stub), "{stub}");
        assert!(step_body(&text, stub).contains("actor: human"), "{stub}");
    }
    // every dead end of the flow really ends (a `then: continue` would run on into the next stub)
    for end in ["eskalation", "vorhanden-bericht", "katalog-fehler"] {
        assert!(ends_the_flow(&text, end), "{end}");
    }
    // counter-tests: a stub with `then: continue`, a stub without any `then`
    for stub in ["facette-abzweig", "modul-abzweig", "suite-abzweig"] {
        let body = step_body(&text, stub);
        let cont = text.replacen(
            &body,
            &body.replace("      then: end", "      then: continue"),
            1,
        );
        assert_ne!(cont, text);
        assert!(!ends_the_flow(&cont, stub), "{stub}");
        let none = text.replacen(&body, &body.replace("      then: end\n", ""), 1);
        assert!(!ends_the_flow(&none, stub), "{stub}");
    }
    assert!(step_body(&text, "facette-abzweig").contains("does NOT exist yet"));
    assert!(!text.contains("name: facette-anlegen"));
    assert!(!text.contains("Gesicht"));
    let again = step_body(&text, "einordnung-again");
    for t in [
        "then: facette-abzweig",
        "then: modul-abzweig",
        "then: suite-abzweig",
    ] {
        assert!(again.contains(t), "{t}");
    }
}

#[test]
fn counter_a_jump_to_a_missing_step_is_red() {
    assert_red(FLOW, "no step of this flow", |t| {
        t.replacen("then: facette-abzweig", "then: facette-weg", 1)
    });
}

// ---- structure rules: graph, loops, order, inputs (each with a counter-test; the prose `contains` tests above are no proof of behaviour)

pub(super) fn swapped(text: &str, a: &str, b: &str) -> String {
    text.replacen(&format!("  - {a}:"), "  - zz-tmp:", 1)
        .replacen(&format!("  - {b}:"), &format!("  - {a}:"), 1)
        .replacen("  - zz-tmp:", &format!("  - {b}:"), 1)
}

/// The steps a step can go to: switch cases (12 spaces), else the step `then` (6 spaces), else the next step.
pub(super) fn successors(text: &str, i: usize) -> Vec<usize> {
    let names = step_names(text);
    let body = step_body(text, &names[i]);
    let want = if body.contains("      switch:\n") {
        12
    } else {
        6
    };
    let mut thens: Vec<String> = body
        .lines()
        .filter(|l| l.len() - l.trim_start().len() == want)
        .filter_map(|l| l.trim().strip_prefix("then: ").map(str::to_string))
        .collect();
    if want == 6 && thens.is_empty() {
        thens.push("continue".into());
    }
    let mut out = vec![];
    for t in thens {
        match t.as_str() {
            "continue" if i + 1 < names.len() => out.push(i + 1),
            "continue" | "end" | "exit" => {}
            name => out.push(names.iter().position(|n| n == name).unwrap_or(usize::MAX)),
        }
    }
    out
}
