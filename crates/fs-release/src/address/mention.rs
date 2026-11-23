//! Pattern: Strategy (one scanner decides what counts as an owner mention; stock, after-check and replace all ask it).
//!
//! A mention is the owner name inside a GitHub address or an owner value. It is found BROADLY (every `Owner/<name>` and every
//! bare owner value with a host or key before it); the replace step only touches the [`Kind::Safe`] ones.
//!
//! Choice against false hits (XDG paths `~/.config/freesynergy/tasks`, `/srv/FreeSynergy`, npm `@freesynergy/ui`): a mention
//! right after `/` counts only behind a known host (`github.com/`, `repos/`, ...); a lower-case `freesynergy/` without host or key
//! does not count. Measured on the stock files this gives fewer false hits than an `allow` entry per XDG path (the `allow`
//! list stays for real history texts), and it still finds `"FreeSynergy/fs-x"` strings in TOML, YAML, Rust and Python.

/// What the text before the owner says.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Kind {
    /// Behind a host (`github.com/`, `repos/`) or a key (`uses:`, `ORG=`, `repo =`, `-R`): the replace step may change it.
    Safe,
    /// An owner mention without such a prefix (`"FreeSynergy/fs-x"` in a list, prose): counted, never replaced.
    Other,
    /// `ghcr.io/<owner>/`: image names are lower case; reported on its own, never replaced, not part of the after-check.
    Ghcr,
}

/// One owner name in a line: byte range of the owner token and its kind.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Mention {
    pub start: usize,
    pub end: usize,
    pub kind: Kind,
}

const HOSTS: &[&str] = &[
    "github.com/",
    "github.com:",
    "github\\.com[/:]",
    "github\\.com/",
    "githubusercontent.com/",
    "repos/",
    "orgs/",
    "users/",
];
const GHCR: &str = "ghcr.io/";
const KEYS: &[&str] = &[
    "owner",
    "org",
    "organization",
    "repository_owner",
    "repo",
    "repository",
    "uses",
    "-r",
    "--repo",
    "--org",
    "--owner",
];

/// All mentions of `owner` (not `owner-...`) in `line`, left to right.
#[must_use]
pub fn mentions(line: &str, owner: &str) -> Vec<Mention> {
    let low = line.to_ascii_lowercase();
    let needle = owner.to_ascii_lowercase();
    let mut out = Vec::new();
    let mut from = 0;
    while let Some(off) = low[from..].find(&needle) {
        let start = from + off;
        let end = start + needle.len();
        from = end;
        if let Some(kind) = judge(line, &low, start, end, owner) {
            out.push(Mention { start, end, kind });
        }
    }
    out
}

fn judge(line: &str, low: &str, start: usize, end: usize, owner: &str) -> Option<Kind> {
    let before = &low[..start];
    let prev = before.chars().next_back();
    if prev.is_some_and(|c| c.is_ascii_alphanumeric() || matches!(c, '_' | '-' | '@')) {
        return None;
    }
    let host = HOSTS.iter().any(|h| before.ends_with(h));
    let ghcr = before.ends_with(GHCR);
    let key = key_before(before);
    let exact_case = &line[start..end] == owner;
    let after = &low[end..];
    let marked = host || ghcr || key;
    let slash_form = after.strip_prefix('/').is_some_and(|rest| {
        marked
            || rest.chars().next().is_some_and(|c| {
                c.is_ascii_alphanumeric() || matches!(c, '{' | '$' | '<' | '*' | '%')
            })
    });
    let bare_end = !slash_form && !bare_blocked(after);
    let kind = if ghcr {
        Kind::Ghcr
    } else if host || key {
        Kind::Safe
    } else {
        Kind::Other
    };
    if slash_form {
        if prev == Some('/') && !marked {
            return None;
        }
        if !exact_case && !marked {
            return None;
        }
        return Some(kind);
    }
    (bare_end && marked).then_some(kind)
}

/// The bare form ends at a boundary: not `owner-Net`, `owner.net`, `owner2`, and not a slash form.
fn bare_blocked(after: &str) -> bool {
    let mut c = after.chars();
    match c.next() {
        Some(x) if x.is_ascii_alphanumeric() || matches!(x, '_' | '-' | '/') => true,
        Some('.') => c.next().is_some_and(|y| y.is_ascii_alphanumeric()),
        _ => false,
    }
}

/// True when a key from [`KEYS`] stands before the owner (`uses: `, `ORG=`, `org = "`, `-R `, `--repo=`).
fn key_before(before: &str) -> bool {
    let quote = |c: char| matches!(c, ' ' | '"' | '\'');
    let s = before.trim_end_matches(quote);
    let s = s.strip_suffix(['=', ':']).unwrap_or(s);
    let s = s.trim_end_matches(quote);
    let word: &str = s
        .rsplit(|c: char| !(c.is_ascii_alphanumeric() || matches!(c, '_' | '-')))
        .next()
        .unwrap_or("");
    KEYS.contains(&word)
}

/// The line with every [`Kind::Safe`] mention swapped to `new`; `None` when nothing changed.
#[must_use]
pub fn swap_safe(line: &str, owner: &str, new: &str) -> Option<String> {
    let safe: Vec<Mention> = mentions(line, owner)
        .into_iter()
        .filter(|m| m.kind == Kind::Safe)
        .collect();
    if safe.is_empty() {
        return None;
    }
    let mut out = String::with_capacity(line.len() + new.len());
    let mut at = 0;
    for m in safe {
        out.push_str(&line[at..m.start]);
        out.push_str(new);
        at = m.end;
    }
    out.push_str(&line[at..]);
    Some(out)
}
