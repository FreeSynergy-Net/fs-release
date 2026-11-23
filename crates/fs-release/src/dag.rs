//! Pattern: Strategy (the graph of the pin cascade is READ from the `Cargo.toml` files of the repos; the workflow header is only checked against it
//! or generated from it, never completed by hand).
//!
//! `fs-release dag-check <workflow.yaml> --repos <dir>`: every `FreeSynergy` git dependency found in the `Cargo.toml` of the repos is an edge
//! `consumer <- owner repo` (owner = the repo named by the git URL, so `fs-conformance` is fs-types). The check is red when
//! an edge is missing in the `# DAG:` lines (or is surplus there), when `# GLIEDER-ORDER:` misses a Glied or is not a topological order.
//! `--emit` prints the two header blocks generated from the same graph (stable order: the current order first, then by name).

use std::collections::{BTreeMap, BTreeSet};
use std::fmt::Write as _;
use std::fs;
use std::path::Path;

/// consumer repo -> owner repos it depends on.
pub type Edges = BTreeMap<String, BTreeSet<String>>;

/// The graph read from the repos, and the git sources that name no repo of the directory (listed, never an edge).
#[derive(Debug, Default)]
pub struct Graph {
    pub edges: Edges,
    pub unknown: BTreeSet<String>,
    /// (consumer, owner, rev) of every `rev` pin.
    pub revs: BTreeSet<(String, String, String)>,
}

/// The repo name of a `github.com/FreeSynergy[-Net]/<repo>[.git]` URL.
fn owner_repo(url: &str) -> Option<String> {
    let rest = url.split("github.com/").nth(1)?;
    let (org, repo) = rest.split_once('/')?;
    if org != "FreeSynergy" && org != "FreeSynergy-Net" {
        return None;
    }
    let repo = repo.trim_end_matches('/').trim_end_matches(".git");
    (!repo.is_empty()).then(|| repo.to_string())
}

/// The first 8 characters of the `rev = "..."` of a line.
fn rev_of(l: &str) -> Option<String> {
    let r = l.split("rev = \"").nth(1)?;
    let r = r.split('"').next()?;
    Some(r.chars().take(8).collect())
}

/// The owner repos (and `rev` pins) of every `git = "<url>"` of one `Cargo.toml` text (`[patch.*]` sections are skipped: they are local heads, not pins).
#[must_use]
pub fn git_owners(text: &str) -> Vec<(String, Option<String>)> {
    let mut patch = false;
    let mut out = Vec::new();
    for line in text.lines() {
        let l = line.trim();
        if l.starts_with('[') {
            patch = l.starts_with("[patch");
        }
        if patch || l.starts_with('#') {
            continue;
        }
        let mut rest = l;
        while let Some(i) = rest.find("git = \"") {
            rest = &rest[i + 7..];
            if let Some(j) = rest.find('"') {
                if let Some(o) = owner_repo(&rest[..j]) {
                    out.push((o, rev_of(l)));
                }
                rest = &rest[j..];
            }
        }
    }
    out
}

fn cargo_files(dir: &Path, out: &mut Vec<std::path::PathBuf>) {
    let Ok(rd) = fs::read_dir(dir) else { return };
    let mut items: Vec<_> = rd.flatten().collect();
    items.sort_by_key(std::fs::DirEntry::file_name);
    for e in items {
        let p = e.path();
        let name = e.file_name();
        if p.is_dir() {
            if name != "target" && name != ".git" {
                cargo_files(&p, out);
            }
        } else if name == "Cargo.toml" {
            out.push(p);
        }
    }
}

/// Reads every `Cargo.toml` below `<repos>/<repo>/`. `foreign` repos (known foreign sources) are no edges; an owner without a directory is `unknown`.
///
/// # Errors
/// The directory cannot be read.
pub fn scan(repos: &Path, foreign: &BTreeSet<String>) -> Result<Graph, String> {
    let mut names = BTreeSet::new();
    let rd = fs::read_dir(repos).map_err(|e| format!("{}: {e}", repos.display()))?;
    for e in rd.flatten() {
        if e.path().is_dir() {
            names.insert(e.file_name().to_string_lossy().into_owned());
        }
    }
    let mut g = Graph::default();
    for repo in &names {
        let mut files = Vec::new();
        cargo_files(&repos.join(repo), &mut files);
        for f in files {
            let text = fs::read_to_string(&f).map_err(|e| format!("{}: {e}", f.display()))?;
            for (o, rev) in git_owners(&text) {
                if &o == repo || foreign.contains(&o) {
                    continue;
                }
                if names.contains(&o) {
                    if let Some(r) = rev {
                        g.revs.insert((repo.clone(), o.clone(), r));
                    }
                    g.edges.entry(repo.clone()).or_default().insert(o);
                } else {
                    g.unknown.insert(format!("{repo} <- {o}"));
                }
            }
        }
    }
    Ok(g)
}

/// The Glieder: every repo that depends on `root`, directly or over other repos.
#[must_use]
pub fn glieder(e: &Edges, root: &str) -> BTreeSet<String> {
    let mut set: BTreeSet<String> = BTreeSet::new();
    loop {
        let n = e
            .iter()
            .filter(|(k, d)| !set.contains(*k) && d.iter().any(|x| x == root || set.contains(x)))
            .map(|(k, _)| k.clone())
            .collect::<Vec<_>>();
        if n.is_empty() {
            return set;
        }
        set.extend(n);
    }
}

