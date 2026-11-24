//! `fs-texts-pack`: packs the text sources of a program once, at build time (ADR 0057, decision 9f 24.11.), so that the running program only loads.
//!
//! `fs-release pack-texts --sources DIR --out DIR [--locales de,en]` reads `DIR/<language>.mf2` (the source of truth, it stays in the repo),
//! checks the pairs (every message in every language, the same variables, every message parses), and writes with the `mf2` 3.0.0 building blocks
//! `manifest.mf2m`, `<locale>.<hash>.mf2b` per language and, apart, `dates/<locale>/icu.table` (Gregorian, no zones). The same command runs on the
//! developer machine and in CI and gives the same bytes.
//!
//! Pattern: Builder (sources -> check -> manifest -> catalogs and date blobs); this file only declares modules and re-exports.

#![deny(missing_docs)]
#![warn(clippy::pedantic)]
#![allow(clippy::module_name_repetitions)]

mod catalog;
mod check;
mod dates;
mod error;
mod locale;
mod pack;
mod source;

pub use error::PackError;
pub use pack::{pack, Packed};
pub use source::{read_dir, Source};
