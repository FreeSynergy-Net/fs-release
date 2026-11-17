//! Test support for `pin-kaskade`: the graph of the Glieder (edges, order) and the switch cases, checked one by one.

use super::workflows::step_body;

/// The real dependencies of the Glieder (Cargo.toml of main of each repo, read 17.11.2026; the owner repo of `fs-conformance` is fs-types, of
/// `fs-people-directory` fs-people). The workflow keeps its own copy in `# DAG:` lines; both must be equal and the order a topological one.
pub const EDGES: &[(&str, &[&str])] = &[
    ("fs-interfaces", &["fs-types"]),
    ("fs-objects", &["fs-types"]),
    ("fs-jscalendar", &["fs-types"]),
    (
        "fs-rpc",
        &["fs-types", "fs-interfaces", "fs-objects", "fs-conformance"],
    ),
    ("fs-components", &["fs-types", "fs-conformance"]),
    (
        "fs-people",
        &["fs-interfaces", "fs-objects", "fs-components"],
    ),
    (
        "fs-tasks",
        &[
            "fs-interfaces",
            "fs-objects",
            "fs-components",
            "fs-jscalendar",
            "fs-people",
        ],
    ),
    (
        "fs-calendars",
        &[
            "fs-types",
            "fs-interfaces",
            "fs-objects",
            "fs-rpc",
            "fs-jscalendar",
            "fs-components",
            "fs-tasks",
        ],
    ),
];

/// The `rev` pins to FreeSynergy repos the header names (`# REV-PIN: <Glied> -> <owner repo> rev <rev> ...`), read from the Cargo.toml of main 17.11.2026.
pub const REV_PINS: &[&str] = &[
    "fs-rpc -> fs-types rev 339aa24f",
    "fs-components -> fs-types rev 339aa24f",
    "fs-tasks -> fs-people rev 358a8145",
    "fs-calendars -> fs-components rev 9950b51",
];

/// The `@fs` calls of `pin-kaskade` that are real scripts in `scripts/`.
pub const SCRIPTS: &[&str] = &[
    "kaskade-lage",
    "kaskade-plan",
    "kaskade-glied",
    "kaskade-glied-fertig",
    "kaskade-fremde-pins",
    "lock-gleich",
    "gates-komplett",
];

/// The text after `prefix` of every line of `text` that starts with it.
pub fn list_lines(text: &str, prefix: &str) -> Vec<String> {
    text.lines()
        .filter_map(|l| l.strip_prefix(prefix))
        .map(str::to_string)
        .collect()
}

pub fn list(text: &str, prefix: &str) -> Vec<String> {
    let l = text
        .lines()
        .find_map(|l| l.strip_prefix(prefix))
        .unwrap_or("");
    l.split(',')
        .map(|x| x.trim().to_string())
        .filter(|x| !x.is_empty())
        .collect()
}

/// The `# DAG:` lines equal `EDGES` and `# GLIEDER-ORDER:` is a topological order of them (a dependency that is a Glied stands before its user).
pub fn graph_ok(t: &str) -> bool {
    let order = list(t, "# GLIEDER-ORDER: ");
    let dag: Vec<(String, Vec<String>)> = t
        .lines()
        .filter_map(|l| l.strip_prefix("# DAG: "))
        .map(|l| {
            let (a, b) = l.split_once(" <- ").unwrap_or((l, ""));
            (
                a.trim().to_string(),
                b.split(',').map(|x| x.trim().to_string()).collect(),
            )
        })
        .collect();
    let same = dag.len() == EDGES.len()
        && EDGES.iter().all(|(r, d)| {
            dag.iter()
                .any(|(r2, d2)| r2 == r && d2.iter().map(String::as_str).eq(d.iter().copied()))
        });
    let mut names: Vec<&str> = EDGES.iter().map(|(r, _)| *r).collect();
    names.sort_unstable();
    let mut o: Vec<&str> = order.iter().map(String::as_str).collect();
    o.sort_unstable();
    let topo = EDGES.iter().all(|(r, d)| {
        let at = order.iter().position(|x| x == r);
        at.is_some()
            && d.iter().all(|x| {
                order
                    .iter()
                    .position(|y| y == x)
                    .is_none_or(|i| i < at.unwrap_or(0))
            })
    });
    same && names == o && topo && order.first().map(String::as_str) == Some("fs-interfaces")
}

