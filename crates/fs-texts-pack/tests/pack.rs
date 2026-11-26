//! Pattern: none (test). Deterministic output, and what is written loads with the reader of `mf2` (catalog, manifest, date blob).

mod support;

use mf2::{Arg, FormatContext, Formatter, Registry};
use mf2_catalog::{content_hash, Catalog, Manifest};

mf2::__date_statics!(gregorian no_zones);

static FUNCTIONS: [(&str, &dyn mf2::Function); 5] = [
    ("integer", &mf2::fn_number::INTEGER),
    ("number", &mf2::fn_number::NUMBER),
    ("datetime", &DATETIME),
    ("date", &DATE),
    ("time", &TIME),
];
static REGISTRY: Registry = Registry::new(&FUNCTIONS).with_numbers(&mf2::fn_number::NUMBERS);
static CX: FormatContext = {
    let mut cx = FormatContext::new(&mf2::host_std::HOST);
    cx.bidi = mf2::BidiStrategy::None;
    cx
};

fn bytes(files: &std::collections::BTreeMap<String, Vec<u8>>, prefix: &str) -> Vec<u8> {
    let (_, b) = files
        .iter()
        .find(|(n, _)| n.starts_with(prefix))
        .unwrap_or_else(|| panic!("no file {prefix}"));
    b.clone()
}

fn say(catalog: &Catalog, manifest: &Manifest, id: &str, args: &[Arg<'_>]) -> String {
    let (mut out, mut errors) = (String::new(), Vec::new());
    Formatter::new(catalog, &REGISTRY, &CX).write(
        manifest.msg_id(id).unwrap(),
        args,
        &mut out,
        &mut errors,
    );
    assert!(errors.is_empty(), "{id}: {errors:?}");
    out
}

#[test]
fn two_runs_give_the_same_bytes_on_disk() {
    let (a, b) = (tempfile::tempdir().unwrap(), tempfile::tempdir().unwrap());
    for dir in [a.path(), b.path()] {
        fs_texts_pack::pack(&support::data(), &[])
            .unwrap()
            .write(dir)
            .unwrap();
    }
    let list = |d: &std::path::Path| {
        let mut v: Vec<_> = walk(d, d);
        v.sort();
        v
    };
    let (la, lb) = (list(a.path()), list(b.path()));
    assert_eq!(
        la.iter().map(|x| &x.0).collect::<Vec<_>>(),
        lb.iter().map(|x| &x.0).collect::<Vec<_>>()
    );
    assert_eq!(la, lb, "byte-equal");
    assert_eq!(
        la.len(),
        7,
        "manifest, 2 catalogs, 2 date tables, 2 name blobs: {:?}",
        la.iter().map(|x| &x.0).collect::<Vec<_>>()
    );
}

fn walk(root: &std::path::Path, dir: &std::path::Path) -> Vec<(String, Vec<u8>)> {
    let mut out = Vec::new();
    for e in std::fs::read_dir(dir).unwrap().flatten() {
        let p = e.path();
        if p.is_dir() {
            out.extend(walk(root, &p));
        } else {
            out.push((
                p.strip_prefix(root).unwrap().to_string_lossy().into_owned(),
                std::fs::read(&p).unwrap(),
            ));
        }
    }
    out
}

#[test]
fn the_catalog_loads_and_a_message_resolves() {
    let files = fs_texts_pack::pack(&support::data(), &[]).unwrap().files;
    let manifest = Manifest::read(&bytes(&files, "manifest.mf2m")).expect("manifest");
    assert_eq!(manifest.ids, ["files", "hello", "when"]);
    for (tag, hello, files_3) in [
        ("de", "Hallo, Welt!", "3 Dateien"),
        ("en", "Hello, Welt!", "3 files"),
    ] {
        let (name, data) = files
            .iter()
            .find(|(n, _)| n.starts_with(&format!("{tag}.")))
            .unwrap();
        assert_eq!(
            *name,
            format!("{tag}.{}.mf2b", content_hash(data)),
            "named by its hash"
        );
        let catalog = Catalog::new(data.clone(), manifest.hash()).expect("catalog");
        assert_eq!(
            say(&catalog, &manifest, "hello", &[Arg::Str("Welt")]),
            hello
        );
        assert_eq!(say(&catalog, &manifest, "files", &[Arg::Int(3)]), files_3);
    }
}

fn leaked(bytes: Vec<u8>) -> &'static [u8] {
    // `with_server_data` takes `'static` data: leaked once per loaded language, as the loader of fs-components-mf2 does.
    Box::leak(bytes.into_boxed_slice())
}

#[test]
fn the_date_table_loads_and_de_formats_a_date() {
    let files = fs_texts_pack::pack(&support::data(), &[]).unwrap().files;
    assert!(
        !files.keys().any(|n| n.ends_with("icu.blob")),
        "no raw blob beside the table"
    );
    let manifest = Manifest::read(&bytes(&files, "manifest.mf2m")).unwrap();
    let table = bytes(&files, "dates/de/icu.table");
    assert!(
        (1000..4000).contains(&table.len()),
        "about 1.4 KB, was {}",
        table.len()
    );
    let de = Catalog::new(bytes(&files, "de."), manifest.hash()).unwrap();
    let de = de.with_server_data(leaked(table)).expect("server data");
    // The blob inside the table loads with the ICU4X provider.
    let blob = de
        .locale_entry(mf2_catalog::format::locale_key::ICU_BLOB)
        .expect("icu.blob entry");
    icu_provider_blob::BlobDataProvider::try_new_from_blob(Box::from(blob))
        .expect("BlobDataProvider");
    assert_eq!(say(&de, &manifest, "when", &[]), "26.11.2026, 12:05");
}

#[test]
fn a_broken_table_is_an_error_not_a_panic() {
    let files = fs_texts_pack::pack(&support::data(), &[]).unwrap().files;
    let manifest = Manifest::read(&bytes(&files, "manifest.mf2m")).unwrap();
    let de = Catalog::new(bytes(&files, "de."), manifest.hash()).unwrap();
    assert!(de
        .with_server_data(leaked(b"this is not a table".to_vec()))
        .is_err());
    let mut cut = bytes(&files, "dates/de/icu.table");
    cut.truncate(cut.len() / 2);
    let de = Catalog::new(bytes(&files, "de."), manifest.hash()).unwrap();
    assert!(de.with_server_data(leaked(cut)).is_err());
}
