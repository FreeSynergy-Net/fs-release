//! Pattern: none (module list and re-exports of the blueprint library: model, check, rules, error codes and messages).
//!
//! `fs-blueprint` (ADR 0099 addendum 2026-11-18): the blueprint `scaffold.toml` (schema 2 only) as a checked value object and the check of it.
//! S1 of the plan: model and check. S2: the generator core (`emit`: artifacts, slots, head lines, `generate`, drift `check`, `write`). Findings are codes with parameters; the sentences are messages in de and en.

pub mod catalog_file;
pub mod check;
pub mod check_data;
pub mod check_exc;
pub mod check_front;
pub mod check_nested;
pub mod check_layout;
pub mod check_types;
pub mod cli;
pub mod cli_fmt;
pub mod cli_new;
pub mod emit;
pub mod error;
pub mod fmt_order;
pub mod form;
pub mod kit;
pub mod load;
pub mod messages;
pub mod mf2;
pub mod model;
pub mod repo_fs;
pub mod rules;
pub mod sink;
pub mod verified;

pub use catalog_file::FileCatalog;
pub use check::{check, check_source};
pub use emit::{generate, Artifact, Class};
pub use error::{Coded, Report, CODES};
pub use kit::{BundleInfo, Kind, KitCatalog, ModuleInfo, RepoView};
pub use model::{Blueprint, SCHEMA};
pub use repo_fs::FsRepo;
pub use rules::{Mode, Rule, Rules};
pub use verified::{verify, Verified};
