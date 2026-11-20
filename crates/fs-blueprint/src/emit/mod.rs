//! Pattern: Template method + Strategy (`generate` runs every emitter, stamps the heads, sorts by path and sets the class from the blueprint; one emitter per kind of output).
//!
//! The same blueprint gives byte-identical output: no time stamp, no machine path, no map with a random order, `/` in every path on every platform.

pub mod artifact;
pub mod check;
pub mod contract;
pub mod dirstore;
pub mod files;
pub mod frontend;
pub mod generate;
pub mod header;
pub mod shape;
pub mod skeleton;
pub mod slot;
pub mod write;

pub use artifact::{join, Artifact, Class, Emitter};
pub use generate::{emitters, generate, generate_with};
