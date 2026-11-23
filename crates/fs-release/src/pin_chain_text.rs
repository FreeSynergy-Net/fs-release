//! Pattern: none (text helpers of `pin_chain`: the internal `uses:` lines of a file and the normalisation of the self-pins).

use fs_release_chain::owner::OWNERS;

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

/// (Only a real YAML `uses:` key at the start of the line, optionally after `- `: a `uses:` inside a script text is no pin.)
/// The target of a `uses:` line (comment lines and trailing comments excluded), if the line has one.
fn target_of(line: &str) -> Option<&str> {
    let code = line.trim_start();
    if code.starts_with('#') {
        return None;
    }
    let code = code.strip_prefix("- ").map_or(code, str::trim_start);
    let after = code.strip_prefix("uses:")?;
    Some(
        after
            .split(" #")
            .next()
            .unwrap_or("")
            .trim()
            .trim_matches(|c| c == '"' || c == '\''),
    )
}

/// The part of a target after `<tracked owner>/fs-release/`, if the owner is one of the exact `OWNERS`.
fn tracked_rest(target: &str) -> Option<&str> {
    OWNERS
        .iter()
        .find_map(|o| target.strip_prefix(&format!("{o}/fs-release/")))
}

/// Every internal `uses: <owner>/fs-release/<path>@<ref>` of a text, comment lines excluded.
pub(crate) fn uses_of(text: &str) -> Vec<Use> {
    text.lines()
        .filter_map(target_of)
        .filter_map(tracked_rest)
        .filter_map(|rest| {
            rest.split_once('@').map(|(path, pin)| Use {
                path: path.to_string(),
                pin: pin.trim().to_string(),
            })
        })
        .collect()
}

/// The `uses:` targets that name a `.../fs-release/...` of an owner that is not exactly one of the `OWNERS` (other spelling or other owner).
/// GitHub resolves the owner without regard to case, but the chain follows only the exact spellings: such a line would stay old and unseen.
pub(crate) fn foreign_uses(text: &str) -> Vec<String> {
    text.lines()
        .filter_map(target_of)
        .filter(|t| t.contains("/fs-release/") && tracked_rest(t).is_none())
        .map(str::to_string)
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
    let target = target_of(line)?;
    tracked_rest(target)?;
    let at = line.find(target)? + target.find('@')? + 1;
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
        assert!(foreign_uses("  run: grep -E 'uses: x/fs-release/' actions").is_empty());
        let n = normalize(&t);
        assert!(n.contains(&format!("freesynergy-net/fs-release/actions/x@{sha}")), "{n}");
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
