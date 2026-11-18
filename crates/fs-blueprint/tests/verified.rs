//! Only a blueprint that passed the check reaches the generator (F2 of the S3 review): `verify` gives the type `Verified` or the whole report.
mod support;
use fs_blueprint::{verify, Blueprint, Mode, Rule, Rules};
use support::synth::{blueprint, text};
use support::{edit, FakeCatalog, FakeView, TASKS};

fn off() -> Rules {
    Rule::ALL
        .into_iter()
        .fold(Rules::projekt(), |r, rule| r.with(rule, Mode::Off))
}

#[test]
fn a_clean_blueprint_is_verified_and_gives_itself_back() {
    let bp = blueprint(1);
    let v = verify(&bp, &support::Lenient, &support::AllView, &off()).unwrap();
    assert_eq!(v.blueprint(), &bp);
}

#[test]
fn a_blueprint_with_an_error_is_not_verified_and_the_report_has_the_errors_and_the_warnings() {
    let bp = Blueprint::from_toml(TASKS).unwrap();
    let report = verify(&bp, &FakeCatalog, &FakeView::standard(), &Rules::projekt()).unwrap_err();
    assert!(report.has("talk-not-found") && report.has("version-unknown"));
    assert_eq!(report.warnings.len(), 3, "the pending skeleton parts");
}

#[test]
fn the_rules_decide_what_is_an_error_a_structure_error_always_stops() {
    let bp = Blueprint::from_toml(TASKS).unwrap();
    // with every rule off only the structure counts: the example then passes with the fake view that has its files
    assert!(verify(&bp, &FakeCatalog, &FakeView::standard(), &off()).is_ok());
    let broken =
        Blueprint::from_toml(&edit(&text(0), "version = \"0.0.1\"", "version = \"1\"")).unwrap();
    assert!(
        verify(&broken, &support::Lenient, &support::AllView, &off())
            .unwrap_err()
            .has("semver-invalid")
    );
}
