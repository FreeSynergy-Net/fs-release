//! Pattern: Facade (command line in, one run out: `lift` prints the report path and the verdict, the flow commands print one line).

use std::path::{Path, PathBuf};

use clap::{Args, Parser, Subcommand};
use fs_release_chain::ctx::{Ctx, Timing};
use fs_release_chain::meta::Reach;
use fs_release_chain::outcome::Outcome;
use fs_release_chain::sys::{SystemClock, SystemRunner};

use crate::flow;
use crate::lift::{self, LiftOpts};
use crate::task::{run_all, NOT_RUN};

#[derive(Parser)]
#[command(
    name = "fs-release",
    version,
    about = "Lift a program onto new tags and walk it through review, tag and push"
)]
struct Cli {
    #[command(subcommand)]
    command: Sub,
}

#[derive(Subcommand)]
enum Sub {
    /// Lift dependencies, regenerate the lock, run lock gate, fmt, clippy pedantic, rules, tests; write the report.
    Lift(LiftArgs),
    /// Write the review request for Opus (needs a green lift report for the head).
    ReviewRequest(StepArgs),
    /// Set the local tag (needs the approval of the head).
    Tag(StepArgs),
    /// Push branch (--reach shared) and tag (--reach public); needs the approval and the pre-approved list.
    Push(StepArgs),
    /// After the Pre-release: start bump.yml of the catalog (--reach public); needs the approval and the pre-approved list.
    Bump(StepArgs),
    /// Every `cargo install` / `cargo binstall` / install-action line in `.github/` and `actions/` pins an exact version and uses --locked (rc 1 = a line to fix).
    CheckCiTools {
        /// The checkout to scan.
        #[arg(default_value = ".")]
        dir: PathBuf,
    },
    /// Chain gate: the code reached through the self-pins (release.yml -> _release-publish.yml -> publish-release ...) equals the code at the start head, except the pins (rc 1 = red, 2 = bad input).
    CheckPinChain(crate::pin_chain::PinChainArgs),
    /// `JA am <SHA>` only with the proof of the runner (cost-log row, run record, byte-equal report); rc 1 = refused (hand-written verdict).
    CheckApproval(ApprovalArgs),
    /// Pack the .mf2 texts of a program: pair check de/en, one catalog per language, the manifest and the date data blob (workflow modul-neu; the same command locally and in CI).
    PackTexts(crate::texts_pack::TextsPackArgs),
    /// Owner swap in GitHub addresses: bestand, ersetzen, nachher (workflow adresse-wechseln).
    Address(crate::address::cmd::AddressArgs),
    /// lift --commit, review-request, tag, push, bump in one run; halts until the approval and the Pre-release exist, run again to go on.
    Flow {
        #[command(flatten)]
        lift: LiftArgs,
        #[command(flatten)]
        flow: FlowArgs,
    },
}

#[derive(Args, Clone)]
struct LiftArgs {
    /// Worktree of the repo.
    repo: PathBuf,
    /// Tag of fs-components, `vX.Y.Z`.
    #[arg(long)]
    components: Option<String>,
    /// Tag of fs-interfaces, `vX.Y.Z` (the same as `--repo fs-interfaces=vX.Y.Z`).
    #[arg(long)]
    interfaces: Option<String>,
    /// Tag of fs-types, `vX.Y.Z` (the same as `--repo fs-types=vX.Y.Z`): the dependency also moves from the repo fs-interfaces to the repo fs-types.
    #[arg(long)]
    types: Option<String>,
    /// A library and its tag, `NAME=vX.Y.Z`, repeatable: fs-components, fs-interfaces, fs-types, fs-objects, fs-rpc, fs-jscalendar.
    #[arg(long = "repo", value_name = "NAME=TAG")]
    repos: Vec<String>,
    /// Use the real tags on GitHub instead of the local-tags git config.
    #[arg(long)]
    online: bool,
    /// Commit the changed Cargo.toml files after green steps (the flow does).
    #[arg(long)]
    commit: bool,
    /// The fs-release checkout (scripts, `RUST_VERSION`).
    #[arg(long, default_value = ".")]
    release_dir: PathBuf,
    /// Throw-away git config for local tags.
    #[arg(long, default_value = lift::LOCAL_TAGS)]
    git_config: PathBuf,
}

#[derive(Args, Clone)]
struct ApprovalArgs {
    /// The verdict file (`review-<topic>-opus.md`).
    verdict: PathBuf,
    /// The full 40-character SHA that is to be pushed.
    #[arg(long)]
    sha: String,
    /// The run id of the review run (`runid=` of run.sh, last column of the cost log).
    #[arg(long)]
    run_id: String,
    /// The folder of the runner (`api-kosten.log`, `out-*.json`).
    #[arg(long, default_value = "/srv/FreeSynergy/.scratch/api-runner")]
    runner_dir: PathBuf,
}

#[derive(Args, Clone)]
struct StepArgs {
    /// Worktree of the repo.
    repo: PathBuf,
    /// The fs-release checkout (pre-approved list).
    #[arg(long, default_value = ".")]
    release_dir: PathBuf,
    #[command(flatten)]
    flow: FlowArgs,
}

#[derive(Args, Clone)]
struct FlowArgs {
    /// Release version without `v` (0.0.9).
    #[arg(long)]
    version: String,
    #[arg(long, default_value = "main")]
    base: String,
    /// Highest reach: local, shared or public.
    #[arg(long, default_value = "local")]
    reach: String,
}

