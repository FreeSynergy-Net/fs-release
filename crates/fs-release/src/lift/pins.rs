//! Pattern: Strategy (the step `pins`, before `lock-sources`: raise the fs-release pins of the lifted repo to the first tag with the lock check).

use fs_release_chain::outcome::Stop;

use super::checks::red_line;
use super::LiftEnv;
use crate::task::{Task, SKIPPED};

/// The tag the pins are raised to (the lock check exists from there; see `lockpin`).
const TAG: &str = "v0.0.10";

/// `scripts/pin-refs.sh --dir <repo> v0.0.10`: every `uses: FreeSynergy/fs-release/...@<ref>` of the repo's `.github/workflows/*.yml`
/// (`@main`, an old tag or a commit) becomes `@v0.0.10`; other actions stay. No workflows = skipped. A repo whose `check.yml` is
/// already on a tag of v0.0.10 or newer is left alone (a pin is never lowered). It changes files; the step `commit` takes them along.
pub struct Pins;

impl Task<LiftEnv<'_>> for Pins {
    fn name(&self) -> &'static str {
        "pins"
    }

    fn continue_on_red(&self) -> bool {
        true
    }

    fn run(&self, env: &LiftEnv) -> Result<String, Stop> {
        let dir = &env.opts.repo_dir;
        if !dir.join(".github/workflows").is_dir() {
            return Ok(format!("{SKIPPED}no .github/workflows in the repo"));
        }
        let check = std::fs::read_to_string(dir.join(".github/workflows/check.yml")).ok();
        if check.as_deref().is_some_and(super::lockpin::already_new) {
            return Ok(format!("already on {TAG} or newer: nothing to pin"));
        }
        let args = vec![
            env.script("pin-refs.sh"),
            "--dir".to_string(),
            dir.display().to_string(),
            TAG.to_string(),
        ];
        let o = env.exec("bash", args, vec![])?;
        if o.ok() {
            Ok(o.stdout
                .lines()
                .last()
                .unwrap_or("pinned")
                .trim()
                .to_string())
        } else {
            Err(Stop::red(red_line(&o)))
        }
    }
}
