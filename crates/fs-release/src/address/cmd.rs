//! Pattern: Facade (`fs-release address bestand|ersetzen|nachher|aufrufer`; every error is exit code 2 with a message, red is 1).

use std::collections::BTreeMap;
use std::path::PathBuf;

use clap::{Args, Subcommand};

use super::allow::Allow;
use super::aufrufer::{Ctx, Report, Rules};
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
    /// Callers: every fs-release `uses:` names the exact owner and a ref that contains owner-neutral and lies on origin/main.
    Aufrufer(Aufrufer),
}

#[derive(Args)]
struct Aufrufer {
    /// The owner-neutral commit on main of fs-release (40 hex, no default).
    #[arg(long)]
    neutral: String,
    /// Local fs-release clone (fetched by the caller; read: origin/main and tags).
    #[arg(long = "fs-release")]
    fs_release: PathBuf,
    /// State of the caller repos to read.
    #[arg(long = "ref", default_value = "HEAD")]
    git_ref: String,
    /// Caller repo directories.
    #[arg(required = true)]
    dirs: Vec<PathBuf>,
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
    /// With `--fs-release`: also run the `aufrufer` rule on the same repos and ref (owner-neutral commit).
    #[arg(long, requires = "fs_release")]
    neutral: Option<String>,
    /// fs-release clone for the `aufrufer` rule.
    #[arg(long = "fs-release", requires = "neutral")]
    fs_release: Option<PathBuf>,
}

/// Runs the command, prints the result and returns the exit code (0 ok, 1 red, 2 error).
#[must_use]
pub fn run(args: &AddressArgs) -> i32 {
    match &args.action {
        Action::Bestand(c) => stock(c, false, false),
        Action::Nachher(n) => nachher(n),
        Action::Aufrufer(a) => aufrufer(&a.neutral, &a.fs_release, &a.git_ref, &a.dirs),
        Action::Ersetzen(c) => replace(c),
    }
}

fn load_allow(c: &Common) -> Result<Allow, String> {
    c.allow
        .as_deref()
        .map_or_else(|| Ok(Allow::none()), |p| Allow::load(p, &c.old))
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

fn nachher(n: &Nachher) -> i32 {
    let code = stock(&n.common, true, n.all_classes);
    let (Some(neutral), Some(clone)) = (&n.neutral, &n.fs_release) else {
        println!("aufrufer: NICHT GEPRUEFT (--neutral/--fs-release fehlen)");
        return code;
    };
    let rc = aufrufer(neutral, clone, &n.common.git_ref, &n.common.dirs);
    if code == 2 || rc == 2 {
        2
    } else {
        i32::from(code != 0 || rc != 0)
    }
}

fn aufrufer(neutral: &str, clone: &std::path::Path, git_ref: &str, dirs: &[PathBuf]) -> i32 {
    let ctx = match Ctx::new(clone, neutral, Rules::strict()) {
        Ok(c) => c,
        Err(e) => return fail(&e),
    };
    let mut rep = Report {
        lines: Vec::new(),
        quiet_repos: Vec::new(),
    };
    for d in dirs {
        if let Err(e) = ctx.check_repo(d, git_ref, &mut rep) {
            return fail(&e);
        }
    }
    print!("{}", rep.text(dirs.len() - rep.quiet_repos.len()));
    i32::from(rep.red() > 0)
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
        let kept: Vec<Hit> = hits
            .into_iter()
            .filter(|h| !allow.allows(&h.repo, &h.path, &h.text))
            .collect();
        print_counts(&name, &kept);
        left.extend(
            kept.into_iter()
                .filter(|h| !h.ghcr_only() && (all_classes || !h.class.is_cargo())),
        );
    }
    // a global entry (no `repo`) is judged only when the run covers several repos: one repo alone cannot show that it is stale.
    let stale = allow.stale(&repos, c.dirs.len() > 1);
    for s in &stale {
        println!("ROT veralteter allow-Eintrag: {s}");
    }
    if code == 2 {
        return 2;
    }
    if check {
        for h in &left {
            println!(
                "TREFFER {} {}:{}: {}",
                h.repo,
                h.path,
                h.line,
                h.text.trim()
            );
        }
        println!(
            "{} nachher: {} Treffer ausserhalb allow, {} veraltete allow-Eintraege",
            verdict(&left, &stale),
            left.len(),
            stale.len()
        );
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
