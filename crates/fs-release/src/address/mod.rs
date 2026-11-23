//! Pattern: none (module list of the owner-swap tool `fs-release address`, workflow `adresse-wechseln`).
//!
//! One duty per file: `mention` (what counts as an owner mention), `class` (class of a path), `allow` (what may stay),
//! `scan` (git grep on a ref), `replace` (work-tree rewrite), `cmd` (the three commands).

pub mod allow;
pub mod class;
pub mod cmd;
pub mod mention;
pub mod replace;
pub mod scan;
