#!/usr/bin/env bash
# Pin consistency of fs-release's OWN workflows (no network): every `uses: FreeSynergy/fs-release/...@<ref>` in .github/workflows/*.yml points at
# ONE common full commit SHA (the round of the pin cascade, scripts/pin-refs.sh), and no internal `uses:` is `@main` (or any other moving ref).
# It does not demand a particular SHA: round 1 (all on SHA A) and round 2 (all on SHA B) are both green; a mixed state is red.
# Counter-checks on copies: one `uses:` on another SHA, a `@main` left standing, a round-2 SHA swapped back to the older one.
#   bash scripts/test_pins_consistent.sh
set -uo pipefail
here="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
repo="$(cd "$here/.." && pwd)"
tmp="$(mktemp -d)"
trap 'rm -rf "$tmp"' EXIT
fails=0
ok() { echo "ok   $1"; }
bad() { echo "FAIL $1"; fails=$((fails + 1)); }

# refs <workflows dir>: one line per internal `uses:` (comments are not read): `<file>:<ref>`
refs() {
  local f
  for f in "$1"/*.yml; do
    grep -vE '^[[:space:]]*#' "$f" | grep -oE 'uses:[[:space:]]*FreeSynergy/fs-release/[^[:space:]@]+@[^[:space:]]+' | sed -E 's/.*@/'"$(basename "$f")"':/'
  done
}
# check <workflows dir>: 0 = consistent (at least one internal use, all refs are one and the same 40-char SHA)
check() {
  local r n
  r="$(refs "$1")"
  [ -n "$r" ] || { echo "no internal uses found in $1"; return 1; }
  if grep -qE ':main$' <<<"$r"; then echo "a moving ref @main is left"; return 1; fi
  if grep -vqE ':[0-9a-f]{40}$' <<<"$r"; then echo "an internal use is not a full 40-char SHA"; return 1; fi
  n="$(sed -E 's/^[^:]*://' <<<"$r" | sort -u | wc -l)"
  [ "$n" -eq 1 ] || { echo "$n different refs (one common SHA is required)"; return 1; }
  return 0
}

if msg="$(check "$repo/.github/workflows")"; then ok "real workflows: all internal uses on ONE SHA, no @main"; else bad "real workflows: $msg"; fi

# mutant <name> <sed expression> <file>: the mutated copy must be red
mutant() {
  local d="$tmp/$1"
  mkdir -p "$d"
  cp "$repo"/.github/workflows/*.yml "$d/"
  sed -i -E "$2" "$d/$3"
  if cmp -s "$d/$3" "$repo/.github/workflows/$3"; then bad "mutant '$1' changed nothing"; return; fi
  if check "$d" >/dev/null; then bad "mutant '$1' stays green"; else ok "mutant '$1' is red"; fi
}
sha="$(refs "$repo/.github/workflows" | head -1 | sed -E 's/^[^:]*://')"
mutant wrong-sha "0,/@${sha}/s//@0000000000000000000000000000000000000000/" check.yml
mutant main-left "0,/@${sha}/s//@main/" check.yml
mutant swapped-round "0,/_release-linux.yml@${sha}/s//_release-linux.yml@1111111111111111111111111111111111111111/" release.yml
mutant short-sha "0,/@${sha}/s//@${sha:0:7}/" check.yml
mutant tag-instead "0,/@${sha}/s//@v0.0.10/" check.yml
# a comment naming @main is not a use
d="$tmp/comment"; mkdir -p "$d"; cp "$repo"/.github/workflows/*.yml "$d/"; echo '# uses: FreeSynergy/fs-release/actions/x@main' >>"$d/check.yml"
if check "$d" >/dev/null; then ok "a comment with @main is not a use"; else bad "a comment with @main counted as a use"; fi

if [ "$fails" -eq 0 ]; then echo "all ok"; else echo "$fails failed"; exit 1; fi
