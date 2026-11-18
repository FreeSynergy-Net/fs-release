#!/usr/bin/env bash
# The topmost section of CHANGELOG.md is `## Unreleased...` or exactly `## <version of crates/fs-release/Cargo.toml>` (no date in the heading: the
# tag comparison of test_changelog_released.sh needs the plain `## X.Y.Z`), and it is not empty. Counter-checks on copies: a date in the heading,
# another version, an empty section, a missing section.
#   bash scripts/test_changelog_top.sh
set -uo pipefail
here="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
repo="$(cd "$here/.." && pwd)"
tmp="$(mktemp -d)"
trap 'rm -rf "$tmp"' EXIT
fails=0
ok() { echo "ok   $1"; }
bad() { echo "FAIL $1"; fails=$((fails + 1)); }
ver="$(sed -nE 's/^version = "([^"]+)"$/\1/p' "$repo/crates/fs-release/Cargo.toml" | head -1)"
[ -n "$ver" ] || { echo "FAIL no crate version found"; exit 1; }

# check <changelog>: 0 = the first `## ` heading is `## Unreleased...` or `## <ver>` and its section has text
check() {
  local h
  h="$(grep -m1 '^## ' "$1")" || { echo "no section"; return 1; }
  if [ "$h" != "## $ver" ] && [[ "$h" != "## Unreleased"* ]]; then echo "topmost heading '$h' is neither '## Unreleased...' nor '## $ver'"; return 1; fi
  if [ -z "$(awk -v h="$h" '$0==h{on=1;next} /^## /{on=0} on && NF' "$1")" ]; then echo "the topmost section is empty"; return 1; fi
  return 0
}
if msg="$(check "$repo/CHANGELOG.md")"; then ok "the topmost CHANGELOG section is Unreleased or $ver and not empty"; else bad "$msg"; fi
mut() { # <name> <sed expression>
  sed -E "$2" "$repo/CHANGELOG.md" >"$tmp/$1.md"
  if cmp -s "$tmp/$1.md" "$repo/CHANGELOG.md"; then bad "mutant '$1' changed nothing (the real top heading is not '## $ver'?)"; return; fi
  if check "$tmp/$1.md" >/dev/null; then bad "mutant '$1' stays green"; else ok "mutant '$1' is red"; fi
}
if [[ "$(grep -m1 '^## ' "$repo/CHANGELOG.md")" == "## $ver" ]]; then
  mut date-in-heading "0,/^## ${ver//./\\.}\$/s//## $ver (2026-11-18)/"
  mut other-version "0,/^## ${ver//./\\.}\$/s//## 9.9.9/"
  awk -v h="## $ver" '$0==h{print;on=1;next} /^## /{on=0} !on' "$repo/CHANGELOG.md" >"$tmp/empty.md"
  if check "$tmp/empty.md" >/dev/null; then bad "mutant 'empty-section' stays green"; else ok "mutant 'empty-section' is red"; fi
fi
printf '# Changelog\n\nno sections\n' >"$tmp/none.md"
if check "$tmp/none.md" >/dev/null; then bad "mutant 'no-section' stays green"; else ok "mutant 'no-section' is red"; fi
printf '# Changelog\n\n## Unreleased (next)\n\n- x\n\n## %s\n\n- y\n' "$ver" >"$tmp/unr.md"
if check "$tmp/unr.md" >/dev/null; then ok "'## Unreleased (...)' with text is fine"; else bad "Unreleased heading refused"; fi

if [ "$fails" -eq 0 ]; then echo "all ok"; else echo "$fails failed"; exit 1; fi
