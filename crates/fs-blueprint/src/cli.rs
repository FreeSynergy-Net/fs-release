//! Pattern: Facade (the command `fs-blueprint check`: read the blueprint of a repo, check it against a catalog and the repo, print the findings as codes and as sentences).

use crate::catalog_file::FileCatalog;
use crate::emit::check as drift;
use crate::emit::generate::planned_paths;
use crate::error::Report;
use crate::kit::KitCatalog;
use crate::kit::RepoView;
use crate::messages::render;
use crate::model::Blueprint;
use crate::repo_fs::FsRepo;
use crate::rules::{Mode, Rule, Rules};
use crate::verified::verify;
use std::io::Write;
use std::path::PathBuf;

const HELP: &str = "fs-blueprint new BAUPLAN [--out DIR] [--catalog FILE] [--talks DIR] [--lang en|de] [--rules NAME]\n\
  new        check BAUPLAN (the file scaffold.toml; a folder forms/ next to it is part of it); when it is clean, make the program in DIR (default fs-<name>); on an error nothing is written\n\
fs-blueprint check [--repo DIR] [--catalog FILE] [--talks DIR] [--lang en|de] [--drift] [--rules NAME]\n\
fs-blueprint fmt [BAUPLAN] [--check] [--lang en|de]\n\
  fmt        write BAUPLAN (default scaffold.toml) in its canonical form (atomic); --check only says whether it is; comments are kept (they move with the line below them)\n\
fs-blueprint package rpm BAUPLAN [--out DIR] [--starter-min V] [--rpm-release N] [--source-date SECS]\n  package rpm  make the noarch RPM fs-<name> of a program (data below /usr/share/freesynergy/programs/<name>, .desktop, Requires from the blueprint); never signed\nfs-blueprint package release --key PUBKEY --pages-url URL [--out DIR] [--pkg-version V]\n  package release  make freesynergy-release (dnf source: stable on, nightly off; the public key)\nfs-blueprint --version | --help\n\
  check      read DIR/scaffold.toml and check it: structure, building blocks (catalog), files of the repo\n\
  --repo     the root of the repo (default: the current folder)\n\
  --catalog  a catalog file (default: the one shipped with the tool)\n\
  --talks    a checkout of the documentation repo: the talk records of the exceptions are looked for there\n\
  --lang     the language of the sentences (default en)\n\
  --drift    also compare the generated files of the repo with what the blueprint generates\n\
Exit codes: 0 no error (warnings are shown), 1 the check found errors, 2 wrong words or a file that cannot be read";

pub(crate) struct Options {
    pub(crate) repo: PathBuf,
    /// The words that are no option (the blueprint of `new`).
    pub(crate) positional: Vec<String>,
    /// `--out` of `new`.
    pub(crate) out_dir: Option<PathBuf>,
    pub(crate) catalog: Option<PathBuf>,
    pub(crate) talks: Option<PathBuf>,
    pub(crate) lang: &'static str,
    pub(crate) drift: bool,
    pub(crate) rules: Rules,
}

pub(crate) fn parse(words: &[String]) -> Result<Options, String> {
    let mut o = Options {
        repo: PathBuf::from("."),
        positional: Vec::new(),
        out_dir: None,
        catalog: None,
        talks: None,
        lang: "en",
        drift: false,
        rules: Rules::projekt(),
    };
    let mut it = words.iter();
    while let Some(w) = it.next() {
        let mut value = || {
            it.next()
                .cloned()
                .filter(|v| !v.starts_with("--"))
                .ok_or_else(|| format!("{w} needs a value"))
        };
        match w.as_str() {
            "--repo" => o.repo = PathBuf::from(value()?),
            "--catalog" => o.catalog = Some(PathBuf::from(value()?)),
            "--talks" => o.talks = Some(PathBuf::from(value()?)),
            "--lang" => {
                o.lang = match value()?.as_str() {
                    "en" => "en",
                    "de" => "de",
                    other => return Err(format!("the language {other} is unknown (en, de)")),
                };
            }
            "--rules" => {
                let name = value()?;
                o.rules = Rules::by_name(&name)
                    .ok_or_else(|| format!("the rule set {name} does not exist"))?;
            }
            "--drift" => o.drift = true,
            "--out" => o.out_dir = Some(PathBuf::from(value()?)),
            other if !other.starts_with("--") => o.positional.push(other.to_string()),
            other => return Err(format!("unknown word {other}")),
        }
    }
    Ok(o)
}

pub(crate) fn print(report: &Report, lang: &str, out: &mut dyn Write) {
    for (kind, list) in [("error", &report.errors), ("warning", &report.warnings)] {
        for c in list {
            let sentence = render(c, lang).unwrap_or_default();
            drop(writeln!(out, "{kind}: {c}  {sentence}"));
        }
    }
    drop(writeln!(
        out,
        "fs-blueprint: {} errors, {} warnings",
        report.errors.len(),
        report.warnings.len()
    ));
}

