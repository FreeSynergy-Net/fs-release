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

/// Every internal `uses: <owner>/fs-release/<path>@<ref>` of a text, comment lines excluded.
pub(crate) fn uses_of(text: &str) -> Vec<Use> {
    let mut out = Vec::new();
    for line in text.lines() {
        let code = line.trim_start();
        if code.starts_with('#') {
            continue;
        }
        let Some((_, after)) = code.split_once("uses:") else {
            continue;
        };
        let target = after
            .split(" #")
            .next()
            .unwrap_or("")
            .trim()
            .trim_matches(|c| c == '"' || c == '\'');
        for o in OWNERS {
            let Some(rest) = target.strip_prefix(&format!("{o}/fs-release/")) else {
                continue;
            };
            if let Some((path, pin)) = rest.split_once('@') {
                out.push(Use {
                    path: path.to_string(),
                    pin: pin.trim().to_string(),
                });
            }
        }
    }
    out
}

/// The text with the 40-hex ref of every `.../fs-release/<path>@<sha>` replaced by `PIN`: the self-pins are the one allowed difference.
pub(crate) fn normalize(text: &str) -> String {
    let mut out = String::with_capacity(text.len());
    let mut rest = text;
    while let Some(i) = rest.find("fs-release/") {
        let (head, tail) = rest.split_at(i + "fs-release/".len());
        out.push_str(head);
        let n = tail
            .find(|c: char| !(c.is_ascii_alphanumeric() || "_./-".contains(c)))
            .unwrap_or(tail.len());
        out.push_str(&tail[..n]);
        let after = &tail[n..];
        match after.strip_prefix('@') {
            Some(r) if r.len() >= 40 && is_sha(&r[..40]) => {
                out.push_str("@PIN");
                rest = &r[40..];
            }
            _ => rest = after,
        }
    }
    out.push_str(rest);
    out
}

#[cfg(test)]
mod tests {
    use super::{normalize, uses_of};

    #[test]
    fn only_self_pins_are_normalised() {
        let a = "uses: FreeSynergy-Net/fs-release/actions/x@0123456789012345678901234567890123456789\necho fs-release/x@main";
        assert_eq!(
            normalize(a),
            "uses: FreeSynergy-Net/fs-release/actions/x@PIN\necho fs-release/x@main"
        );
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
