//! Decision 45: the generated `main` is purely generated and exchangeable; it is byte-equal in every blueprint except for the references into the blueprint.
mod support;
use fs_blueprint::emit::header::blueprint_hash;
use fs_blueprint::Blueprint;
use support::synth::{blueprint, COUNT};
use support::{generate, FakeCatalog, SHIFT, TASKS};

fn blueprints() -> Vec<Blueprint> {
    let mut v: Vec<Blueprint> = (0..COUNT).map(blueprint).collect();
    v.push(Blueprint::from_toml(TASKS).unwrap());
    v.push(Blueprint::from_toml(SHIFT).unwrap());
    v
}

fn main_of(bp: &Blueprint) -> String {
    let path = format!("crates/fs-{}-bin/src/main.rs", bp.program.name);
    generate(bp, &FakeCatalog)
        .unwrap()
        .into_iter()
        .find(|a| a.path == path)
        .unwrap_or_else(|| panic!("no main at {path}"))
        .content
}

/// The references into the blueprint: the blueprint hash in the head line and the program name constant. Everything else must be equal.
fn masked(bp: &Blueprint) -> String {
    let main = main_of(bp);
    let hash = blueprint_hash(bp).unwrap();
    // the content hash of the head line names the body, which holds the program name: also a reference
    let (head, rest) = main.split_once("content sha256 ").unwrap();
    let main = format!("{head}content sha256 @BODY@{}", &rest[64..]);
    main.replace(&hash, "@HASH@").replace(
        &format!("const PROGRAM: &str = \"{}\";", bp.program.name),
        "const PROGRAM: &str = \"@NAME@\";",
    )
}

#[test]
fn the_generated_main_is_byte_equal_in_all_example_blueprints_except_the_references() {
    let all = blueprints();
    assert!(all.len() >= 7);
    let first = masked(&all[0]);
    for bp in &all {
        assert_eq!(first, masked(bp), "{}", bp.program.name);
    }
    // the mask hides only the references: without it the files differ
    assert_ne!(main_of(&all[0]), main_of(&all[1]));
}

#[test]
fn the_main_names_nothing_of_the_program_but_its_name() {
    for bp in blueprints() {
        let main = main_of(&bp);
        let mut specific = vec![
            bp.contract.id.clone(),
            bp.contract.version.clone(),
            bp.program.capability.clone(),
        ];
        specific.extend(bp.program.summary.en.clone());
        specific.extend(bp.program.description.de.clone());
        specific.extend(bp.module.iter().map(|m| m.name.clone()));
        specific.extend(bp.types.iter().map(|t| t.name.clone()));
        for word in specific
            .into_iter()
            .filter(|w| w.len() > 3 && *w != bp.program.name)
        {
            assert!(
                !main.contains(&word),
                "{}: main contains {word}",
                bp.program.name
            );
        }
        // the name appears once in the body (the constant) and nowhere else but the head line is free of it
        assert_eq!(
            main.matches(bp.program.name.as_str()).count(),
            1,
            "{}",
            bp.program.name
        );
    }
}

#[test]
fn two_mains_differ_in_exactly_two_lines_the_head_line_and_the_name() {
    let (a, b) = (main_of(&blueprint(0)), main_of(&blueprint(1)));
    let (la, lb): (Vec<&str>, Vec<&str>) = (a.lines().collect(), b.lines().collect());
    assert_eq!(la.len(), lb.len());
    let differing = la.iter().zip(&lb).filter(|(x, y)| x != y).count();
    assert_eq!(differing, 2);
}
