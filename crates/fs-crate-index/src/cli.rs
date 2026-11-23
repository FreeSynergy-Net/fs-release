//! Pattern: Facade (command line in, one line and an exit code out).

use clap::{Parser, Subcommand};

use crate::apply::{apply, ApplyArgs};
use crate::publish::{publish, PublishArgs};
use crate::scan::{scan, ScanArgs};
use crate::wait::{wait, WaitArgs};

#[derive(Parser)]
#[command(
    name = "fs-crate-index",
    version,
    about = "Write crate versions into the git index registry (pull path: scan = job 1, apply = job 2)"
)]
struct Cli {
    #[command(subcommand)]
    command: Sub,
}

#[derive(Subcommand)]
enum Sub {
    /// Job 1 (read only): new tags of the allowed repos -> checks, `cargo package`, semver gate -> plan and `.crate` files.
    Scan(ScanArgs),
    /// Job 2: enter the planned crates into the registry checkout, one commit each (no push).
    Apply(ApplyArgs),
    /// Enter ONE packaged `.crate` (used by `apply`; also the unit of the tests).
    Publish(PublishArgs),
    /// Wait for a version to show up in the index; after the time a clear blocker text.
    Wait(WaitArgs),
}

fn report<T>(what: &str, r: anyhow::Result<T>, ok: impl FnOnce(T) -> String) -> i32 {
    match r {
        Ok(v) => {
            println!("GREEN {what} {}", ok(v));
            0
        }
        Err(e) => {
            println!("RED {what}: {e:#}");
            1
        }
    }
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
        Sub::Publish(a) => report("crate-index", publish(&a), |l| l),
        Sub::Apply(a) => report("crate-index apply", apply(&a), |v| {
            format!("{} entered\n{}", v.len(), v.join("\n"))
        }),
        Sub::Wait(a) => report("crate-index wait", wait(&a), |()| "in the index".into()),
        Sub::Scan(a) => match scan(&a) {
            Ok(r) => {
                for n in &r.notes {
                    println!("note {n}");
                }
                println!("entries={}", r.entries);
                if let Some(e) = r.error {
                    println!("RED crate-index scan: {e}");
                    1
                } else {
                    println!("GREEN crate-index scan: {} crate(s) planned", r.entries);
                    0
                }
            }
            Err(e) => {
                println!("RED crate-index scan: {e:#}");
                1
            }
        },
    }
}
