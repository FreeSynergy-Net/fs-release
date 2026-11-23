//! Pattern: Strategy (one rule per kind of install line, all answering "is this tool pinned to an exact version?").
//!
//! The release chain signs what the build job produced, so a tool installed in CI must not float: every `cargo install`, `cargo binstall` and
//! `taiki-e/install-action` line in `.github/` and `actions/` names an exact version (`name@x.y.z`) and uses `--locked`; a `cargo install --git` needs
//! a `--rev` of a full commit. `fs-release check-ci-tools [dir]` runs it over a checkout (rc 1 = a line to fix).

use std::fs;
use std::path::Path;

/// `name@MAJOR.MINOR.PATCH` with a crate name of `[a-z0-9_-]`.
fn is_pinned(tool: &str) -> bool {
    let Some((name, ver)) = tool.split_once('@') else {
        return false;
    };
    let parts: Vec<&str> = ver.split('.').collect();
    !name.is_empty()
        && name
            .chars()
            .all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == '-' || c == '_')
        && parts.len() == 3
        && parts
            .iter()
            .all(|p| !p.is_empty() && p.chars().all(|c| c.is_ascii_digit()))
}

fn is_sha(s: &str) -> bool {
    let s = s.trim_matches(|c| c == '"' || c == '\'');
    s.len() == 40 && s.chars().all(|c| c.is_ascii_hexdigit()) || s.starts_with('$')
}

/// The value of a YAML line `key: value` (`TOOLS: ...`, `tool: ...`, `tools: ...`), not an input definition (`{ description ... }`).
fn yaml_value<'a>(line: &'a str, key: &str) -> Option<&'a str> {
    let rest = line.trim_start().strip_prefix(key)?.strip_prefix(':')?;
    let v = rest.trim();
    (!v.is_empty() && !v.starts_with('{') && !v.starts_with("${{")).then_some(v)
}

fn install_line(line: &str) -> Option<String> {
    let words: Vec<&str> = line.split_whitespace().collect();
    let at = words
        .windows(2)
        .position(|w| w[0] == "cargo" && (w[1] == "install" || w[1] == "binstall"))?;
    let (kind, args) = (words[at + 1], &words[at + 2..]);
    if !args.contains(&"--locked") {
        return Some(format!("`cargo {kind}` without --locked"));
    }
    if kind == "install" {
        let rev = args.iter().position(|a| *a == "--rev");
        if args.contains(&"--git") {
            return match rev.and_then(|i| args.get(i + 1)) {
                Some(r) if is_sha(r) => None,
                _ => Some("`cargo install --git` without `--rev <full commit>`".into()),
            };
        }
        let versioned = args.iter().any(|a| *a == "--version" || is_pinned(a));
        return (!versioned)
            .then(|| "`cargo install` without an exact version (name@x.y.z)".into());
    }
    // binstall: the tools come as arguments; `$TOOLS` is checked where it is set.
    let loose = args
        .iter()
        .filter(|a| !a.starts_with('-') && !a.starts_with('$'))
        .find(|a| !is_pinned(a));
    loose.map(|a| format!("`cargo binstall {a}` without an exact version (name@x.y.z)"))
}

/// The violations of one file as `file:line: reason`; comment lines (`#`) are not checked.
#[must_use]
pub fn violations(file: &str, text: &str) -> Vec<String> {
    let mut out = Vec::new();
    for (n, line) in text.lines().enumerate() {
        if line.trim_start().starts_with('#') {
            continue;
        }
        let mut why = install_line(line);
        for key in ["tool", "tools", "TOOLS"] {
            if let Some(v) = yaml_value(line, key) {
                let v = v.trim_matches(|c| c == '"' || c == '\'');
                let loose = v
                    .split([' ', ','])
                    .filter(|t| !t.is_empty())
                    .find(|t| !is_pinned(t));
                if let Some(t) = loose {
                    why = Some(format!(
                        "`{key}: {t}` without an exact version (name@x.y.z)"
                    ));
                }
            }
        }
        if let Some(w) = why {
            out.push(format!("{file}:{}: {w}", n + 1));
        }
    }
    out
}

fn walk(dir: &Path, out: &mut Vec<String>) {
    let Ok(rd) = fs::read_dir(dir) else { return };
    let mut paths: Vec<_> = rd.flatten().map(|e| e.path()).collect();
    paths.sort();
    for p in paths {
        if p.is_dir() {
            walk(&p, out);
        } else if matches!(
            p.extension().and_then(|e| e.to_str()),
            Some("yml" | "yaml" | "sh")
        ) {
            if let Ok(t) = fs::read_to_string(&p) {
                out.extend(violations(&p.display().to_string(), &t));
            }
        }
    }
}

/// All violations in `.github/` and `actions/` of a checkout.
#[must_use]
pub fn scan(root: &Path) -> Vec<String> {
    let mut out = Vec::new();
    for sub in [".github", "actions"] {
        walk(&root.join(sub), &mut out);
    }
    out
}
