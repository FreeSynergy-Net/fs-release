//! Test support for `pin-kaskade`: the graph of the Glieder (edges, order) and the switch cases, checked one by one.

use super::workflows::step_body;

/// The `@fs` calls of `pin-kaskade` that are real scripts in `scripts/`.
pub const SCRIPTS: &[&str] = &[
    "kaskade-lage",
    "kaskade-plan",
    "kaskade-glied",
    "kaskade-glied-fertig",
    "kaskade-fremde-pins",
    "lock-gleich",
    "gates-komplett",
    "kaskade-zyklen",
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

/// The header is self-consistent: every Glied of `# GLIEDER-ORDER:` has its `# DAG:` line (none missing, none surplus) and the order is a topological one of the
/// `# DAG:` lines. That the edges are the REAL ones is proven by `fs-release dag-check` against the Cargo.toml files (tests/dag.rs).
pub fn graph_ok(t: &str) -> bool {
    let h = fs_release::dag::parse_header(t);
    let mut keys: Vec<&String> = h.dag.keys().collect();
    keys.sort();
    let mut o: Vec<&String> = h.order.iter().collect();
    o.sort();
    let topo = h.order.iter().enumerate().all(|(i, g)| {
        h.dag
            .get(g)
            .into_iter()
            .flatten()
            .all(|d| h.order.iter().position(|y| y == d).is_none_or(|j| j < i))
    });
    !h.order.is_empty() && keys == o && topo
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
