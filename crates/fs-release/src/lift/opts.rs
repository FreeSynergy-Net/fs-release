//! Pattern: Value object (what one lift is asked to do, and what a lift step sees while it runs).

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

use fs_release_chain::outcome::Stop;
use fs_release_chain::sys::{Cmd, Out, Runner};

/// The shared build slots: every cargo call of a lift goes through this script when it exists.
pub const BUILD_SLOT: &str = "/srv/FreeSynergy/.scratch/build-slot.sh";
/// Throw-away git config that maps the GitHub URLs of fs-components / fs-interfaces to the local repos (tags not pushed yet).
pub const LOCAL_TAGS: &str = "/srv/FreeSynergy/.scratch/gitconfig-local-tags";

/// The libraries a lift can move, in the order the steps handle them (ADR 0092/0093: `fs-types` is its own repo from V1+).
pub const LIBS: [&str; 6] = [
    "fs-components",
    "fs-interfaces",
    "fs-types",
    "fs-objects",
    "fs-rpc",
    "fs-jscalendar",
];

/// What one lift is asked to do.
#[derive(Debug, Clone)]
pub struct LiftOpts {
    /// Worktree of the repo (the one that gets lifted).
    pub repo_dir: PathBuf,
    /// `vX.Y.Z` for the fs-components dependency (None: leave it).
    pub components: Option<String>,
    /// `vX.Y.Z` for the fs-interfaces dependency (None: leave it).
    pub interfaces: Option<String>,
    /// `vX.Y.Z` for the other libraries (`fs-types`, `fs-objects`, `fs-rpc`, `fs-jscalendar`), by name; see [`LiftOpts::set_lib`].
    pub repos: BTreeMap<String, String>,
    /// Real tags from GitHub. Without it the local-tags git config is used (nothing leaves this machine).
    pub online: bool,
    /// The fs-release checkout (scripts, `RUST_VERSION`).
    pub release_dir: PathBuf,
    /// Git config for local tags.
    pub git_config: PathBuf,
    /// Program (and arguments) put in front of every cargo call.
    pub wrapper: Vec<String>,
    /// Alternative list for the Blinc lock gate (tests); default: the one next to the script.
    pub approved: Option<PathBuf>,
    /// Extra environment for every program (tests).
    pub env: Vec<(String, String)>,
    /// After green steps: commit the changed `Cargo.toml` files (and `Cargo.lock` when online).
    pub commit: bool,
}

impl LiftOpts {
    /// Options with the production defaults.
    #[must_use]
    pub fn new(repo_dir: PathBuf, release_dir: PathBuf) -> LiftOpts {
        LiftOpts {
            repo_dir,
            components: None,
            interfaces: None,
            repos: BTreeMap::new(),
            online: false,
            release_dir,
            git_config: PathBuf::from(LOCAL_TAGS),
            wrapper: if Path::new(BUILD_SLOT).is_file() {
                vec![BUILD_SLOT.to_string()]
            } else {
                vec![]
            },
            approved: None,
            env: vec![],
            commit: false,
        }
    }

    /// Asks for `name` at `tag` (`--repo fs-types=v0.0.1`). The two long-standing options are the same thing under their own names.
    ///
    /// # Errors
    /// The name is not one of [`LIBS`], or the library was asked for twice (also through `--components` / `--interfaces` / `--types`).
    pub fn set_lib(&mut self, name: &str, tag: &str) -> Result<(), String> {
        if !LIBS.contains(&name) {
            return Err(format!(
                "unknown library {name:?}: use one of {}",
                LIBS.join(", ")
            ));
        }
        let slot = match name {
            "fs-components" => &mut self.components,
            "fs-interfaces" => &mut self.interfaces,
            _ => {
                if self.repos.contains_key(name) {
                    return Err(format!("{name} given twice"));
                }
                self.repos.insert(name.to_string(), tag.to_string());
                return Ok(());
            }
        };
        if slot.is_some() {
            return Err(format!("{name} given twice"));
        }
        *slot = Some(tag.to_string());
        Ok(())
    }

    /// Splits `NAME=TAG` and calls [`set_lib`](Self::set_lib).
    ///
    /// # Errors
    /// No `=`, or what `set_lib` says.
    pub fn set_lib_arg(&mut self, arg: &str) -> Result<(), String> {
        let (name, tag) = arg
            .split_once('=')
            .ok_or_else(|| format!("--repo {arg:?}: use NAME=vX.Y.Z (e.g. fs-types=v0.0.1)"))?;
        self.set_lib(name.trim(), tag.trim())
    }

    /// Every library the lift is asked to move, with its tag, in the order of [`LIBS`].
    #[must_use]
    pub fn wanted(&self) -> Vec<(&'static str, String)> {
        LIBS.iter()
            .filter_map(|&n| {
                let t = match n {
                    "fs-components" => self.components.clone(),
                    "fs-interfaces" => self.interfaces.clone(),
                    _ => self.repos.get(n).cloned(),
                };
                t.map(|t| (n, t))
            })
            .collect()
    }

