//! Pattern: Facade (command line in, one line and an exit code out).

use clap::{Parser, Subcommand};

use crate::publish::{publish, PublishArgs};

#[derive(Parser)]
#[command(
    name = "fs-crate-index",
    version,
    about = "Write one crate version into the git index registry (tag run only)"
)]
struct Cli {
    #[command(subcommand)]
    command: Sub,
}

#[derive(Subcommand)]
enum Sub {
    /// Package the crate, add the index line and the `.crate` file to the registry checkout, commit there (no push).
    Publish(PublishArgs),
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
        Sub::Publish(a) => match publish(&a) {
            Ok(line) => {
                println!("GREEN crate-index {line}");
                0
            }
            Err(e) => {
                println!("RED crate-index: {e:#}");
                1
            }
        },
    }
}
