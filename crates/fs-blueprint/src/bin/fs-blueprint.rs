//! Pattern: Facade (the command `fs-blueprint`: a thin `main` over `fs_blueprint::cli::run`).

use std::process::ExitCode;

fn main() -> ExitCode {
    let words: Vec<String> = std::env::args().skip(1).collect();
    let code = fs_blueprint::cli::run(&words, &mut std::io::stdout(), &mut std::io::stderr());
    ExitCode::from(u8::try_from(code).unwrap_or(2))
}
