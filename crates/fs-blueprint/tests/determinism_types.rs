//! E3-4: contract text and tables are byte-identical on every run, and the same blueprint written differently (comments, spacing) gives the same bytes.
mod support;
use fs_blueprint::Blueprint;
use support::{generate, FakeCatalog, SHIFT};

fn schema_files(src: &str) -> Vec<(String, String)> {
    let bp = Blueprint::from_toml(src).unwrap();
    generate(&bp, &FakeCatalog)
        .unwrap()
        .into_iter()
        .filter(|a| a.path.starts_with("schema/"))
        .map(|a| (a.path, a.content))
        .collect()
}

#[test]
fn generating_twice_gives_byte_identical_contract_and_tables() {
    let first = schema_files(SHIFT);
    assert_eq!(first.len(), 2);
    for _ in 0..20 {
        assert_eq!(first, schema_files(SHIFT));
    }
}

#[test]
fn comments_and_blank_lines_of_the_author_change_nothing() {
    let noisy = format!(
        "# a comment\n\n{}\n\n# end\n",
        SHIFT.replace("\n\n", "\n\n\n")
    );
    assert_eq!(schema_files(SHIFT), schema_files(&noisy));
}
