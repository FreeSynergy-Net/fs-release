//! Pattern: Facade (the command line of `texte-packen` in, the packer of `fs-texts-pack` behind it; rc 0 packed, rc 1 the texts are not sound or the output
//! cannot be written, rc 2 bad input).
//!
//! `fs-release texte-packen --quellen DIR --ausgabe DIR [--sprachen de,en]`: the same command on the developer machine and in CI.

use std::path::PathBuf;

use clap::Args;
use fs_texts_pack::PackError;

/// Arguments of `texte-packen`.
#[derive(Args, Clone)]
pub struct TextsPackArgs {
    /// Folder with the sources `<language>.mf2` of one program (stay in the repo, they are the source of truth).
    #[arg(long)]
    quellen: PathBuf,
    /// Output folder: `manifest.mf2m`, `<locale>.<hash>.mf2b` and `dates/<locale>/icu.table`.
    #[arg(long)]
    ausgabe: PathBuf,
    /// Only these languages are written (`de,en`); the check always covers all sources. Default: all.
    #[arg(long, value_delimiter = ',')]
    sprachen: Vec<String>,
}

/// Runs the command; the result is the exit code.
#[must_use]
pub fn run(a: &TextsPackArgs) -> i32 {
    let packed = fs_texts_pack::read_dir(&a.quellen)
        .and_then(|sources| fs_texts_pack::pack(&sources, &a.sprachen))
        .and_then(|p| p.write(&a.ausgabe).map(|()| p));
    match packed {
        Ok(p) => {
            for (name, bytes) in &p.files {
                println!("{name} {} bytes", bytes.len());
            }
            println!(
                "GREEN texte-packen: {} files in {}",
                p.files.len(),
                a.ausgabe.display()
            );
            0
        }
        Err(e @ PackError::NoSuchLanguage(_)) => {
            eprintln!("RED texte-packen: {e}");
            2
        }
        Err(e) => {
            eprintln!("RED texte-packen: {e}");
            1
        }
    }
}
