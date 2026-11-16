#!/usr/bin/env bash
# Self test of scripts/pin-refs.sh with counter-checks on a copy of the repo (never the real files):
# a valid SHA changes exactly the `uses:` lines (not comments); 'x#e', 'a&b', empty, too short, uppercase = rc 2 and NO file changed.
# A mutation (the ref check removed) must turn the bad-ref cases red.
#   bash scripts/test_pin_refs.sh
set -uo pipefail
here="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
repo="$(cd "$here/.." && pwd)"
tmp="$(mktemp -d)"
trap 'rm -rf "$tmp"' EXIT
fails=0
ok() { echo "ok   $1"; }
bad() { echo "FAIL $1"; fails=$((fails + 1)); }

# mk <dir> <script>: a throw-away git repo with the workflows and actions of this repo and the given pin script
mk() {
  mkdir -p "$1/scripts"
  cp -r "$repo/.github" "$repo/actions" "$1/"
  cp "$2" "$1/scripts/pin-refs.sh"
  git -C "$1" init -q && git -C "$1" add -A && git -C "$1" -c user.name=t -c user.email=t@t commit -qm base
}
# bad_refs <dir> <label>: every bad ref must give rc 2 and leave the tree untouched
bad_refs() {
  local r rc
  for r in 'x#e' 'a&b' '' 'abc123' 'ABCDEF0123456789ABCDEF0123456789ABCDEF01' 'v1.2' '0123456789abcdef0123456789abcdef0123456;'; do
    bash "$1/scripts/pin-refs.sh" "$r" >/dev/null 2>&1; rc=$?
    if [ "$rc" -eq 2 ] && [ -z "$(git -C "$1" status --porcelain)" ]; then ok "$2 bad ref '$r' rc 2, nothing changed"; else bad "$2 bad ref '$r' rc=$rc or files changed"; git -C "$1" checkout -q . ; fi
  done
}

sha=0123456789abcdef0123456789abcdef01234567
mk "$tmp/real" "$here/pin-refs.sh"
uses_before="$(git -C "$tmp/real" grep -hE '^[[:space:]]*(-[[:space:]]+)?uses:.*FreeSynergy/fs-release/' | wc -l)"
bash "$tmp/real/scripts/pin-refs.sh" "$sha" >/dev/null; rc=$?
[ "$rc" -eq 0 ] && ok "valid SHA rc 0" || bad "valid SHA rc=$rc"
changed="$(git -C "$tmp/real" diff -U0 | grep -c '^+[^+]')"
[ "$changed" -eq "$uses_before" ] && [ "$changed" -gt 0 ] && ok "exactly the $uses_before uses: lines changed" || bad "changed lines $changed vs uses: lines $uses_before"
if git -C "$tmp/real" diff -U0 | grep '^+[^+]' | grep -vqE '^\+[[:space:]]*(-[[:space:]]+)?uses:'; then bad "a line that is not uses: was changed"; else ok "only uses: lines changed (no comment)"; fi
[ "$(git -C "$tmp/real" grep -c "@$sha" -- .github actions | awk -F: '{s+=$NF} END{print s}')" -eq "$uses_before" ] && ok "all uses: lines carry the SHA" || bad "SHA count differs"
git -C "$tmp/real" checkout -q .
bash "$tmp/real/scripts/pin-refs.sh" v9.8.7 >/dev/null && ok "vX.Y.Z accepted" || bad "vX.Y.Z rejected"
git -C "$tmp/real" checkout -q .
bad_refs "$tmp/real" real

# mutation: the ref check removed: the bad-ref cases must go red
grep -v '^\[\[ \$ref =~' "$here/pin-refs.sh" >"$tmp/mutant.sh"
if cmp -s "$tmp/mutant.sh" "$here/pin-refs.sh"; then bad "mutation did not change the script"; fi
mk "$tmp/mut" "$tmp/mutant.sh"
out="$(bad_refs "$tmp/mut" mutant 2>&1)"
if echo "$out" | grep -q '^FAIL'; then ok "mutation (check removed) turns the bad-ref cases red"; else bad "mutation stayed green"; fi

if [ "$fails" -eq 0 ]; then echo "all ok"; else echo "$fails failed"; exit 1; fi
