//! Pattern: none (module list of the owner-swap tool `fs-release address`, workflow `adresse-wechseln`).
//!
//! One duty per file: `mention` (what counts as an owner mention), `class` (class of a path), `allow` (what may stay),
//! `scan` (git grep on a ref), `replace` (work-tree rewrite), `aufrufer` (caller rule), `cmd` (the commands).

pub mod allow;
pub mod aufrufer;
pub mod class;
pub mod cmd;
pub mod mention;
pub mod replace;
pub mod scan;
pub mod uses_line;