    /// Where the report goes.
    #[must_use]
    pub fn report_dir(&self) -> PathBuf {
        self.repo_dir.join(".fs-release")
    }
}

/// What a lift step sees.
pub struct LiftEnv<'a> {
    pub opts: &'a LiftOpts,
    pub runner: &'a dyn Runner,
}

impl LiftEnv<'_> {
    /// Runs a program in the worktree with the extra environment.
    ///
    /// # Errors
    /// [`Stop::Red`] when the program cannot be started.
    pub fn exec(
        &self,
        program: &str,
        args: Vec<String>,
        env: Vec<(String, String)>,
    ) -> Result<Out, Stop> {
        let mut all = self.opts.env.clone();
        all.extend(env);
        let cmd = Cmd {
            program: program.into(),
            args,
            cwd: Some(self.opts.repo_dir.clone()),
            env: all,
        };
        self.runner
            .run(&cmd)
            .map_err(|e| Stop::red(format!("cannot start {program}: {e}")))
    }

    /// Runs `cargo <args>` through the wrapper, with the pinned toolchain and (offline) the local tags.
    ///
    /// # Errors
    /// [`Stop::Red`] when cargo cannot be started.
    pub fn cargo(&self, args: &[&str]) -> Result<Out, Stop> {
        self.cargo_on(args, None)
    }

    /// Like [`cargo`](Self::cargo), on the given toolchain instead of the pinned one (`stable`: what GitHub CI uses).
    ///
    /// # Errors
    /// [`Stop::Red`] when cargo cannot be started.
    pub fn cargo_on(&self, args: &[&str], toolchain: Option<&str>) -> Result<Out, Stop> {
        let mut words = self.opts.wrapper.clone();
        // The slot script adds `-j<N>` to a cargo call; only the compiling subcommands accept it. For the others (update,
        // generate-lockfile, fmt) `env` in front keeps the slot (and its memory limit) but hides the call from that rule.
        if !matches!(
            args.first().copied(),
            Some("build" | "check" | "clippy" | "test")
        ) && !words.is_empty()
        {
            words.push("env".into());
        }
        words.push("cargo".into());
        words.extend(args.iter().map(|a| (*a).to_string()));
        let program = words.remove(0);
        let mut env = Vec::new();
        if let Some(t) = toolchain {
            env.push(("RUSTUP_TOOLCHAIN".to_string(), t.to_string()));
        } else if let Ok(v) = std::fs::read_to_string(self.opts.release_dir.join("RUST_VERSION")) {
            env.push(("RUSTUP_TOOLCHAIN".to_string(), v.trim().to_string()));
        }
        if !self.opts.online {
            env.push((
                "GIT_CONFIG_GLOBAL".into(),
                self.opts.git_config.display().to_string(),
            ));
            env.push(("CARGO_NET_GIT_FETCH_WITH_CLI".into(), "true".into()));
        }
        self.exec(&program, words, env)
    }

    /// Path of a script of the fs-release checkout.
    #[must_use]
    pub fn script(&self, name: &str) -> String {
        self.opts
            .release_dir
            .join("scripts")
            .join(name)
            .display()
            .to_string()
    }
}

#[cfg(test)]
mod tests {
    use super::LiftOpts;

    fn opts() -> LiftOpts {
        LiftOpts::new("/x".into(), "/y".into())
    }

    #[test]
    fn repo_arguments_fill_the_named_slots_and_wanted_keeps_the_order_of_libs() {
        let mut o = opts();
        for a in [
            "fs-types=v0.0.1",
            "fs-interfaces=v0.0.27",
            "fs-rpc=v0.0.2",
            "fs-components=v0.0.25",
        ] {
            o.set_lib_arg(a).unwrap();
        }
        assert_eq!(o.components.as_deref(), Some("v0.0.25"));
        assert_eq!(o.interfaces.as_deref(), Some("v0.0.27"));
        let names: Vec<_> = o
            .wanted()
            .into_iter()
            .map(|(n, t)| format!("{n}={t}"))
            .collect();
        assert_eq!(
            names,
            [
                "fs-components=v0.0.25",
                "fs-interfaces=v0.0.27",
                "fs-types=v0.0.1",
                "fs-rpc=v0.0.2"
            ]
        );
    }

    #[test]
    fn unknown_names_duplicates_and_bad_arguments_are_errors() {
        let mut o = opts();
        assert!(o
            .set_lib_arg("fs-wiki=v0.0.1")
            .unwrap_err()
            .contains("unknown library"));
        assert!(o
            .set_lib_arg("fs-types")
            .unwrap_err()
            .contains("NAME=vX.Y.Z"));
        o.set_lib_arg("fs-types=v0.0.1").unwrap();
        assert!(o
            .set_lib_arg("fs-types=v0.0.2")
            .unwrap_err()
            .contains("twice"));
        o.interfaces = Some("v0.0.1".into());
        assert!(o
            .set_lib_arg("fs-interfaces=v0.0.2")
            .unwrap_err()
            .contains("twice"));
    }
}
