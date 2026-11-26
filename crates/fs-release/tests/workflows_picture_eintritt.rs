//! Pattern: none (test). `picture-acceptance`: rule [EINTRITT-SYSTEM] - the entry preconditions come before every picture; a red or missing precondition takes no picture.

mod support;
use support::workflows::{read, step_body, step_names};

#[test]
fn picture_acceptance_stops_before_any_picture_when_an_entry_precondition_is_red() {
    let text = read("picture-acceptance");
    let names = step_names(&text);
    let at = |n: &str| {
        names
            .iter()
            .position(|x| x == n)
            .unwrap_or_else(|| panic!("no step {n}"))
    };
    assert!(
        at("vorbedingungen") < at("vorbedingungen-weiche")
            && at("vorbedingungen-weiche") < at("menue-bilder")
    );
    let weiche = step_body(&text, "vorbedingungen-weiche");
    assert!(weiche.contains("then: vorbedingung-fehlt") && !weiche.contains("then: pictures"));
    let step = step_body(&text, "vorbedingungen");
    for k in [
        "freesynergy-filesystem",
        "--check-system",
        "format 3 and 4",
        "red=0",
        "`repos`",
        "`person`",
    ] {
        assert!(
            step.contains(k) || text.contains(k),
            "precondition names {k}"
        );
    }
    // M1: development pictures skip the entry measure, installed ones go through it
    let w = step_body(&text, "eintritt-weiche");
    assert!(at("eintritt-weiche") < at("vorbedingungen"));
    assert!(w.contains("installiert // false) == true") && w.contains("then: vorbedingungen"));
    assert!(w.contains("then: menue-bilder"));
    for f in [
        "frontend-round",
        "frontend-umbau",
        "programm-grundgeruest",
        "grundfassung",
    ] {
        assert!(
            read(f).contains("installiert: false"),
            "{f} passes installiert"
        );
    }
    // M2: `befunde` = failed points only, the evidence of every point is in `belege`; the gate reads red and befunde
    let step = step_body(&text, "vorbedingungen");
    assert!(
        step.contains("belege: [{ punkt, beleg }]")
            && step.contains("ONLY the failed or unchecked points")
    );
    assert!(
        weiche.contains("red // false) == true") && weiche.contains("befunde // [] | length) > 0")
    );
    assert!(weiche.contains("then: continue") && weiche.contains("ohne-ergebnis"));
    assert!(step_body(&text, "vorbedingung-fehlt").contains("no picture is taken"));
}

/// Which case of `vorbedingungen-weiche` fires for `$context` = `ctx` (first case whose jq `when` is true; the last case has none).
fn weiche_case(text: &str, ctx: &str) -> String {
    use std::io::Write;
    let body = step_body(text, "vorbedingungen-weiche");
    let mut cases: Vec<(String, Option<String>)> = Vec::new();
    for l in body.lines() {
        if let Some(n) = l
            .strip_prefix("        - ")
            .and_then(|x| x.strip_suffix(':'))
        {
            cases.push((n.to_string(), None));
        } else if let Some(w) = l.strip_prefix("            when: ") {
            let e = w
                .trim()
                .trim_matches('\'')
                .trim_start_matches("${")
                .trim_end_matches('}');
            cases.last_mut().unwrap().1 = Some(e.to_string());
        }
    }
    for (n, e) in cases {
        let Some(e) = e else { return n };
        let mut c = std::process::Command::new("jq")
            .args(["-e", "--argjson", "context", ctx, &e])
            .stdin(std::process::Stdio::piped())
            .stdout(std::process::Stdio::null())
            .spawn()
            .expect("jq is installed");
        c.stdin.take().unwrap().write_all(b"null").unwrap();
        if c.wait().unwrap().success() {
            return n;
        }
    }
    unreachable!("the last case has no when")
}

#[test]
fn a_missing_or_incomplete_result_is_never_green() {
    let t = read("picture-acceptance");
    let five = r#"[{"punkt":1},{"punkt":2},{"punkt":3},{"punkt":4},{"punkt":5}]"#;
    let ctx = |v: String| format!(r#"{{"vorbedingungen":{v}}}"#);
    assert_eq!(
        weiche_case(
            &t,
            &ctx(format!(r#"{{"red":false,"befunde":[],"belege":{five}}}"#))
        ),
        "gruen"
    );
    assert_eq!(weiche_case(&t, "{}"), "ohne-ergebnis");
    assert_eq!(weiche_case(&t, &ctx("null".into())), "ohne-ergebnis");
    assert_eq!(weiche_case(&t, &ctx("{}".into())), "ohne-ergebnis");
    assert_eq!(
        weiche_case(&t, &ctx(r#"{"red":false,"befunde":[]}"#.into())),
        "ohne-ergebnis"
    );
    assert_eq!(
        weiche_case(
            &t,
            &ctx(r#"{"red":false,"befunde":[],"belege":[{"punkt":1}]}"#.into())
        ),
        "ohne-ergebnis"
    );
    assert_eq!(
        weiche_case(
            &t,
            &ctx(format!(r#"{{"red":true,"befunde":[],"belege":{five}}}"#))
        ),
        "rot"
    );
    assert_eq!(
        weiche_case(
            &t,
            &ctx(format!(
                r#"{{"red":false,"befunde":[{{"punkt":2}}],"belege":{five}}}"#
            ))
        ),
        "rot"
    );
}
