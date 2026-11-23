//! The workflow file as a PARSED tree (no text search): steps, switch cases, calls, `if`, `then`, and the changes the mutants make.

use serde_json::{json, Value};

pub fn load(name: &str) -> Value {
    let path = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../workflows")
        .join(format!("{name}.yaml"));
    let text = std::fs::read_to_string(path).unwrap();
    serde_norway::from_str(&text).unwrap()
}

fn list(d: &Value) -> &Vec<Value> {
    d["do"].as_array().expect("do is a list")
}

pub fn names(d: &Value) -> Vec<String> {
    list(d)
        .iter()
        .map(|s| s.as_object().unwrap().keys().next().unwrap().clone())
        .collect()
}

pub fn step<'a>(d: &'a Value, n: &str) -> Option<&'a Value> {
    list(d).iter().find_map(|s| s.get(n))
}

pub fn pos(d: &Value, n: &str) -> Option<usize> {
    names(d).iter().position(|x| x == n)
}

/// The text of a field of a step (`call`, `if`, `then`), empty when missing.
pub fn field<'a>(d: &'a Value, n: &str, k: &str) -> &'a str {
    step(d, n)
        .and_then(|s| s.get(k))
        .and_then(Value::as_str)
        .unwrap_or("")
}

pub fn meta<'a>(d: &'a Value, n: &str, k: &str) -> &'a str {
    step(d, n)
        .and_then(|s| s["metadata"].get(format!("freesynergy.net:{k}")))
        .and_then(Value::as_str)
        .unwrap_or("")
}

/// The call without the version (`text-gate:0.0.17@fs` gives `text-gate`).
pub fn call<'a>(d: &'a Value, n: &str) -> &'a str {
    field(d, n, "call").split(':').next().unwrap_or("")
}

/// A `with` parameter of a step.
pub fn with<'a>(d: &'a Value, n: &str, k: &str) -> &'a str {
    step(d, n)
        .and_then(|s| s["with"].get(k))
        .and_then(Value::as_str)
        .unwrap_or("")
}

/// The switch cases of a step: (case name, `when`, `then`).
pub fn cases(d: &Value, n: &str) -> Vec<(String, String, String)> {
    let Some(sw) = step(d, n).and_then(|s| s["switch"].as_array()) else {
        return vec![];
    };
    sw.iter()
        .map(|case| {
            let (name, body) = case.as_object().unwrap().iter().next().unwrap();
            let get = |f: &str| {
                body.get(f)
                    .and_then(Value::as_str)
                    .unwrap_or("")
                    .to_string()
            };
            (name.clone(), get("when"), get("then"))
        })
        .collect()
}

pub fn case_to(d: &Value, n: &str, then: &str) -> Option<String> {
    cases(d, n).into_iter().find(|c| c.2 == then).map(|c| c.1)
}

// ---- the changes of the mutants

pub fn edit(d: &mut Value, n: &str, f: impl FnOnce(&mut Value)) {
    let s = d["do"]
        .as_array_mut()
        .unwrap()
        .iter_mut()
        .find_map(|s| s.get_mut(n))
        .unwrap_or_else(|| panic!("no step {n}"));
    f(s);
}

pub fn drop_step(d: &mut Value, n: &str) {
    d["do"]
        .as_array_mut()
        .unwrap()
        .retain(|s| s.get(n).is_none());
}

pub fn move_step_to_end(d: &mut Value, n: &str) {
    let list = d["do"].as_array_mut().unwrap();
    let at = list.iter().position(|x| x.get(n).is_some()).unwrap();
    let moved = list.remove(at);
    list.push(moved);
}

pub fn set_text(d: &mut Value, n: &str, k: &str, v: &str) {
    edit(d, n, |s| s[k] = json!(v));
}

pub fn remove_key(d: &mut Value, n: &str, k: &str) {
    edit(d, n, |s| {
        s.as_object_mut().unwrap().remove(k).expect("key");
    });
}

pub fn set_meta(d: &mut Value, n: &str, k: &str, v: &str) {
    edit(d, n, |s| {
        s["metadata"][format!("freesynergy.net:{k}")] = json!(v);
    });
}

pub fn set_with(d: &mut Value, n: &str, k: &str, v: &str) {
    edit(d, n, |s| s["with"][k] = json!(v));
}

/// The switch case `case` of the step gets another `when`.
pub fn set_when(d: &mut Value, n: &str, case: &str, when: &str) {
    edit(d, n, |s| {
        for c in s["switch"].as_array_mut().unwrap() {
            if let Some(v) = c.get_mut(case) {
                v["when"] = json!(when);
            }
        }
    });
}

pub fn set_then(d: &mut Value, n: &str, case: &str, then: &str) {
    edit(d, n, |s| {
        for c in s["switch"].as_array_mut().unwrap() {
            if let Some(v) = c.get_mut(case) {
                v["then"] = json!(then);
            }
        }
    });
}

pub fn drop_case(d: &mut Value, n: &str, case: &str) {
    edit(d, n, |s| {
        s["switch"]
            .as_array_mut()
            .unwrap()
            .retain(|c| c.get(case).is_none());
    });
}
