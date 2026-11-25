//! Pattern: Strategy (the step `pins`, before `lock-sources`: raise the fs-release pins of the lifted repo to the release tag, never lower one).

use fs_release_chain::outcome::Stop;

use super::checks::red_line;
use super::LiftEnv;
use crate::task::{Task, SKIPPED};

/// Environment variable that overrides the tag the pins are raised to (`vX.Y.Z`).
pub const TAG_ENV: &str = "FS_LIFT_PIN_TAG";

/// Environment variable the script `pin-refs.sh` reads for the built `fs-release` that runs its chain gate (`check-pin-chain`).
pub const BIN_ENV: &str = "FS_RELEASE_BIN";

/// `vX.Y.Z` of exactly this string.
fn is_tag(t: &str) -> bool {
    t.strip_prefix('v').is_some_and(|r| {
        let p: Vec<&str> = r.split('.').collect();
        p.len() == 3
            && p.iter()
                .all(|x| !x.is_empty() && x.bytes().all(|b| b.is_ascii_digit()))
    })
}

/// The tag the pins are raised to: `FS_LIFT_PIN_TAG` (in the lift options or the process environment) when it is a tag, else `v<version of
/// this crate>`. The crate version is the release tag (rule "version = tag"), so the release bump to the next tag moves the pin
/// with no code change.
#[must_use]
pub fn tag(env: &LiftEnv) -> String {
    env.opts
        .env
        .iter()
        .find(|(k, _)| k == TAG_ENV)
        .map(|(_, v)| v.clone())
        .or_else(|| std::env::var(TAG_ENV).ok())
        .filter(|v| is_tag(v))
        .unwrap_or_else(|| format!("v{}", env!("CARGO_PKG_VERSION")))
}

/// `scripts/pin-refs.sh --dir <repo> <tag>`: (owner rewritten to FreeSynergy-Net) every `uses: FreeSynergy-Net/fs-release/...@<ref>` of the repo's `.github/workflows/*.yml`
/// (`@main`, an older tag or a commit) becomes `@<tag>`; other actions stay. No workflows = skipped. A pin is never lowered, the whole run is red (not line by
/// line): the script checks (`FS_RELEASE_REPO`, default its own repo) in the fs-release clone that the tag equals or descends from every existing pin (git merge-base --is-ancestor); a lower, unrelated or unknown pin = red (rc 3), nothing written. Before writing, the script runs the hard chain gate
/// (`fs-release check-pin-chain <sha>`) with the running binary (`FS_RELEASE_BIN`, set here unless the caller or the environment names one): red or not runnable = red (rc 4), nothing written. It changes
/// files; the step `commit` takes them along.
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
        let args = vec![
            env.script("pin-refs.sh"),
            "--dir".to_string(),
            dir.display().to_string(),
            tag(env),
        ];
        // The chain gate of the script needs a built `fs-release`: lift IS one, so it hands itself over (unless the caller named another).
        let named =
            env.opts.env.iter().any(|(k, _)| k == BIN_ENV) || std::env::var_os(BIN_ENV).is_some();
        let extra = match std::env::current_exe() {
            Ok(exe) if !named => vec![(BIN_ENV.to_string(), exe.display().to_string())],
            _ => vec![],
        };
        let o = env.exec("bash", args, extra)?;
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

#[cfg(test)]
mod tests {
    use super::is_tag;

    #[test]
    fn only_vx_y_z_is_a_tag() {
        assert!(is_tag("v0.0.11") && is_tag("v10.2.300"));
        for bad in [
            "0.0.1", "main", "v1.2", "v1.2.3.4", "v1..3", "v1.2.x", "v1.2.3;", "",
        ] {
            assert!(!is_tag(bad), "{bad}");
        }
    }
}
