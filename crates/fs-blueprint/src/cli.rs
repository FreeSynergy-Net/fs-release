//! Pattern: Facade (the command `fs-blueprint check`: read the blueprint of a repo, check it against a catalog and the repo, print the findings as codes and as sentences).

use crate::catalog_file::FileCatalog;
use crate::emit::{check as drift, generate};
use crate::error::Report;
use crate::kit::KitCatalog;
use crate::messages::render;
use crate::model::Blueprint;
use crate::repo_fs::FsRepo;
use crate::rules::Rules;
use std::io::Write;
use std::path::PathBuf;

const HELP: &str = "fs-blueprint check [--repo DIR] [--catalog FILE] [--talks DIR] [--lang en|de] [--drift] [--rules NAME]\n\
fs-blueprint --version | --help\n\
  check      read DIR/scaffold.toml and check it: structure, building blocks (catalog), files of the repo\n\
  --repo     the root of the repo (default: the current folder)\n\
  --catalog  a catalog file (default: the one shipped with the tool)\n\
  --talks    a checkout of the documentation repo: the talk records of the exceptions are looked for there\n\
  --lang     the language of the sentences (default en)\n\
  --drift    also compare the generated files of the repo with what the blueprint generates\n\
Exit codes: 0 no error (warnings are shown), 1 the check found errors, 2 wrong words or a file that cannot be read";

struct Options {
    repo: PathBuf,
    catalog: Option<PathBuf>,
    talks: Option<PathBuf>,
    lang: &'static str,
    drift: bool,
    rules: Rules,
}

fn parse(words: &[String]) -> Result<Options, String> {
    let mut o = Options {
        repo: PathBuf::from("."),
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
            other => return Err(format!("unknown word {other}")),
        }
    }
    Ok(o)
}

fn print(report: &Report, lang: &str, out: &mut dyn Write) {
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

fn read_catalog(path: Option<&PathBuf>) -> Result<FileCatalog, String> {
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
    let generated = generate(&bp, &catalog as &dyn KitCatalog)
        .map(|a| a.into_iter().map(|x| x.path).collect())
        .unwrap_or_default();
    let view = FsRepo::new(&o.repo, generated, o.talks.clone());
    let mut report = crate::check::check(&bp, &catalog, &view, &o.rules);
    if o.drift {
        let d = drift::check(&bp, &catalog, &view);
        report.errors.extend(d.errors);
        report.warnings.extend(d.warnings);
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
        Some("check") => match parse(&words[1..]).and_then(|o| run_check(&o, out)) {
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
