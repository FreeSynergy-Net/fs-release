//! Pattern: Strategy (the four independent checks after the lock: fmt, clippy pedantic, coding rules, tests).

use fs_release_chain::outcome::Stop;
use fs_release_chain::sys::Out;

use super::LiftEnv;
use crate::task::{Task, SKIPPED};

/// The line a person (or an agent) should see first: a compiler error, a failed test, a panic, a format diff.
pub(super) fn red_line(o: &Out) -> String {
    let all = format!("{}\n{}", o.stderr, o.stdout);
    let lines = || all.lines().map(str::trim);
    // A failed test or a format diff says more than the closing `error: test failed`, so those come first.
    let hit = lines()
        .find(|l| l.contains("FAILED") || l.contains("panicked") || l.starts_with("Diff in"))
        .or_else(|| lines().find(|l| l.starts_with("error")));
    let line = hit.map_or_else(|| o.first_error(), str::to_string);
    line.chars().take(200).collect()
}

fn verdict(name: &str, o: &Out) -> Result<String, Stop> {
    if o.ok() {
        Ok(format!("{name} green"))
    } else {
        Err(Stop::red(red_line(o)))
    }
}

/// `cargo fmt --check`.
pub struct Fmt;

impl Task<LiftEnv<'_>> for Fmt {
    fn name(&self) -> &'static str {
        "fmt"
    }

    fn continue_on_red(&self) -> bool {
        true
    }

    fn run(&self, env: &LiftEnv) -> Result<String, Stop> {
        verdict("fmt", &env.cargo(&["fmt", "--check"])?)
    }
}

/// The same clippy call as `scripts/gates.sh`.
pub struct Clippy;

impl Task<LiftEnv<'_>> for Clippy {
    fn name(&self) -> &'static str {
        "clippy"
    }

    fn continue_on_red(&self) -> bool {
        true
    }

    fn run(&self, env: &LiftEnv) -> Result<String, Stop> {
        let args = [
            "clippy",
            "--workspace",
            "--all-targets",
            "--all-features",
            "--",
            "-D",
            "warnings",
            "-W",
            "clippy::pedantic",
        ];
        verdict("clippy pedantic", &env.cargo(&args)?)
    }
}

/// `scripts/check_rust_rules.py` (file length, module docs, pattern line, Blinc rules).
pub struct Rules;

impl Task<LiftEnv<'_>> for Rules {
    fn name(&self) -> &'static str {
        "rules"
    }

    fn continue_on_red(&self) -> bool {
        true
    }

    fn run(&self, env: &LiftEnv) -> Result<String, Stop> {
        let args = vec![
            "-I".to_string(),
            env.script("check_rust_rules.py"),
            env.opts.repo_dir.display().to_string(),
        ];
        verdict("rules", &env.exec("python3", args, vec![])?)
    }
}

/// `cargo test --workspace --all-features`.
pub struct Tests;

impl Task<LiftEnv<'_>> for Tests {
    fn name(&self) -> &'static str {
        "tests"
    }

    fn continue_on_red(&self) -> bool {
        true
    }

    fn run(&self, env: &LiftEnv) -> Result<String, Stop> {
        verdict(
            "tests",
            &env.cargo(&["test", "--workspace", "--all-features"])?,
        )
    }
}

/// `scripts/check_yaml.py`: every `.github/workflows/*.yml` parses (a step name with `: ` inside is not YAML; GitHub only
/// says "workflow file issue" after the push). Exit 2 of the script = no `PyYAML`: the step is skipped with a warning.
pub struct YamlCheck;

impl Task<LiftEnv<'_>> for YamlCheck {
    fn name(&self) -> &'static str {
        "yaml-check"
    }

    fn continue_on_red(&self) -> bool {
        true
    }

    fn run(&self, env: &LiftEnv) -> Result<String, Stop> {
        let args = vec![
            "-I".to_string(),
            env.script("check_yaml.py"),
            env.opts.repo_dir.display().to_string(),
        ];
        let o = env.exec("python3", args, vec![])?;
        match o.code {
            0 => Ok(o.stdout.trim().to_string()),
            2 => Ok(format!("{SKIPPED}{}", o.stdout.trim())),
            _ => Err(Stop::red(red_line(&o))),
        }
    }
}

/// `scripts/gates-lock.sh`: every shared library in exactly one source per Cargo.lock (warning first, red with `strict`).
pub struct LockSources;

impl Task<LiftEnv<'_>> for LockSources {
    fn name(&self) -> &'static str {
        "lock-sources"
    }

    fn continue_on_red(&self) -> bool {
        true
    }

    fn run(&self, env: &LiftEnv) -> Result<String, Stop> {
        let args = vec![
            env.script("gates-lock.sh"),
            env.opts.repo_dir.display().to_string(),
        ];
        // A lift that moves fs-types (V1+) also asks the lock check for "fs-types only from the repo FreeSynergy/fs-types".
        let only_from = env.opts.wanted().iter().any(|(n, _)| *n == "fs-types");
        let extra = if only_from {
            vec![("FS_LOCK_ONLY_FROM".to_string(), "1".to_string())]
        } else {
            vec![]
        };
        let ok = verdict("lock-sources", &env.exec("bash", args, extra)?)?;
        // From the lift run to V1+ (FS_LIFT_V1PLUS=1) the step is red until the repo has the new pin and `strict = true`.
        let v1plus = env
            .opts
            .env
            .iter()
            .any(|(k, v)| k == "FS_LIFT_V1PLUS" && v == "1")
            || std::env::var("FS_LIFT_V1PLUS").is_ok_and(|v| v == "1");
        if v1plus {
            let dir = &env.opts.repo_dir;
            let read = |p: &str| std::fs::read_to_string(dir.join(p)).ok();
            if let Some(p) = super::lockpin::problem(
                read(".github/workflows/check.yml").as_deref(),
                read(".fs-lock-sources.toml").as_deref(),
            ) {
                return Err(Stop::red(p));
            }
        }
        Ok(ok)
    }
}

/// The explicit CI-strict run: the current stable toolchain, the lock as it is (`--locked`), pedantic, all targets. The pinned
/// toolchain of `gates.sh` does not know the newest lints (`items_after_statements`, `assert!(is_empty)` in tests, ...); GitHub CI does.
pub struct ClippyStable;

impl Task<LiftEnv<'_>> for ClippyStable {
    fn name(&self) -> &'static str {
        "clippy-stable"
    }

    fn continue_on_red(&self) -> bool {
        true
    }

    fn run(&self, env: &LiftEnv) -> Result<String, Stop> {
        let args = [
            "clippy",
            "--workspace",
            "--all-targets",
            "--all-features",
            "--locked",
            "--",
            "-D",
            "warnings",
            "-W",
            "clippy::pedantic",
        ];
        verdict("clippy stable", &env.cargo_on(&args, Some("stable"))?)
    }
}