/// Parses the command line, runs it and returns the exit code.
#[must_use]
pub fn main_with(args: Vec<String>) -> i32 {
    let cli = match Cli::try_parse_from(args) {
        Ok(c) => c,
        Err(e) => {
            let _ = e.print();
            return if e.use_stderr() { 2 } else { 0 };
        }
    };
    match cli.command {
        Sub::Lift(a) => lift_cmd(&a),
        Sub::Address(a) => crate::address::cmd::run(&a),
        Sub::PackTexts(a) => crate::texts_pack::run(&a),
        Sub::ReviewRequest(s) => one(&s, 0),
        Sub::Tag(s) => one(&s, 1),
        Sub::Push(s) => one(&s, 2),
        Sub::Bump(s) => one(&s, 3),
        Sub::Flow { lift, flow } => full(&lift, &flow),
        Sub::CheckCiTools { dir } => crate::ci_tools::run(&dir),
        Sub::CheckPinChain(a) => crate::pin_chain::run(&a),
        Sub::CheckApproval(a) => check_approval(&a),
    }
}

fn opts(a: &LiftArgs) -> Result<LiftOpts, String> {
    let mut o = LiftOpts::new(a.repo.clone(), a.release_dir.clone());
    for (name, tag) in [
        ("fs-components", &a.components),
        ("fs-interfaces", &a.interfaces),
        ("fs-types", &a.types),
    ] {
        if let Some(t) = tag {
            o.set_lib(name, t)?;
        }
    }
    for r in &a.repos {
        o.set_lib_arg(r)?;
    }
    o.online = a.online;
    o.commit = a.commit;
    o.git_config.clone_from(&a.git_config);
    Ok(o)
}

fn lift_cmd(a: &LiftArgs) -> i32 {
    let o = match opts(a) {
        Ok(o) => o,
        Err(e) => {
            eprintln!("fs-release lift: {e}");
            return 2;
        }
    };
    let (rep, code) = lift::run(&o, &SystemRunner::default());
    let red: Vec<String> = rep
        .steps
        .iter()
        .filter(|s| s.status == "red")
        .map(|s| format!("{}: {}", s.name, s.first_error.as_deref().unwrap_or("")))
        .collect();
    println!(
        "{} lift {} {:.0}s report {}/lift-report.md",
        if rep.ok { "GREEN" } else { "RED" },
        rep.repo,
        rep.secs,
        o.report_dir().display()
    );
    for r in red {
        println!("  {r}");
    }
    for s in rep
        .steps
        .iter()
        .filter(|s| s.status == "skipped" && s.detail != NOT_RUN)
    {
        println!("  WARNING {} skipped: {}", s.name, s.detail);
    }
    code
}

fn with_ctx(
    f: &FlowArgs,
    dir: &Path,
    release_dir: &Path,
    body: impl FnOnce(&Ctx) -> Outcome,
) -> i32 {
    let runner = SystemRunner::default();
    let clock = SystemClock::new();
    let line = |o: Outcome| {
        println!("{}", o.line());
        o.code()
    };
    let red = |why: String| {
        line(Outcome::Red {
            step: "setup".into(),
            line: why,
        })
    };
    let Some(reach) = Reach::parse(&f.reach) else {
        return red(format!("--reach {}: use local, shared or public", f.reach));
    };
    let target = match flow::build_target(&runner, dir, release_dir, &f.version, &f.base, None) {
        Ok(t) => t,
        Err(e) => return red(e),
    };
    let ctx = Ctx::new(target, reach, Timing::standard(), &runner, &clock);
    line(body(&ctx))
}

/// `which`: 0 review-request, 1 tag, 2 push, 3 bump (the one step, with all that must hold before it).
fn one(s: &StepArgs, which: usize) -> i32 {
    with_ctx(&s.flow, &s.repo, &s.release_dir, |ctx| {
        let mut all = flow::tasks();
        let list = vec![all.remove(which)];
        let (records, first) = run_all(&list, ctx);
        match first {
            None => Outcome::Done {
                summary: records[0].detail.clone(),
            },
            Some((name, stop)) => Outcome::from_stop(&name, stop),
        }
    })
}

fn full(l: &LiftArgs, f: &FlowArgs) -> i32 {
    let mut a = l.clone();
    a.commit = true;
    let o = match opts(&a) {
        Ok(o) => o,
        Err(e) => {
            eprintln!("fs-release flow: {e}");
            return 2;
        }
    };
    with_ctx(f, &l.repo, &l.release_dir, |ctx| {
        flow::run(&o, ctx, &SystemRunner::default())
    })
}

fn check_approval(a: &ApprovalArgs) -> i32 {
    let (verdict, log) = (
        std::fs::read_to_string(&a.verdict),
        std::fs::read_to_string(a.runner_dir.join("api-kosten.log")),
    );
    let (Ok(verdict), Ok(log)) = (verdict, log) else {
        println!("RED check-approval: verdict file or cost log not readable");
        return 1;
    };
    let records = crate::approval::run_records(&a.runner_dir, &log, &a.run_id);
    let ev = crate::approval::Evidence {
        verdict: &verdict,
        sha: &a.sha,
        run_id: &a.run_id,
        log: &log,
        records: &records,
    };
    match crate::approval::verify(&ev) {
        Ok(()) => {
            println!(
                "GREEN check-approval: JA am {} has a runner proof ({})",
                a.sha, a.run_id
            );
            0
        }
        Err(e) => {
            println!("RED check-approval: {e}");
            1
        }
    }
}
