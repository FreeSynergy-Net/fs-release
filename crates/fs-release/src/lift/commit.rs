//! Pattern: Strategy (the optional last step: commit what the lift changed, so a review has a commit to look at).

use fs_release_chain::outcome::Stop;

use super::LiftEnv;
use crate::task::Task;

/// The identity of every lift commit (house rule: author and committer are the project, never a person).
pub const NAME: &str = "FreeSynergy";
/// The public address of the project.
pub const EMAIL: &str = "support@freesynergy.net";
/// Environment variable with the `Co-Authored-By` value of the footer.
pub const COAUTHOR_ENV: &str = "FS_LIFT_COAUTHOR";
/// Footer value when the variable is not set.
pub const COAUTHOR_DEFAULT: &str = "Claude Sonnet 5.5 <noreply@anthropic.com>";

/// `Name <local@host>`, one line.
fn is_coauthor(v: &str) -> bool {
    let one_line = !v.contains(['\n', '\r']);
    v.strip_suffix('>')
        .and_then(|r| r.split_once(" <"))
        .is_some_and(|(n, m)| {
            one_line
                && !n.trim().is_empty()
                && m.split_once('@')
                    .is_some_and(|(l, h)| !l.is_empty() && !h.is_empty())
        })
}

/// The footer value: `FS_LIFT_COAUTHOR` (lift options, then process environment) or the default.
fn coauthor(env: &LiftEnv) -> Result<String, Stop> {
    let set = env
        .opts
        .env
        .iter()
        .find(|(k, _)| k == COAUTHOR_ENV)
        .map(|(_, v)| v.clone())
        .or_else(|| std::env::var(COAUTHOR_ENV).ok());
    let v = set.map_or_else(|| COAUTHOR_DEFAULT.to_string(), |v| v.trim().to_string());
    if is_coauthor(&v) {
        Ok(v)
    } else {
        Err(Stop::red(format!(
            "{COAUTHOR_ENV} is {v:?}: it must look like `Name <mail@host>` (the Co-Authored-By footer is part of the house rule); nothing committed"
        )))
    }
}

/// The git identity variables that are set anywhere (lift options or process environment) beat `-c user.name`: for those the house
/// identity is set explicitly, so a caller's `GIT_AUTHOR_NAME` can never turn into the author of a lift commit.
fn identity_env(env: &LiftEnv) -> Vec<(String, String)> {
    [
        ("GIT_AUTHOR_NAME", NAME),
        ("GIT_AUTHOR_EMAIL", EMAIL),
        ("GIT_COMMITTER_NAME", NAME),
        ("GIT_COMMITTER_EMAIL", EMAIL),
    ]
    .into_iter()
    .filter(|(k, _)| env.opts.env.iter().any(|(e, _)| e == k) || std::env::var_os(k).is_some())
    .map(|(k, v)| (k.to_string(), v.to_string()))
    .collect()
}

/// Commits the changed `Cargo.toml` files, the workflow files the step `pins` changed, and `Cargo.lock` only after an online lift (a local-tags lock is not committed).
/// The identity is set explicitly (`git -c user.name=FreeSynergy -c user.email=support@freesynergy.net commit`, also against `GIT_AUTHOR_*`
/// in the environment) and the message ends with `Co-Authored-By: <FS_LIFT_COAUTHOR>`; an invalid footer value stops the step before anything is committed.
/// Runs only when every step before it was green.
pub struct Commit;

impl Task<LiftEnv<'_>> for Commit {
    fn name(&self) -> &'static str {
        "commit"
    }

    fn needs_green(&self) -> bool {
        true
    }

    fn run(&self, env: &LiftEnv) -> Result<String, Stop> {
        let git = |args: &[&str]| {
            env.exec(
                "git",
                args.iter().map(|a| (*a).to_string()).collect(),
                vec![],
            )
        };
        let footer = format!("Co-Authored-By: {}", coauthor(env)?);
        let list = git(&["diff", "--name-only"])?;
        let files: Vec<&str> = list
            .stdout
            .lines()
            .filter(|f| {
                f.ends_with("Cargo.toml")
                    || (env.opts.online && *f == "Cargo.lock")
                    || (f.starts_with(".github/workflows/") && f.ends_with("yml"))
            })
            .collect();
        if files.is_empty() {
            return Ok("nothing to commit (already on these tags)".into());
        }
        let mut add = vec!["add", "--"];
        add.extend(files.iter().copied());
        let o = git(&add)?;
        if !o.ok() {
            return Err(Stop::red(format!("git add: {}", o.first_error())));
        }
        let (c, i) = (
            env.opts.components.as_deref(),
            env.opts.interfaces.as_deref(),
        );
        let what: Vec<String> = [("fs-components", c), ("fs-interfaces", i)]
            .into_iter()
            .filter_map(|(n, t)| t.map(|t| format!("{n} {t}")))
            .collect();
        let o = env.exec(
            "git",
            [
                "-c",
                &format!("user.name={NAME}"),
                "-c",
                &format!("user.email={EMAIL}"),
                "commit",
                "--quiet",
                "-m",
                &format!("lift to {}", what.join(", ")),
                "-m",
                &footer,
            ]
            .iter()
            .map(|a| (*a).to_string())
            .collect(),
            identity_env(env),
        )?;
        if !o.ok() {
            return Err(Stop::red(format!("git commit: {}", o.first_error())));
        }
        Ok(format!("committed {} file(s)", files.len()))
    }
}
