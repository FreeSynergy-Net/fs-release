//! The language tag of a file name, in one canonical spelling.
//!
//! Pattern: none (a pure function). `de-at` and `de_AT` are the same language: language lower case, script title case, region upper case, as BCP 47
//! writes it. Whether `mf2`'s locale data knows the language is asked later by the library itself (`direction`).

/// The canonical spelling of `tag`, or why it is no language tag.
pub(crate) fn canonical(tag: &str) -> Result<String, String> {
    let tag = tag.replace('_', "-");
    let mut out = Vec::new();
    for (i, part) in tag.split('-').enumerate() {
        let letters = part.bytes().all(|b| b.is_ascii_alphabetic());
        let plain = part.bytes().all(|b| b.is_ascii_alphanumeric());
        let ok = if i == 0 {
            letters && (2..=3).contains(&part.len())
        } else {
            plain && (1..=8).contains(&part.len())
        };
        if !ok {
            return Err(format!("`{tag}` is no language tag"));
        }
        out.push(match (i, part.len()) {
            (0, _) => part.to_ascii_lowercase(),
            (_, 4) if letters => {
                let (head, rest) = part.split_at(1);
                format!("{}{}", head.to_ascii_uppercase(), rest.to_ascii_lowercase())
            }
            (_, 2) if letters => part.to_ascii_uppercase(),
            _ => part.to_ascii_lowercase(),
        });
    }
    Ok(out.join("-"))
}
