//! Synthetic blueprints for the generator tests: every one differs from the others in name, capability, texts, contract, modules and exceptions.
#![allow(dead_code)]

use std::collections::BTreeMap;

use fs_blueprint::emit::write::{Found, Store};
use fs_blueprint::Blueprint;

/// (program name, capability, contract id, contract version) of the synthetic blueprints.
const SPECS: [(&str, &str, &str, &str); 5] = [
    ("alpha-desk", "desking", "0xa04d258971543401", "0.1"),
    (
        "beta-shelf-planner",
        "shelving",
        "0xb04d258971543402",
        "0.7",
    ),
    ("gamma", "gamma-things", "0xc04d258971543403", "1.0"),
    ("delta-lab-notes", "labbing", "0xd04d258971543404", "0.12"),
    ("epsilonware", "epsilons", "0xe04d258971543405", "3.4"),
];

pub const COUNT: usize = SPECS.len();

/// The text of the synthetic blueprint number `i` (0..COUNT).
pub fn text(i: usize) -> String {
    let (name, cap, id, ver) = SPECS[i];
    let extra = match i % 3 {
        0 => format!("[[module]]\nname = \"fs-mod{i}\"\nkind = \"standard\"\nversion = \"0.0.{i}\"\n\n"),
        1 => format!("[[exception]]\nwhat = \"own thing {i}\"\nwhy = \"nothing fits {i}\"\nchecked = [\"fs-a\"]\nalternatives = [\"b\"]\ntalk = \"conventions/talks/t{i}.md\"\npaths = [\"crates/fs-{name}-core/src/own{i}.rs\"]\n\n"),
        _ => String::new(),
    };
    format!(
        "schema = 2\n\n[program]\nname = \"{name}\"\nskeleton = \"0.1.0\"\ncapability = \"{cap}\"\nsummary = {{ en = \"Summary of {name}\", de = \"Kurz {name}\" }}\ndescription = {{ en = \"Long text about {cap} number {i}.\", de = \"Langtext {cap} Nummer {i}.\" }}\ncategories = [\"Utility\", \"Cat{i}\"]\n\n[contract]\nversion = \"{ver}\"\nid = \"{id}\"\nprovides = [\"cap.{i}\"]\n\n{extra}[frontend]\nstandard = \"front{i}\"\nlayout = \"layout/f{i}.toml\"\ncovers = \"all\"\n"
    )
}

pub fn blueprint(i: usize) -> Blueprint {
    Blueprint::from_toml(&text(i)).expect("the synthetic blueprint loads")
}

/// A store in memory.
#[derive(Default)]
pub struct MemStore {
    pub files: BTreeMap<String, String>,
    pub fail: bool,
}

impl Store for MemStore {
    fn read(&self, path: &str) -> Found {
        self.files
            .get(path)
            .map_or(Found::Missing, |c| Found::Text(c.clone()))
    }
    fn put(&mut self, path: &str, content: &str) -> Result<(), String> {
        if self.fail {
            return Err("disk full".to_string());
        }
        self.files.insert(path.to_string(), content.to_string());
        Ok(())
    }
}
