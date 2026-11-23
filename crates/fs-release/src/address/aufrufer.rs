//! Pattern: Strategy (one rule set, switchable per rule so that tests can switch a rule off and see the suite turn red).
//!
//! `fs-release address aufrufer`: every `uses: <owner>/fs-release/...@<ref>` line of the callers must name the owner exactly,
//! pin a full SHA or an existing tag, and the ref must contain owner-neutral and lie on `origin/main` of the fs-release clone.

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

use super::scan::{git, repo_name};

/// The only valid owner of the callers.
pub const OWNER: &str = "FreeSynergy-Net";

/// Switches for the rules (all on in the tool; tests switch one off as a mutant).
#[derive(Clone, Copy)]
pub struct Rules {
    pub contains_neutral: bool,
    pub on_main: bool,
    pub case_insensitive_owner: bool,
}

impl Rules {
    #[must_use]
    pub const fn strict() -> Self {
        Self {
            contains_neutral: true,
            on_main: true,
            case_insensitive_owner: false,
        }
    }
}

/// The fs-release clone and the owner-neutral commit.
pub struct Ctx {
    clone: PathBuf,
    neutral: String,
    rules: Rules,
}

/// One checked line.
pub struct Line {
    pub repo: String,
    pub path: String,
    pub line: u32,
    pub red: Vec<(u8, String)>,
    pub ref12: String,
}

/// Result of one run.
pub struct Report {
    pub lines: Vec<Line>,
    pub quiet_repos: Vec<String>,
}

impl Report {
    #[must_use]
    pub fn red(&self) -> usize {
        self.lines.iter().filter(|l| !l.red.is_empty()).count()
    }

    /// The report as text (every line, never shortened) and the verdict.
    #[must_use]
    pub fn text(&self, repos: usize) -> String {
        let mut out: Vec<String> = self
            .quiet_repos
            .iter()
            .map(|r| format!("{r}: keine fs-release-Aufrufe"))
            .collect();
        for l in &self.lines {
            let head = format!("{}:{}:{}:", l.repo, l.path, l.line);
            out.push(if l.red.is_empty() {
                format!("{head} OK {}", l.ref12)
            } else {
                let nrs: Vec<String> = l.red.iter().map(|(n, _)| n.to_string()).collect();
                let why: Vec<&str> = l.red.iter().map(|(_, w)| w.as_str()).collect();
                format!("{head} ROT {} {}", nrs.join(","), why.join("; "))
            });
        }
        let (n, k) = (self.lines.len(), self.red());
        out.push(if k == 0 {
            format!("GREEN aufrufer: {n} Zeilen in {repos} Repos, 0 rot")
        } else {
            format!("RED aufrufer: {k} von {n} Zeilen rot")
        });
        out.join("\n") + "\n"
    }
}

impl Ctx {
    /// # Errors
    /// `neutral` is no 40-hex commit of the clone, or the clone is no git repo or has no `origin/main`.
    pub fn new(clone: &Path, neutral: &str, rules: Rules) -> Result<Self, String> {
        if !is_sha(neutral) {
            return Err(format!("--neutral ist keine 40-hex-SHA: {neutral}"));
        }
        let c = Self {
            clone: clone.to_path_buf(),
            neutral: neutral.to_ascii_lowercase(),
            rules,
        };
        if c.commit("refs/remotes/origin/main").is_none() {
            return Err(format!(
                "{}: kein Git-Klon mit origin/main",
                clone.display()
            ));
        }
        if c.commit(neutral).is_none() {
            return Err(format!("--neutral {neutral} nicht im Klon"));
        }
        Ok(c)
    }

    fn commit(&self, name: &str) -> Option<String> {
        let spec = format!("{name}^{{commit}}");
        let o = git(&self.clone, &["rev-parse", "--verify", "--quiet", &spec]).ok()?;
        o.status
            .success()
            .then(|| String::from_utf8_lossy(&o.stdout).trim().to_string())
    }

    fn ancestor(&self, a: &str, b: &str) -> bool {
        git(&self.clone, &["merge-base", "--is-ancestor", a, b]).is_ok_and(|o| o.status.success())
    }

    /// Rules 2 to 5 for one ref: the red reasons and the resolved commit.
    fn judge_ref(&self, r: &str) -> (Vec<(u8, String)>, Option<String>) {
        let (name, rule2) = if is_sha(r) {
            (r.to_string(), None)
        } else if is_tag(r) {
            (format!("refs/tags/{r}"), None)
        } else {
            (
                String::new(),
                Some(format!("Ref `{r}` ist weder 40-hex noch Tag vX.Y.Z")),
            )
        };
        if let Some(w) = rule2 {
            return (vec![(2, w)], None);
        }
        let Some(c) = self.commit(&name) else {
            let rule = if is_sha(r) { 3 } else { 2 };
            return (vec![(rule, format!("Ref {r} im Klon unbekannt"))], None);
        };
        let mut red = Vec::new();
        if self.rules.contains_neutral && !self.ancestor(&self.neutral, &c) {
            red.push((4, "Ref enthaelt owner-neutral nicht".to_string()));
        }
        if self.rules.on_main && !self.ancestor(&c, "refs/remotes/origin/main") {
            red.push((5, "Ref nicht auf origin/main (Zweig oder Fork)".to_string()));
        }
        (red, Some(c))
    }