pub(crate) fn read_catalog(path: Option<&PathBuf>) -> Result<FileCatalog, String> {
    match path {
        None => Ok(FileCatalog::builtin()),
        Some(p) => {
            let text = std::fs::read_to_string(p)
                .map_err(|e| format!("cannot read the catalog {}: {e}", p.display()))?;
            FileCatalog::parse(&text)
                .map_err(|e| format!("the catalog {} is invalid: {e}", p.display()))
        }
    }
}

fn reject_positional(o: Options) -> Result<Options, String> {
    match o.positional.first() {
        Some(w) => Err(format!("unknown word {w}")),
        None => Ok(o),
    }
}

fn run_check(o: &Options, out: &mut dyn Write) -> Result<i32, String> {
    let catalog = read_catalog(o.catalog.as_ref())?;
    let file = o.repo.join("scaffold.toml");
    let text = std::fs::read_to_string(&file)
        .map_err(|e| format!("cannot read {}: {e}", file.display()))?;
    let bp = match Blueprint::from_toml(&text) {
        Ok(bp) => bp,
        Err(code) => {
            let report = Report {
                errors: vec![code],
                warnings: Vec::new(),
            };
            print(&report, o.lang, out);
            return Ok(1);
        }
    };
    let view = FsRepo::new(
        &o.repo,
        planned_paths(&bp, &catalog as &dyn KitCatalog),
        o.talks.clone(),
    );
    let mut report = crate::check::check(&bp, &catalog, &view, &o.rules);
    if o.drift {
        // The drift needs a blueprint that is sound in its structure, not one that has no finding at all: a repo with hand-written files or a missing talk
        // record still shows its drift. So the gate here is the structure only (every rule off, the hand-written files not counted).
        let rules = Rule::ALL
            .into_iter()
            .fold(o.rules.clone(), |r, rule| r.with(rule, Mode::Off));
        if let Ok(v) = verify(&bp, &catalog, &WithoutUnowned(&view), &rules) {
            let d = drift::check(&v, &catalog, &view);
            report.errors.extend(d.errors);
            report.warnings.extend(d.warnings);
        }
    }
    print(&report, o.lang, out);
    Ok(i32::from(!report.is_ok()))
}

/// Runs the command with the words after the program name. Returns the exit code.
pub fn run(words: &[String], out: &mut dyn Write, err: &mut dyn Write) -> i32 {
    match words.first().map(String::as_str) {
        Some("--version" | "-V") if words.len() == 1 => {
            drop(writeln!(out, "fs-blueprint {}", env!("CARGO_PKG_VERSION")));
            0
        }
        Some("--help" | "-h") if words.len() == 1 => {
            drop(writeln!(out, "{HELP}"));
            0
        }
        Some("new") => match parse(&words[1..]).and_then(|o| crate::cli_new::run_new(&o, out)) {
            Ok(code) => code,
            Err(m) => {
                drop(writeln!(
                    err,
                    "fs-blueprint: {m}\nTry `fs-blueprint --help`."
                ));
                2
            }
        },
        Some("package") => match crate::cli_package::run_package(&words[1..], out) {
            Ok(code) => code,
            Err(m) => {
                drop(writeln!(
                    err,
                    "fs-blueprint: {m}\nTry `fs-blueprint --help`."
                ));
                2
            }
        },
        Some("fmt") => match crate::cli_fmt::run_fmt(&words[1..], out) {
            Ok(code) => code,
            Err(m) => {
                drop(writeln!(
                    err,
                    "fs-blueprint: {m}\nTry `fs-blueprint --help`."
                ));
                2
            }
        },
        Some("check") => match parse(&words[1..])
            .and_then(|o| reject_positional(o).and_then(|o| run_check(&o, out)))
        {
            Ok(code) => code,
            Err(m) => {
                drop(writeln!(
                    err,
                    "fs-blueprint: {m}\nTry `fs-blueprint --help`."
                ));
                2
            }
        },
        _ => {
            drop(writeln!(
                err,
                "fs-blueprint: unknown or missing word\nTry `fs-blueprint --help`."
            ));
            2
        }
    }
}

/// The repo as it is, but with no hand-written file counted: the view for the structure gate of the drift.
struct WithoutUnowned<'a>(&'a FsRepo);

impl RepoView for WithoutUnowned<'_> {
    fn exists(&self, path: &str) -> bool {
        self.0.exists(path)
    }
    fn covers_generated(&self, path: &str) -> bool {
        self.0.covers_generated(path)
    }
    fn unowned_files(&self) -> Vec<String> {
        Vec::new()
    }
    fn main_files(&self) -> Vec<String> {
        self.0.main_files()
    }
    fn read(&self, path: &str) -> Option<String> {
        self.0.read(path)
    }
    fn talk_exists(&self, path: &str) -> bool {
        self.0.talk_exists(path)
    }
}
