#!/usr/bin/env bash
# Every `## X.Y.Z` section of CHANGELOG.md whose tag vX.Y.Z exists must equal the section of that tag (released history is never rewritten,
# a renamed heading turns this red). Counter-check: a changed section and a renamed heading are caught.
#   bash scripts/test_changelog_released.sh
set -uo pipefail
here="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
repo="$(cd "$here/.." && pwd)"
tmp="$(mktemp -d)"
trap 'rm -rf "$tmp"' EXIT
fails=0

# section <file>: the text of one section, from its heading up to (excluding) the next `## ` heading, trailing blank lines removed
section() { awk -v h="## $2" '$0==h{on=1;print;next} /^## /{on=0} on' "$1" | sed -e :a -e '/^\n*$/{$d;N;ba' -e '}'; }

# compare <changelog file> <tag-version>: 0 = equal
compare() {
  git -C "$repo" show "v$2:CHANGELOG.md" >"$tmp/tag.md" 2>/dev/null || return 0 # no such tag: nothing released
  diff <(section "$tmp/tag.md" "$2") <(section "$1" "$2") >/dev/null && [ -n "$(section "$1" "$2")" ]
}

checked=0
while read -r v; do
  if git -C "$repo" rev-parse -q --verify "refs/tags/v$v" >/dev/null; then
    checked=$((checked + 1))
    if compare "$repo/CHANGELOG.md" "$v"; then echo "ok   section $v equals the tag v$v"; else echo "FAIL section $v differs from the tag v$v"; fails=$((fails + 1)); fi
  fi
done < <(sed -n 's/^## \([0-9][0-9.]*\)$/\1/p' "$repo/CHANGELOG.md")

# counter-checks on a copy, against any tag that exists
first="$(git -C "$repo" tag -l 'v[0-9]*' | sort -V | tail -1)"
if [ -n "$first" ]; then
  v="${first#v}"
  sed "s/^## $v\$/## $v-x/" "$repo/CHANGELOG.md" >"$tmp/renamed.md"
  if compare "$tmp/renamed.md" "$v"; then echo "FAIL a renamed heading was not caught"; fails=$((fails + 1)); else echo "ok   red: renamed heading"; fi
  sed "/^## $v\$/,/^## /s/^- All crates.*/- changed/" "$repo/CHANGELOG.md" >"$tmp/changed.md"
  if compare "$tmp/changed.md" "$v"; then echo "FAIL a changed entry was not caught"; fails=$((fails + 1)); else echo "ok   red: changed entry"; fi
  if [ "$checked" -eq 0 ]; then echo "FAIL no released section found in CHANGELOG.md"; fails=$((fails + 1)); fi
fi
if [ "$fails" -eq 0 ]; then echo "all ok"; else echo "$fails failed"; exit 1; fi