/// The cases of one `switch` step as (name, when, then), in order (A2: every case is checked one by one, not by `contains`).
pub fn cases(t: &str, step: &str) -> Vec<(String, String, String)> {
    let mut out: Vec<(String, String, String)> = Vec::new();
    for l in step_body(t, step).lines() {
        if let Some(n) = l
            .strip_prefix("        - ")
            .and_then(|x| x.strip_suffix(':'))
        {
            out.push((n.into(), String::new(), String::new()));
        } else if let (Some(c), Some(w)) = (out.last_mut(), l.strip_prefix("            when: ")) {
            c.1 = w.into();
        } else if let (Some(c), Some(w)) = (out.last_mut(), l.strip_prefix("            then: ")) {
            c.2 = w.into();
        }
    }
    out
}

/// Every switch of the flow: the one green case goes on, EVERY other case goes to `eskalation` (or back to the retry step).
pub const SWITCHES: &[(&str, &[(&str, &str)])] = &[
    (
        "vorhanden-pruefen-again",
        &[
            ("runden", "eskalation"),
            ("red", "vorhanden-pruefen"),
            ("green", "continue"),
            ("ohne-ergebnis", "eskalation"),
        ],
    ),
    (
        "plan-again",
        &[
            ("runden", "eskalation"),
            ("red", "plan"),
            ("green", "continue"),
            ("ohne-ergebnis", "eskalation"),
        ],
    ),
    (
        "glied-weiche",
        &[
            ("fertig", "lock-ende"),
            ("schleife", "eskalation"),
            ("weiter", "glied-start"),
            ("ohne-ergebnis", "eskalation"),
        ],
    ),
    (
        "rev-pins-weiter",
        &[
            ("befund", "eskalation"),
            ("ohne-befund", "continue"),
            ("ohne-ergebnis", "eskalation"),
        ],
    ),
    (
        "windows-weiter",
        &[
            ("rot", "eskalation"),
            ("gruen", "continue"),
            ("ohne-ergebnis", "eskalation"),
        ],
    ),
    (
        "gates-weiter",
        &[
            ("rot", "eskalation"),
            ("gruen", "continue"),
            ("ohne-ergebnis", "eskalation"),
        ],
    ),
    (
        "ci-weiter",
        &[("gruen", "review"), ("rot-oder-unbekannt", "eskalation")],
    ),
    (
        "review-weiter",
        &[("ja", "ff-tag"), ("nein-oder-offen", "eskalation")],
    ),
    (
        "ff-weiter",
        &[("fertig", "naechstes"), ("sonst", "eskalation")],
    ),
    (
        "lock-ende-weiter",
        &[("gruen", "bericht"), ("rot-oder-unbekannt", "eskalation")],
    ),
];

pub fn switches_ok(t: &str) -> bool {
    SWITCHES.iter().all(|(step, want)| {
        let got = cases(t, step);
        got.len() == want.len()
            && got.iter().zip(*want).all(|(g, w)| g.0 == w.0 && g.2 == w.1)
            && match *step {
                "vorhanden-pruefen-again" | "plan-again" => {
                    got[0].1.contains(">= 3") && got[0].1.contains("== true")
                }
                "gates-weiter" => {
                    got[1].1.contains("lockcheck.red == false")
                        && got[1].1.contains("bau.red == false")
                }
                "windows-weiter" => got[1].1.contains("windows.red == false"),
                "ci-weiter" => got[0].1.contains("== \"green\""),
                "review-weiter" => got[0].1.contains("== \"JA\""),
                "ff-weiter" => got[0].1.contains("== \"done\""),
                _ => true,
            }
    })
}

/// `t` with the `then` of one case of one switch replaced (a mutant).
pub fn with_then(t: &str, step: &str, case: &str, new: &str) -> String {
    let (mut cur, mut c, mut out) = (String::new(), String::new(), Vec::new());
    for l in t.lines() {
        if let Some(n) = l.strip_prefix("  - ").and_then(|x| x.strip_suffix(':')) {
            cur = n.into();
        }
        if let Some(n) = l
            .strip_prefix("        - ")
            .and_then(|x| x.strip_suffix(':'))
        {
            c = n.into();
        }
        match l.strip_prefix("            then: ") {
            Some(_) if cur == step && c == case => out.push(format!("            then: {new}")),
            _ => out.push(l.to_string()),
        }
    }
    out.join("\n") + "\n"
}
