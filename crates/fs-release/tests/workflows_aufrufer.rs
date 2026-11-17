//! Pattern: none (test). A caller of a flow passes every NEW input the callee reads without a default (`frontend_crate`, `schicht_fertig`): the callers of
//! `frontend-round` (`programm-neu`) and `modul-neu` (`ausnahme-gespraech`) are red when an input is missing. Counter-test on a changed copy.

mod support;
use support::workflows::{read, FILES};

const NEW_INPUTS: [&str; 2] = ["frontend_crate", "schicht_fertig"];
const CALLEES: [&str; 7] = [
    "frontend-round",
    "frontend-umbau",
    "stil-neu",
    "grundfassung",
    "geruest-heben",
    "modul-neu",
    "bausatz-neu",
];

/// The new inputs a flow reads without a default (`.input.<k>` not followed by ` //`).
fn required(callee: &str) -> Vec<&'static str> {
    let text = read(callee);
    NEW_INPUTS
        .iter()
        .copied()
        .filter(|k| {
            let key = format!(".input.{k}");
            text.match_indices(&key)
                .any(|(i, _)| !text[i + key.len()..].trim_start().starts_with("//"))
        })
        .collect()
}

/// (callee, the keys of its input block) for every `run:` call of a workflow in the text.
fn calls(text: &str) -> Vec<(String, Vec<String>)> {
    let lines: Vec<&str> = text.lines().collect();
    let mut out = Vec::new();
    for (i, l) in lines.iter().enumerate() {
        let Some(name) = l
            .trim()
            .strip_prefix("name: ")
            .filter(|_| l.starts_with("          name: "))
        else {
            continue;
        };
        let keys = lines[i..]
            .iter()
            .skip_while(|l| l.trim() != "input:")
            .skip(1)
            .take_while(|l| l.starts_with("          "))
            .filter_map(|l| {
                l.strip_prefix("          ")?
                    .split_once(':')
                    .map(|(k, _)| k.to_string())
            })
            .collect();
        out.push((name.to_string(), keys));
    }
    out
}

/// The callers that miss a required input: `caller -> callee: input`.
fn missing(text_of: impl Fn(&str) -> String) -> Vec<String> {
    let mut bad = Vec::new();
    for f in FILES {
        for (callee, keys) in calls(&text_of(f)) {
            if CALLEES.contains(&callee.as_str()) {
                for k in required(&callee) {
                    if !keys.iter().any(|x| x == k) {
                        bad.push(format!("{f} -> {callee}: {k}"));
                    }
                }
            }
        }
    }
    bad
}

#[test]
fn every_caller_passes_the_new_inputs_of_its_callee() {
    assert_eq!(missing(read), Vec::<String>::new());
    assert!(calls(&read("programm-neu"))
        .iter()
        .any(|(c, _)| c == "frontend-round"));
    assert!(calls(&read("ausnahme-gespraech"))
        .iter()
        .any(|(c, _)| c == "modul-neu"));
    assert!(required("frontend-round").contains(&"frontend_crate"));
    assert!(required("geruest-heben").contains(&"schicht_fertig"));
}

#[test]
fn counter_a_caller_without_the_new_input_is_red() {
    let bad = missing(|f| {
        let t = read(f);
        if f == "programm-neu" {
            t.replace("          frontend_crate: ${ .input.frontend_crate }\n", "")
        } else {
            t
        }
    });
    assert_eq!(
        bad,
        vec!["programm-neu -> frontend-round: frontend_crate".to_string()]
    );
}