    /// All `uses:` lines of fs-release in the caller repo `dir` at `git_ref`.
    ///
    /// # Errors
    /// Repo or ref not readable.
    pub fn check_repo(&self, dir: &Path, git_ref: &str, rep: &mut Report) -> Result<(), String> {
        let repo = repo_name(dir)?;
        let spec = format!("{git_ref}^{{commit}}");
        let ok = git(dir, &["rev-parse", "--verify", "--quiet", &spec])?;
        if !ok.status.success() {
            return Err(format!("{repo}: FEHLT ref {git_ref}"));
        }
        // The dir must be the root of the repo: a subfolder would list only its subtree (silent green).
        let top = git(dir, &["rev-parse", "--show-toplevel"])?;
        let top = PathBuf::from(String::from_utf8_lossy(&top.stdout).trim());
        if !matches!((top.canonicalize(), dir.canonicalize()), (Ok(a), Ok(b)) if a == b) {
            return Err(format!(
                "{repo}: {} ist nicht die Wurzel des Repos",
                dir.display()
            ));
        }
        // `-z`: no quoting of non-ASCII names; `--full-tree`: paths from the root, whatever the cwd.
        let ls = git(
            dir,
            &["ls-tree", "-r", "-z", "--full-tree", "--name-only", git_ref],
        )?;
        if !ls.status.success() {
            return Err(format!("{repo}: git ls-tree fehlgeschlagen"));
        }
        let mut found: Vec<(Line, String)> = Vec::new();
        for path in String::from_utf8_lossy(&ls.stdout)
            .split('\0')
            .filter(|p| is_caller_file(p))
        {
            let show = git(dir, &["show", &format!("{git_ref}:{path}")])?;
            if !show.status.success() {
                return Err(format!("{repo}: {path} nicht lesbar"));
            }
            for (i, text) in String::from_utf8_lossy(&show.stdout).lines().enumerate() {
                let Some((owner, name, r)) = parse_uses(text) else {
                    continue;
                };
                let mut red = Vec::new();
                let same = if self.rules.case_insensitive_owner {
                    owner.eq_ignore_ascii_case(OWNER)
                } else {
                    owner == OWNER
                };
                if !same {
                    red.push((1, format!("Owner `{owner}` statt {OWNER}")));
                }
                if name != "fs-release" {
                    red.push((
                        1,
                        format!("Repo-Name `{name}` statt fs-release (Look-alike)"),
                    ));
                }
                let (more, commit) = self.judge_ref(&r);
                red.extend(more);
                let key = commit.unwrap_or_else(|| r.clone());
                let line = Line {
                    repo: repo.clone(),
                    path: path.to_string(),
                    line: u32::try_from(i + 1).unwrap_or(0),
                    red,
                    ref12: r.chars().take(12).collect(),
                };
                found.push((line, key));
            }
        }
        if found.is_empty() {
            rep.quiet_repos.push(repo);
            return Ok(());
        }
        let mut keys: BTreeMap<&str, usize> = BTreeMap::new();
        for (_, k) in &found {
            *keys.entry(k.as_str()).or_default() += 1;
        }
        let mixed = keys.len() > 1;
        for (mut l, _) in found {
            if mixed {
                l.red
                    .push((6, "gemischt: mehrere Refs im Repo".to_string()));
            }
            rep.lines.push(l);
        }
        Ok(())
    }
}

fn is_sha(s: &str) -> bool {
    s.len() == 40 && s.chars().all(|c| c.is_ascii_hexdigit())
}

fn is_tag(s: &str) -> bool {
    let Some(v) = s.strip_prefix('v') else {
        return false;
    };
    let p: Vec<&str> = v.split('.').collect();
    p.len() == 3
        && p.iter()
            .all(|x| !x.is_empty() && x.chars().all(|c| c.is_ascii_digit()))
}

/// Workflow files and composite actions (the places where callers use fs-release).
fn is_caller_file(p: &str) -> bool {
    let yaml = |n: &str| {
        Path::new(n)
            .extension()
            .is_some_and(|e| e == "yml" || e == "yaml")
    };
    if let Some(rest) = p.strip_prefix(".github/workflows/") {
        return !rest.contains('/') && yaml(rest);
    }
    let action = matches!(p.rsplit('/').next(), Some("action.yml" | "action.yaml"));
    action && (p.starts_with("actions/") || p.starts_with(".github/actions/"))
}

/// `uses: <owner>/fs-release/...@<ref>` without the comment part: the owner (maximal `[A-Za-z0-9-]` run left of
/// `/fs-release/`, left-anchored) and the ref.
#[must_use]
pub fn parse_uses(line: &str) -> Option<(String, String, String)> {
    let code = line.split_once(" #").map_or(line, |(c, _)| c);
    let target = code
        .trim_start()
        .trim_start_matches("- ")
        .trim_start()
        .strip_prefix("uses:")?
        .trim()
        .trim_matches(|c| c == '"' || c == '\'');
    if target.starts_with("./") {
        return None;
    }
    let at = target.to_ascii_lowercase().find("fs-release/")?; // recognised case-insensitively (GitHub resolves owner/repo so), judged exactly by the caller
    let name = target[at..at + "fs-release".len()].to_string();
    let before = &target[..at];
    let head = before.strip_suffix('/').unwrap_or(before);
    if head.len() == before.len() && !before.is_empty() {
        return None; // `xfs-release/`: a different name
    }
    let start = head
        .rfind(|c: char| !(c.is_ascii_alphanumeric() || c == '-'))
        .map_or(0, |i| i + 1);
    let owner = head[start..].to_string();
    let r = target.split_once('@').map_or("", |(_, r)| r).trim();
    Some((owner, name, r.to_string()))
}
