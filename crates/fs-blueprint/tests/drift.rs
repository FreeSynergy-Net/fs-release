//! `check` (is the repo the product of the blueprint?) and `write` (only generated files, never a seed over a seed, never an exception).
mod support;
use fs_blueprint::emit::write::{write, DirStore, Found, Store};
use fs_blueprint::Report;
use support::synth::{blueprint, MemStore};
use support::{drift as check, generate, FakeCatalog, FakeView};

const MAIN: &str = ".github/workflows/check.yml";
/// No emitter makes a seed any more (the manifests are gone); the machinery of the class stays and is tested with a hand-made seed artifact.
const SEED: &str = "crates/fs-beta-shelf-planner-core/Cargo.toml";
const ATTR: &str = ".gitattributes";
const OWN: &str = "crates/fs-beta-shelf-planner-core/src/own1.rs";

fn written() -> MemStore {
    let mut store = MemStore::default();
    let arts = generate(&blueprint(1), &FakeCatalog).unwrap();
    write(&arts, &mut store).unwrap();
    store
}

fn drift(store: &MemStore) -> Report {
    check(&blueprint(1), &FakeCatalog, &FakeView::of_store(store))
}

fn texts(r: &Report) -> Vec<String> {
    for e in r.errors.iter().chain(&r.warnings) {
        assert!(e.conforms(), "{e}");
    }
    r.errors.iter().map(ToString::to_string).collect()
}

#[test]
fn a_repo_that_is_the_output_has_no_drift_and_the_exception_is_left_alone() {
    let store = written();
    assert!(store.files.contains_key(MAIN));
    assert!(
        !store.files.keys().any(|p| p.ends_with("Cargo.toml")),
        "a pure data package: no manifest"
    );
    assert!(
        !store.files.contains_key(OWN),
        "an exception is never written"
    );
    assert_eq!(texts(&drift(&store)), Vec::<String>::new());
}

#[test]
fn a_generated_file_changed_by_hand_is_drift_with_its_path() {
    let mut store = written();
    store.files.get_mut(MAIN).unwrap().push_str("// hand\n");
    assert_eq!(
        texts(&drift(&store)),
        [format!("generated-changed{{path={MAIN}}}")]
    );
}

#[test]
fn a_missing_generated_file_is_reported() {
    let mut store = written();
    store.files.remove(ATTR);
    assert_eq!(
        texts(&drift(&store)),
        [format!("generated-missing{{path={ATTR}}}")]
    );
}

#[test]
fn a_file_without_the_head_line_at_a_generated_path_is_drift() {
    let mut store = written();
    store
        .files
        .insert(MAIN.to_string(), "fn main() {}\n".to_string());
    assert_eq!(
        texts(&drift(&store)),
        [format!("generated-changed{{path={MAIN}}}")]
    );
}

#[test]
fn a_changed_blueprint_makes_the_old_output_drift() {
    let store = written();
    let mut other = blueprint(1);
    other.program.summary.en = Some("something else".to_string());
    let r = check(&other, &FakeCatalog, &FakeView::of_store(&store));
    let got = texts(&r);
    assert!(
        got.contains(&format!("generated-changed{{path={MAIN}}}")),
        "{got:?}"
    );
    assert!(!got.iter().any(|g| g.contains(SEED)));
}

#[test]
fn write_twice_is_stable_and_never_overwrites_a_seed() {
    let mut store = written();
    let before = store.files.clone();
    let mut arts = generate(&blueprint(1), &FakeCatalog).unwrap();
    arts.push(fs_blueprint::Artifact::new(
        SEED,
        fs_blueprint::Class::Seed,
        "// seed\n",
    ));
    let out = write(&arts, &mut store).unwrap();
    assert_eq!(out.kept, Vec::<String>::new());
    assert_eq!(store.files[SEED], "// seed\n");
    store.files.remove(SEED);
    let out = write(&arts, &mut store).unwrap();
    assert_eq!(out.kept, Vec::<String>::new());
    assert_eq!(store.files.len(), before.len() + 1);
    let out = write(&arts, &mut store).unwrap();
    assert_eq!(out.kept, [SEED]);
    assert_eq!(out.skipped, Vec::<String>::new());
    store
        .files
        .insert(SEED.to_string(), "// mine\n".to_string());
    let out = write(&arts, &mut store).unwrap();
    assert_eq!(store.files[SEED], "// mine\n");
    assert_eq!(out.kept, [SEED]);
}

#[test]
fn write_never_touches_a_file_of_an_exception() {
    let mut store = MemStore::default();
    store
        .files
        .insert(OWN.to_string(), "hand written\n".to_string());
    let mut arts = generate(&blueprint(1), &FakeCatalog).unwrap();
    arts.push(fs_blueprint::Artifact::new(
        OWN,
        fs_blueprint::Class::Exception,
        "generated?\n",
    ));
    let out = write(&arts, &mut store).unwrap();
    assert_eq!(store.files[OWN], "hand written\n");
    assert_eq!(out.skipped, [OWN]);
}

#[test]
fn write_refuses_a_hand_changed_generated_file_and_writes_nothing_at_all() {
    let mut store = MemStore::default();
    store
        .files
        .insert(MAIN.to_string(), "fn main() { /* mine */ }\n".to_string());
    let arts = generate(&blueprint(1), &FakeCatalog).unwrap();
    let err = write(&arts, &mut store).unwrap_err();
    assert_eq!(
        err.iter().map(ToString::to_string).collect::<Vec<_>>(),
        [format!("generated-changed{{path={MAIN}}}")]
    );
    assert_eq!(store.files.len(), 1, "nothing else was written");
    let mut full = written();
    full.files.get_mut(MAIN).unwrap().push_str("// hand\n");
    assert!(write(&arts, &mut full).is_err());
}

#[test]
fn write_replaces_the_output_of_an_older_blueprint() {
    let mut store = written();
    let mut other = blueprint(1);
    other.program.summary.en = Some("something else".to_string());
    let arts = generate(&other, &FakeCatalog).unwrap();
    write(&arts, &mut store).unwrap();
    let left = texts(&check(&other, &FakeCatalog, &FakeView::of_store(&store)));
    assert_eq!(left, Vec::<String>::new());
}

#[test]
fn a_failing_store_is_reported_with_the_file_that_failed() {
    let mut store = MemStore {
        fail: true,
        ..MemStore::default()
    };
    let arts = generate(&blueprint(1), &FakeCatalog).unwrap();
    let err = write(&arts, &mut store).unwrap_err();
    assert_eq!(err.len(), 1);
    assert!(err.iter().all(|e| e.code == "write-failed" && e.conforms()));
}

#[test]
fn the_directory_store_writes_nested_paths_with_exact_bytes() {
    let root = std::path::Path::new(env!("CARGO_TARGET_TMPDIR"))
        .join(format!("emit-{}", std::process::id()));
    let mut store = DirStore::new(&root);
    let arts = generate(&blueprint(2), &FakeCatalog).unwrap();
    write(&arts, &mut store).unwrap();
    for a in &arts {
        let on_disk =
            std::fs::read(a.path.split('/').fold(root.clone(), |p, c| p.join(c))).unwrap();
        assert_eq!(on_disk, a.content.as_bytes(), "{}", a.path);
        assert!(!on_disk.contains(&b'\r'));
        assert_eq!(store.read(&a.path), Found::Text(a.content.clone()));
    }
    assert_eq!(store.read("nope/x.rs"), Found::Missing);
}
