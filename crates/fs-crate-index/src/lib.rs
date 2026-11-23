//! Pattern: Adapter (our guards around the `reg-index` library; the index format itself is not ours).
//!
//! PULL path (ADR 0093, decision of the strategy: no secret, no right across repos): a workflow IN the registry repo `FreeSynergy-Net/fs-crates`
//! (template: `templates/fs-crates/`) runs on a schedule. Job 1 (`contents: read`) calls `scan`: `git ls-remote --tags` of the repos on the allow list,
//! checks, `cargo package --locked`, semver gate with `--baseline-root` on the newest `.crate` of the own checkout. Job 2 (`contents: write`) calls
//! `apply`: no cargo, only the entries (`publish` per crate, dependency order) and one push. The token never meets `cargo build` or `cargo package`.
//!
//! Guards (threat model: protects against MISTAKES, not against someone who can change workflow, toolchain or scripts; whoever can write
//! to the registry repo can swap crates, the checksum lies in the same repo):
//! - source repo on the allow list; the tag is annotated; the tag commit is on `main` of the source repo; tag = version of every package;
//! - a version already in the index is never replaced; a tag that moved after its entry is reported red and entered again never (the tag commit is in the commit message);
//! - `config.json` of the registry exists and its `dl` leads to exactly the place where the `.crate` is stored;
//! - features with `dep:` or `?/` are written in index format v2 (`features2`, `v: 2`).

pub mod allow;
pub mod apply;
pub mod cli;
pub mod guard;
pub mod publish;
pub mod scan;
pub mod wait;

pub use allow::{list_tags, parse_allow, Source};
pub use apply::{apply, ApplyArgs};
pub use cli::main_with;
pub use guard::{index_file, split_v2, version_of_ref};
pub use publish::{check_dl, package, publish, PublishArgs};
pub use scan::{scan, ScanArgs};
pub use wait::{wait, WaitArgs};