/// A topological order of the Glieder; ties go by position in `current` (unlisted after), then by name. `Err` = the repos of a cycle.
///
/// # Errors
/// The Glieder contain a cycle.
pub fn order(e: &Edges, root: &str, current: &[String]) -> Result<Vec<String>, Vec<String>> {
    let gl = glieder(e, root);
    let rank = |n: &String| {
        (
            current.iter().position(|x| x == n).unwrap_or(usize::MAX),
            n.clone(),
        )
    };
    let mut left: BTreeSet<String> = gl.clone();
    let mut out: Vec<String> = Vec::new();
    while !left.is_empty() {
        let next = left
            .iter()
            .filter(|n| {
                e.get(*n)
                    .is_none_or(|d| d.iter().all(|x| !left.contains(x) || x == *n))
            })
            .min_by_key(|n| rank(n))
            .cloned();
        let Some(n) = next else {
            return Err(left.into_iter().collect());
        };
        left.remove(&n);
        out.push(n);
    }
    Ok(out)
}

/// The header of the workflow as written: `# GLIEDER-ORDER:` and the `# DAG:` lines.
#[derive(Debug, Default)]
pub struct Header {
    pub order: Vec<String>,
    pub dag: Edges,
    pub revs: BTreeSet<(String, String, String)>,
}

fn split(s: &str) -> Vec<String> {
    s.split(',')
        .map(|x| x.trim().to_string())
        .filter(|x| !x.is_empty())
        .collect()
}

#[must_use]
pub fn parse_header(text: &str) -> Header {
    let mut h = Header::default();
    for l in text.lines() {
        if let Some(r) = l.strip_prefix("# GLIEDER-ORDER:") {
            h.order = split(r);
        } else if let Some(r) = l.strip_prefix("# REV-PIN: ") {
            if let Some((a, rest)) = r.split_once(" -> ") {
                let mut w = rest.split_whitespace();
                if let (Some(o), Some("rev"), Some(v)) = (w.next(), w.next(), w.next()) {
                    h.revs
                        .insert((a.trim().to_string(), o.to_string(), v.to_string()));
                }
            }
        } else if let Some(r) = l.strip_prefix("# DAG: ") {
            if let Some((a, b)) = r.split_once(" <- ") {
                h.dag
                    .entry(a.trim().to_string())
                    .or_default()
                    .extend(split(b));
            }
        }
    }
    h
}

/// The red findings (empty = green): missing or surplus edges, Glieder missing in or foreign to the order, an order that is not topological, cycles.
#[must_use]
pub fn check(g: &Graph, h: &Header, root: &str) -> Vec<String> {
    let e = &g.edges;
    let gl = glieder(e, root);
    let mut f = Vec::new();
    for g in &gl {
        let want = &e[g];
        let have = h.dag.get(g).cloned().unwrap_or_default();
        for d in want.difference(&have) {
            f.push(format!("edge missing in DAG: {g} <- {d} (Cargo.toml)"));
        }
        for d in have.difference(want) {
            f.push(format!(
                "edge surplus in DAG: {g} <- {d} (in no Cargo.toml)"
            ));
        }
    }
    for g in h.dag.keys().filter(|k| !gl.contains(*k)) {
        f.push(format!("DAG line of a non-Glied: {g}"));
    }
    for g in &gl {
        if !h.order.contains(g) {
            f.push(format!("Glied missing in GLIEDER-ORDER: {g}"));
        }
    }
    for g in h.order.iter().filter(|g| !gl.contains(*g)) {
        f.push(format!("GLIEDER-ORDER names a non-Glied: {g}"));
    }
    for (i, g) in h.order.iter().enumerate() {
        for d in e.get(g).into_iter().flatten() {
            if let Some(j) = h.order.iter().position(|x| x == d) {
                if j > i {
                    f.push(format!("order: {d} must stand before {g} ({g} <- {d})"));
                }
            }
        }
    }
    let want: BTreeSet<_> = g
        .revs
        .iter()
        .filter(|r| gl.contains(&r.0))
        .cloned()
        .collect();
    for r in want.difference(&h.revs) {
        f.push(format!("REV-PIN missing: {} -> {} rev {}", r.0, r.1, r.2));
    }
    for r in h.revs.difference(&want) {
        f.push(format!(
            "REV-PIN surplus or stale: {} -> {} rev {} (in no Cargo.toml)",
            r.0, r.1, r.2
        ));
    }
    if let Err(c) = order(e, root, &h.order) {
        f.push(format!("cycle among: {}", c.join(", ")));
    }
    f
}

/// The two header blocks generated from the graph.
///
/// # Errors
/// The Glieder contain a cycle.
pub fn emit(graph: &Graph, root: &str, current: &[String]) -> Result<String, Vec<String>> {
    let sorted = order(&graph.edges, root, current)?;
    let mut text = format!("# GLIEDER-ORDER: {}\n", sorted.join(", "));
    for glied in &sorted {
        let deps: Vec<&str> = graph.edges[glied].iter().map(String::as_str).collect();
        let _ = writeln!(text, "# DAG: {glied} <- {}", deps.join(", "));
    }
    for r in graph.revs.iter().filter(|r| sorted.contains(&r.0)) {
        let _ = writeln!(text, "# REV-PIN: {} -> {} rev {}", r.0, r.1, r.2);
    }
    Ok(text)
}

/// `OWNER/NAME` lines of the foreign-source list as repo names (comments after `#`).
#[must_use]
pub fn foreign_repos(text: &str) -> BTreeSet<String> {
    text.lines()
        .map(|l| l.split('#').next().unwrap_or("").trim())
        .filter_map(|l| l.split('/').nth(1))
        .map(str::to_string)
        .collect()
}
