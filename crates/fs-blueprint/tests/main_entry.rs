//! The generated `main` holds the program name and the hash of the blueprint as constants; the blueprint is read when the program starts, never embedded (S5b).
mod support;
use fs_blueprint::emit::header::blueprint_hash;
use fs_blueprint::Blueprint;
use support::{generate, FakeCatalog, TASKS};

#[test]
fn main_has_the_name_and_the_hash_and_embeds_no_blueprint() {
    let bp = Blueprint::from_toml(TASKS).unwrap();
    let out = generate(&bp, &FakeCatalog).unwrap();
    let main = out
        .iter()
        .find(|a| a.path == "crates/fs-tasks-bin/src/main.rs")
        .unwrap();
    let hash = blueprint_hash(&bp).unwrap();
    assert!(main.content.contains("const PROGRAM: &str = \"tasks\";"));
    assert!(main
        .content
        .contains(&format!("const BLUEPRINT_SHA256: &str = \"{hash}\";")));
    for gone in ["include_str!", "BLUEPRINT:", "../scaffold.toml"] {
        assert!(!main.content.contains(gone), "{gone}");
    }
    assert!(main.content.contains("FS_BLUEPRINT") && main.content.contains("read_to_string"));
}

#[test]
fn a_change_of_the_blueprint_changes_the_hash_in_main() {
    let a = Blueprint::from_toml(TASKS).unwrap();
    let b = Blueprint::from_toml(&support::edit(
        TASKS,
        "Tasks, dependencies and boards",
        "Tasks and boards",
    ))
    .unwrap();
    let hash_in = |bp: &Blueprint| {
        let out = generate(bp, &FakeCatalog).unwrap();
        let m = out
            .iter()
            .find(|x| x.path == "crates/fs-tasks-bin/src/main.rs")
            .unwrap();
        m.content
            .lines()
            .find(|l| l.starts_with("const BLUEPRINT_SHA256"))
            .unwrap()
            .to_string()
    };
    assert_ne!(hash_in(&a), hash_in(&b));
}
