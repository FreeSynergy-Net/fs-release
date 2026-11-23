//! Pattern: Facade (`fs-release address bestand|ersetzen|nachher`; every error is exit code 2 with a message, red is 1).

use std::collections::BTreeMap;
use std::path::PathBuf;

use clap::{Args, Subcommand};

use super::allow::Allow;
use super::class::Class;
use super::replace::replace_tree;
use super::scan::{repo_name, scan, Hit};

#[derive(Args)]
pub struct AddressArgs {
    #[command(subcommand)]
    action: Action,
}

#[derive(Subcommand)]
enum Action {
    /// Read only: count owner mentions per repo and class on a ref (broad pattern).
    Bestand(Common),
    /// Swap the owner in the safe forms in the work tree (never Cargo sources, lock files, catalog data). No commit.
    Ersetzen(Common),
    /// After-check with the broad pattern: green only if nothing is left outside `allow` and outside the Cargo classes.
    Nachher(Nachher),
}

#[derive(Args)]
struct Common {
    /// Old owner.
    #[arg(long, default_value = "FreeSynergy")]
    old: String,
    /// New owner.
    #[arg(long, default_value = "FreeSynergy-Net")]
    new: String,
    /// JSON file with `allow` entries {pattern, reason, scope, repo}.
    #[arg(long)]
    allow: Option<PathBuf>,
    /// Ref to read (bestand, nachher).
    #[arg(long = "ref", default_value = "origin/main")]
    git_ref: String,
    /// Repo directories.
    #[arg(required = true)]
    dirs: Vec<PathBuf>,
}

#[derive(Args)]
struct Nachher {
    #[command(flatten)]
    common: Common,
    /// After part 2: the Cargo classes a and b must be empty too.
    #[arg(long)]
    all_classes: bool,
}

/// Runs the command, prints the result and returns the exit code (0 ok, 1 red, 2 error).
#[must_use]
pub fn run(args: &AddressArgs) -> i32 {
    match &args.action {
        Action::Bestand(c) => stock(c, false, false),
        Action::Nachher(n) => stock(&n.common, true, n.all_classes),
        Action::Ersetzen(c) => replace(c),
    }
}

fn load_allow(c: &Common) -> Result<Allow, String> {
    c.allow.as_deref().map_or_else(|| Ok(Allow::none()), Allow::load)
}

fn replace(c: &Common) -> i32 {
    let mut allow = match load_allow(c) {
        Ok(a) => a,
        Err(e) => return fail(&e),
    };
    let mut code = 0;
    for d in &c.dirs {
        match replace_tree(d, &c.old, &c.new, &mut allow) {
            Ok(r) => println!(
                "{} ersetzt: {} Dateien, {} Zeilen; unberuehrt wegen Klasse (Cargo, Lock, Katalog): {} Dateien",
                d.display(),
                r.files,
                r.lines,
                r.left_by_class
            ),
            Err(e) => code = fail(&e),
        }
    }
    code
}

fn fail(msg: &str) -> i32 {
    eprintln!("fehler: {msg}");
    2
}

/// `check`: after-check mode (green or red); otherwise only the counts.
fn stock(c: &Common, check: bool, all_classes: bool) -> i32 {
    let mut allow = match load_allow(c) {
        Ok(a) => a,
        Err(e) => return fail(&e),
    };
    let (mut code, mut repos) = (0, Vec::new());
    let mut left: Vec<Hit> = Vec::new();
    for d in &c.dirs {
        let hits = match scan(d, &c.git_ref, &c.old) {
            Ok(h) => h,
            Err(e) => {
                code = fail(&e);
                continue;
            }
        };
        let name = repo_name(d).unwrap_or_default();
        repos.push(name.clone());
        let kept: Vec<Hit> = hits.into_iter().filter(|h| !allow.allows(&h.repo, &h.path, &h.text)).collect();
        print_counts(&name, &kept);
        left.extend(kept.into_iter().filter(|h| !h.ghcr_only() && (all_classes || !h.class.is_cargo())));
    }
    let stale = allow.stale(&repos);
    for s in &stale {
        println!("ROT veralteter allow-Eintrag: {s}");
    }
    if code == 2 {
        return 2;
    }
    if check {
        for h in &left {
            println!("TREFFER {} {}:{}: {}", h.repo, h.path, h.line, h.text.trim());
        }
        println!("{} nachher: {} Treffer ausserhalb allow, {} veraltete allow-Eintraege", verdict(&left, &stale), left.len(), stale.len());
    }
    i32::from(!stale.is_empty() || (check && !left.is_empty()))
}

fn verdict(left: &[Hit], stale: &[String]) -> &'static str {
    if left.is_empty() && stale.is_empty() {
        "GREEN"
    } else {
        "RED"
    }
}

fn print_counts(repo: &str, hits: &[Hit]) {
    let mut per: BTreeMap<Class, usize> = BTreeMap::new();
    let (mut ghcr, mut total) = (0, 0);
    for h in hits {
        if h.ghcr_only() {
            ghcr += 1;
        } else {
            *per.entry(h.class).or_default() += 1;
            total += 1;
        }
    }
    for (class, n) in per {
        println!("{repo} {} {n}", class.letter());
    }
    if ghcr > 0 {
        println!("{repo} ghcr {ghcr}");
    }
    println!("{repo} gesamt {total}");
}
