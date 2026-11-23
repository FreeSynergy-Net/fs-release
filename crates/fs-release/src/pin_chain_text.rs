//! Pattern: none (text helpers of `pin_chain`: the internal `uses:` lines of a file and the normalisation of the self-pins).
//! The `uses:` line is read by the one parser of the crate, `address::aufrufer::parse_uses_path` (recognises `fs-release/` in any spelling).

use fs_release_chain::owner::OWNERS;

use crate::address::aufrufer::parse_uses_path;

/// One reached `uses:` of the own repo: the path inside the repo and the 40-hex ref (or whatever stands after the `@`).
#[derive(Debug, PartialEq, Eq)]
pub(crate) struct Use {
    pub(crate) path: String,
    pub(crate) pin: String,
}

pub(crate) fn is_sha(s: &str) -> bool {
    s.len() == 40
        && s.bytes()
            .all(|b| b.is_ascii_hexdigit() && !b.is_ascii_uppercase())
}

/// The `uses:` of a line if it is a TRACKED one: owner exactly one of `OWNERS`, repo name exactly `fs-release`, with an `@<ref>`.
fn tracked(line: &str) -> Option<Use> {
    let (owner, name, path, pin) = parse_uses_path(line)?;
    (OWNERS.contains(&owner.as_str()) && name == "fs-release" && line.contains('@'))
        .then_some(Use { path, pin })
}

/// Every internal `uses: <owner>/fs-release/<path>@<ref>` of a text, comment lines excluded.
pub(crate) fn uses_of(text: &str) -> Vec<Use> {
    text.lines().filter_map(tracked).collect()
}

/// A non-comment line that mentions `uses` and `fs-release/` (any case) in a form the parser does not read (`- {uses: ...}`, `uses :`, quoted key):
/// it would stay old and unseen, so it is red (fail closed). A `run:` script text that merely mentions both is no such form.
fn unread_form(line: &str) -> bool {
    let code = line.trim_start();
    let code = code.strip_prefix("- ").map_or(code, str::trim_start);
    let low = code.to_ascii_lowercase();
    !low.starts_with('#')
        && low.contains("fs-release/")
        && (low.starts_with('{')
            || low.starts_with('[')
            || low.starts_with("uses")
            || low.starts_with("\"uses\"")
            || low.starts_with("'uses'"))
}

/// The `uses:` targets that name a `.../fs-release/...` which the chain does not follow: another owner or owner spelling, another spelling of the
/// repo name (GitHub resolves owner and repo without regard to case), no `@<ref>`, or a form the parser does not read.
pub(crate) fn foreign_uses(text: &str) -> Vec<String> {
    text.lines()
        .filter(|l| tracked(l).is_none() && (parse_uses_path(l).is_some() || unread_form(l)))
        .map(|l| l.trim().to_string())
        .collect()
}

/// The text with the 40-hex ref of every TRACKED `uses: <owner>/fs-release/<path>@<sha>` replaced by `PIN`: the self-pins that are followed
/// are the one allowed difference. Any other text (comments, scripts, foreign owners) stays as it is, so no difference can vanish unfollowed.
pub(crate) fn normalize(text: &str) -> String {
    let mut out = String::with_capacity(text.len());
    for line in text.split_inclusive('\n') {
        match tracked_line_pin(line) {
            Some((from, to)) => {
                out.push_str(&line[..from]);
                out.push_str("PIN");
                out.push_str(&line[to..]);
            }
            None => out.push_str(line),
        }
    }
    out
}

/// Byte range of the 40-hex ref of a tracked internal `uses:` line.
fn tracked_line_pin(line: &str) -> Option<(usize, usize)> {
    tracked(line)?;
    let from = line.to_ascii_lowercase().find("fs-release/")?;
    let at = from + line[from..].find('@')? + 1;
    let r = &line[at..];
    (r.len() >= 40 && is_sha(&r[..40])).then_some((at, at + 40))
}

#[cfg(test)]
mod tests {
    use super::{foreign_uses, normalize, uses_of};

    #[test]
    fn only_self_pins_are_normalised() {
        let a = "uses: FreeSynergy-Net/fs-release/actions/x@0123456789012345678901234567890123456789\necho fs-release/x@main";
        assert_eq!(
            normalize(a),
            "uses: FreeSynergy-Net/fs-release/actions/x@PIN\necho fs-release/x@main"
        );
    }

    #[test]
    fn foreign_owner_spellings_are_found_and_normalize_leaves_them_alone() {
        let sha = "0123456789012345678901234567890123456789";
        let t = format!("- uses: freesynergy-net/fs-release/actions/x@{sha}\n- uses: Evil/fs-release/a@{sha}\n- uses: FreeSynergy-Net/fs-release/a@{sha}\n- uses: actions/checkout@v4\n# uses: evil/fs-release/a@b\n");
        assert_eq!(foreign_uses(&t).len(), 2);
        assert_eq!(
            foreign_uses("  run: grep -E 'uses: x/fs-release/' actions"),
            Vec::<String>::new()
        );
        let n = normalize(&t);
        assert!(
            n.contains(&format!("freesynergy-net/fs-release/actions/x@{sha}")),
            "{n}"
        );
        assert!(n.contains("FreeSynergy-Net/fs-release/a@PIN"), "{n}");
    }

    #[test]
    fn uses_skip_comments_and_foreign_owners() {
        let t = "# uses: FreeSynergy-Net/fs-release/a@b\n- uses: Evil/fs-release/a@b\n- uses: FreeSynergy/fs-release/.github/workflows/c.yml@v1 # t";
        let u = uses_of(t);
        assert_eq!(u.len(), 1);
        assert_eq!(
            (u[0].path.as_str(), u[0].pin.as_str()),
            (".github/workflows/c.yml", "v1")
        );
    }
}
