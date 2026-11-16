#!/usr/bin/env bash
# Released history is never rewritten: for EVERY tag vX.Y.Z (`git tag -l 'v[0-9]*'`) the section `## X.Y.Z` of CHANGELOG.md must exist and equal
# the section of that tag (a renamed heading, a deleted or a changed section turns this red). Without tags or in a shallow clone the test is
# NOT green: it prints "skipped" and exits 2 (the gate cannot be checked there; CI checks out with fetch-depth 0).
# Counter-checks run the same loop over a renamed heading, a deleted section and a changed entry.
#   bash scripts/test_changelog_released.sh
set -uo pipefail
here="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
repo="$(cd "$here/.." && pwd)"
tmp="$(mktemp -d)"
trap 'rm -rf "$tmp"' EXIT
fails=0

tags="$(git -C "$repo" tag -l 'v[0-9]*' | sort -V)"
if [ -z "$tags" ]; then echo "skipped: no tag v* in this clone, the released sections cannot be checked (not green; fetch the tags)"; exit 2; fi
if [ "$(git -C "$repo" rev-parse --is-shallow-repository)" != "false" ]; then echo "skipped: shallow clone, the tags cannot be trusted (not green; fetch-depth 0)"; exit 2; fi

# section <file> <version>: the text of one section, from its heading up to (excluding) the next `## ` heading, trailing blank lines removed
section() { awk -v h="## $2" '$0==h{on=1;print;next} /^## /{on=0} on' "$1" | sed -e :a -e '/^\n*$/{$d;N;ba' -e '}'; }

# loop <changelog file> <label>: runs over ALL tags, prints one line per tag; the number of red tags in $red
red=0
loop() {
  red=0
  local t v
  for t in $tags; do
    v="${t#v}"
    if [ -z "$(section "$1" "$v")" ]; then echo "$([ "$2" = real ] && echo FAIL || echo red )  [$2] section $v is missing in the CHANGELOG (tag $t)"; red=$((red + 1)); continue; fi
    if ! git -C "$repo" show "$t:CHANGELOG.md" >"$tmp/tag.md" 2>/dev/null || [ -z "$(section "$tmp/tag.md" "$v")" ]; then
      echo "ok   [$2] section $v present (the tag $t has no section of its own to compare)"; continue
    fi
    if diff <(section "$tmp/tag.md" "$v") <(section "$1" "$v") >/dev/null; then echo "ok   [$2] section $v equals the tag $t"; else echo "$([ "$2" = real ] && echo FAIL || echo red )  [$2] section $v differs from the tag $t"; red=$((red + 1)); fi
  done
}

loop "$repo/CHANGELOG.md" real
fails=$((fails + red))

# counter-checks through the real loop, against the newest tag
v="$(echo "$tags" | tail -1)"
v="${v#v}"
sed "s/^## $v\$/## $v (old)/" "$repo/CHANGELOG.md" >"$tmp/renamed.md"
loop "$tmp/renamed.md" renamed
if [ "$red" -ge 1 ]; then echo "ok   red: renamed heading"; else echo "FAIL a renamed heading was not caught"; fails=$((fails + 1)); fi
awk -v h="## $v" '$0==h{skip=1;next} /^## /{skip=0} !skip' "$repo/CHANGELOG.md" >"$tmp/deleted.md"
loop "$tmp/deleted.md" deleted
if [ "$red" -ge 1 ]; then echo "ok   red: deleted section"; else echo "FAIL a deleted section was not caught"; fails=$((fails + 1)); fi
sed "/^## $v\$/,/^## /s/^- All crates.*/- changed/" "$repo/CHANGELOG.md" >"$tmp/changed.md"
if cmp -s "$tmp/changed.md" "$repo/CHANGELOG.md"; then sed "/^## $v\$/{n;n;s/^\(.\)/changed \1/}" "$repo/CHANGELOG.md" >"$tmp/changed.md"; fi
loop "$tmp/changed.md" changed
if [ "$red" -ge 1 ]; then echo "ok   red: changed entry"; else echo "FAIL a changed entry was not caught"; fails=$((fails + 1)); fi
# an old tag's section deleted (not only the newest)
first="$(echo "$tags" | head -1)"
first="${first#v}"
awk -v h="## $first" '$0==h{skip=1;next} /^## /{skip=0} !skip' "$repo/CHANGELOG.md" >"$tmp/deleted1.md"
loop "$tmp/deleted1.md" deleted-oldest
if [ "$red" -ge 1 ]; then echo "ok   red: oldest section deleted"; else echo "FAIL a deleted oldest section was not caught"; fails=$((fails + 1)); fi

if [ "$fails" -eq 0 ]; then echo "all ok"; else echo "$fails failed"; exit 1; fi
