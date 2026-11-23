//! Pattern: Strategy (one rule per kind of install line, all answering "is this tool pinned to an exact version?").
//!
//! The release chain signs what the build job produced, so a tool installed in CI must not float: every `cargo install`, `cargo binstall` and
//! `taiki-e/install-action` line in `.github/` and `actions/` names an exact version (`name@x.y.z`) and uses `--locked`; a `cargo install --git` needs
//! a `--rev` of a full commit. Every `uses:` is a full commit SHA (a foreign action also names its tag in a comment); a moving tag or branch can be
//! moved under a signed release. `fs-release check-ci-tools [dir]` runs it over a checkout (rc 1 = a line to fix).

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

/// `x.y.z` (or `=x.y.z`).
fn is_exact(v: &str) -> bool {
    is_pinned(&format!("x@{}", v.trim_start_matches('=')))
}

fn is_sha(s: &str) -> bool {
    let s = s.trim_matches(|c| c == '"' || c == '\'');
    (s.len() == 40 && s.chars().all(|c| c.is_ascii_hexdigit()))
        || matches!(s, "$REV" | "${REV}" | "$rev" | "$GITHUB_SHA")
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
        .iter()
        .position(|w| *w == "cargo" || *w == "cargo-binstall")?;
    let direct = words[at] == "cargo-binstall";
    let rest: Vec<&str> = words[at + 1..]
        .iter()
        .copied()
        .filter(|w| !w.starts_with('+'))
        .collect();
    let (kind, args) = if direct {
        ("binstall", &rest[..])
    } else {
        match rest.first() {
            Some(&k) if k == "install" || k == "binstall" => (k, &rest[1..]),
            _ => return None,
        }
    };
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
        let versioned = args.iter().any(|a| is_pinned(a))
            || args
                .iter()
                .position(|a| *a == "--version")
                .and_then(|i| args.get(i + 1))
                .is_some_and(|v| is_exact(v));
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

/// A `uses:` line: local (`./...`) or `owner/repo[/path]@<40 hex>`; a foreign action also names its tag in a comment (`# v4.1.0`).
fn uses_line(line: &str) -> Option<String> {
    let (code, comment) = line.split_once(" #").map_or((line, ""), |(c, k)| (c, k));
    let target = code
        .trim_start()
        .trim_start_matches("- ")
        .strip_prefix("uses:")?
        .trim()
        .trim_matches(|c| c == '"' || c == '\'');
    if target.starts_with("./") || target.starts_with("docker://") && target.contains("@sha256:") {
        return None;
    }
    let Some((name, r)) = target.split_once('@') else {
        return Some(format!("`uses: {target}` has no @<commit>"));
    };
    if r.len() != 40 || !r.chars().all(|c| c.is_ascii_hexdigit()) {
        return Some(format!(
            "`uses: {name}@{r}` is not pinned to a full commit SHA"
        ));
    }
    let own = name.starts_with("FreeSynergy/fs-release/");
    (!own && comment.trim().is_empty())
        .then(|| format!("`uses: {name}@<sha>` names no tag in a comment (# vX.Y.Z)"))
}

/// The violations of one file as `file:line: reason`; comment lines (`#`) are not checked.
#[must_use]
pub fn violations(file: &str, text: &str) -> Vec<String> {
    let mut out = Vec::new();
    for (n, line) in text.lines().enumerate() {
        if line.trim_start().starts_with('#') {
            continue;
        }
        let mut why = install_line(line).or_else(|| uses_line(line));
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
