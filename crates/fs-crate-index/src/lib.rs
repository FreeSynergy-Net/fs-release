//! Pattern: Adapter (our guards around the `reg-index` library; the index format itself is not ours).
//!
//! `fs-crate-index publish` puts ONE crate version into the git index registry `freesynergy` (repo `FreeSynergy-Net/fs-crates`):
//! `cargo package --locked` -> `.crate` -> index line (name, version, dependencies, features, SHA-256 checksum) in the layout of the
//! cargo registry index -> the `.crate` file under `crates/{crate}/` -> ONE commit. Pushing is the job of the workflow, not of this tool.
//!
//! Guards (threat model: protects against MISTAKES, not against someone who can change workflow, toolchain or scripts; whoever can write
//! to the registry repo can swap crates, the checksum lies in the same repo):
//! - only a tag run: `--git-ref` must be `refs/tags/vX.Y.Z`; a branch, a pull request or nothing writes nothing;
//! - tag = version of the package (as for every release);
//! - `config.json` of the registry exists and its `dl` leads to exactly the place where the `.crate` is stored;
//! - a version already in the index is never replaced (`reg-index` refuses).

pub mod cli;
pub mod guard;
pub mod publish;

pub use cli::main_with;
pub use guard::{check_features, version_of_ref};
pub use publish::{check_dl, publish, PublishArgs};
