//! Pattern: none (pure checks of the lift step `lock-sources`: fs-release pin and `strict` switch of a repo lifted to V1+).

/// First fs-release tag that carries the lock check (`scripts/check_lock_sources.py`).
const FIRST_TAG: (u64, u64, u64) = (0, 0, 10);

fn tag_of(check_yml: &str) -> Option<(u64, u64, u64)> {
    let at = check_yml.find("fs-release/.github/workflows/check.yml@")?
        + "fs-release/.github/workflows/check.yml@".len();
    let word: String = check_yml[at..]
        .chars()
        .take_while(|c| !c.is_whitespace())
        .collect();
    let mut n = word.strip_prefix('v')?.split('.').map(str::parse::<u64>);
    Some((n.next()?.ok()?, n.next()?.ok()?, n.next()?.ok()?))
}

/// What is wrong in a repo that is lifted to V1+ (the step is red then): the fs-release pin of its `check.yml` is a tag older than
/// v0.0.10, or `.fs-lock-sources.toml` does not say `strict = true`. A pin that is `@main` or a commit is not a tag and cannot be compared:
/// it is not a problem here.
#[must_use]
pub fn problem(check_yml: Option<&str>, own_toml: Option<&str>) -> Option<String> {
    if let Some(t) = check_yml.and_then(tag_of) {
        if t < FIRST_TAG {
            return Some(format!(
                "check.yml pins fs-release v{}.{}.{}: raise it to v0.0.10 or newer (the lock check exists from there)",
                t.0, t.1, t.2
            ));
        }
    }
    let strict = own_toml.is_some_and(|t| {
        t.lines().any(|l| {
            l.split('#')
                .next()
                .is_some_and(|x| x.split_whitespace().collect::<String>() == "strict=true")
        })
    });
    (!strict).then(|| "`.fs-lock-sources.toml` with `strict = true` is missing: set it when the repo is on V1+".to_string())
}

#[cfg(test)]
mod tests {
    use super::problem;

    const NEW: &str = "uses: FreeSynergy-Net/fs-release/.github/workflows/check.yml@v0.0.10\n";
    const OLD: &str = "uses: FreeSynergy-Net/fs-release/.github/workflows/check.yml@v0.0.9\n";

    #[test]
    fn green_with_a_new_tag_and_strict() {
        assert_eq!(problem(Some(NEW), Some("strict = true # on V1+\n")), None);
        assert_eq!(
            problem(
                Some("uses: FreeSynergy-Net/fs-release/.github/workflows/check.yml@main\n"),
                Some("strict=true")
            ),
            None
        );
    }

    #[test]
    fn red_for_an_old_pin_and_for_a_missing_strict() {
        assert!(problem(Some(OLD), Some("strict = true"))
            .unwrap()
            .contains("v0.0.9"));
        assert!(problem(Some(NEW), None).unwrap().contains("strict = true"));
        assert!(problem(Some(NEW), Some("strict = false\n")).is_some());
        assert!(problem(Some(NEW), Some("# strict = true\n")).is_some());
    }
}
