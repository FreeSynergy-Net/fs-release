//! Pattern: none (the one reader of a `uses: <owner>/fs-release/...@<ref>` line, shared by `aufrufer` and the pin-chain gate).

/// `uses: <owner>/fs-release/...@<ref>` without the comment part: the owner (maximal `[A-Za-z0-9-]` run left of
/// `/fs-release/`, left-anchored) and the ref.
#[must_use]
pub fn parse_uses(line: &str) -> Option<(String, String, String)> {
    parse_uses_path(line).map(|(o, n, _, r)| (o, n, r))
}

/// Like [`parse_uses`], plus the path (between `fs-release/` and `@`): `(owner, name, path, ref)`.
#[must_use]
pub fn parse_uses_path(line: &str) -> Option<(String, String, String, String)> {
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
    let rest = &target[at + "fs-release/".len()..];
    let (path, r) = rest.split_once('@').unwrap_or((rest, ""));
    Some((owner, name, path.to_string(), r.trim().to_string()))
}
