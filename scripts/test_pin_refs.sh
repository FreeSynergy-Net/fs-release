#!/usr/bin/env bash
# Self test of scripts/pin-refs.sh with counter-checks on a copy of the repo (never the real files):
# a valid SHA changes exactly the `uses:` lines (not comments); 'x#e', 'a&b', empty, too short, uppercase = rc 2 and NO file changed.
# A mutation (the ref check removed) must turn the bad-ref cases red.
# `--dir <repo>` (a foreign repo): only fs-release `uses:` lines of .github/workflows/*.yml change (@main, @v0.0.6, @sha), foreign actions,
# comments and other lines stay, a second run changes nothing, no .github/workflows = rc 2; mutations (anchor removed, pattern widened,
# --dir ignored) must turn it red.
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

# --- `--dir <repo>`: a foreign repo (a program that calls fs-release) ---
foreign_sha=6ebd3f8aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa
checkout=actions/checkout@11bd71901bbe5b1630ceea73d27597364c9af683
# mkforeign <dir>: two workflows with every kind of line the script must treat differently
mkforeign() {
  mkdir -p "$1/.github/workflows"
  cat >"$1/.github/workflows/check.yml" <<YML
# comment: FreeSynergy/fs-release/.github/workflows/check.yml@main stays
# note: uses: FreeSynergy/fs-release/.github/workflows/check.yml@main (old)
jobs:
  check:
    uses: FreeSynergy/fs-release/.github/workflows/check.yml@main
  other:
    with:
      note: FreeSynergy/fs-release/x@main is no uses
YML
  cat >"$1/.github/workflows/release.yml" <<YML
jobs:
  steps:
    - uses: FreeSynergy/fs-release/.github/workflows/release.yml@v0.0.6
    - uses: $checkout
    - uses: FreeSynergy/fs-release/actions/sbom@$foreign_sha # pinned
  ref: FreeSynergy/fs-release/.github/workflows/check.yml@main
YML
  echo "not a workflow" >"$1/.github/workflows/notes.txt"
  git -C "$1" init -q && git -C "$1" add -A && git -C "$1" -c user.name=t -c user.email=t@t commit -qm base
}
# dir_checks <script> <label>: prints ok/FAIL lines; the script is run from a place outside the foreign repo
dir_checks() {
  local s=$1 l=$2 d="$tmp/f-$2" out rc n
  mkforeign "$d"
  out="$(cd "$tmp" && bash "$s" --dir "$d" v0.0.10 2>&1)"; rc=$?
  [ "$rc" -eq 0 ] && ok "$l --dir rc 0" || bad "$l --dir rc=$rc"
  n="$(git -C "$d" grep -hE '^[[:space:]]*(-[[:space:]]+)?uses: FreeSynergy/fs-release/[^@]*@v0.0.10( |$)' | wc -l)"
  [ "$n" -eq 3 ] && ok "$l 3 fs-release uses: lines now @v0.0.10" || bad "$l uses: lines @v0.0.10: $n (want 3)"
  git -C "$d" grep -q "uses: $checkout" && ok "$l foreign action untouched" || bad "$l foreign action changed"
  git -C "$d" grep -q '# comment: FreeSynergy/fs-release/.github/workflows/check.yml@main' && git -C "$d" grep -q '# note: uses: FreeSynergy/fs-release/.github/workflows/check.yml@main (old)' && git -C "$d" grep -q 'note: FreeSynergy/fs-release/x@main' && git -C "$d" grep -q '  ref: FreeSynergy/fs-release/.github/workflows/check.yml@main' && ok "$l comment and non-uses lines untouched" || bad "$l comment or non-uses line changed"
  git -C "$d" grep -q '@v0.0.10 # pinned' && ok "$l trailing comment of a uses: line kept" || bad "$l trailing comment lost"
  [ -z "$(git -C "$d" diff --name-only | grep -v '^.github/workflows/.*\.yml$')" ] && ok "$l only workflow yml files changed" || bad "$l other files changed"
  echo "$out" | grep -q 'changed .*check.yml' && echo "$out" | grep -q 'changed .*release.yml' && ok "$l changed files reported" || bad "$l changed files not reported: $out"
  git -C "$d" -c user.name=t -c user.email=t@t commit -qam pinned
  out="$(cd "$tmp" && bash "$s" --dir "$d" v0.0.10 2>&1)"; rc=$?
  [ "$rc" -eq 0 ] && [ -z "$(git -C "$d" status --porcelain)" ] && echo "$out" | grep -q ' 0 file' && ok "$l second run idempotent" || bad "$l second run changed something (rc=$rc)"
  bash "$s" v0.0.11 --dir "$d" >/dev/null 2>&1 && git -C "$d" grep -q 'check.yml@v0.0.11' && ok "$l --dir after the ref works" || bad "$l --dir after the ref"
  bash "$s" --dir "$d" 'x#e' >/dev/null 2>&1; rc=$?
  [ "$rc" -eq 2 ] && ok "$l --dir with a bad ref rc 2" || bad "$l --dir bad ref rc=$rc"
}
dir_checks "$here/pin-refs.sh" real
mkdir -p "$tmp/empty"
out="$(bash "$here/pin-refs.sh" --dir "$tmp/empty" v0.0.10 2>&1)"; rc=$?
[ "$rc" -ne 0 ] && echo "$out" | grep -q 'no .github/workflows' && ok "no .github/workflows: rc $rc with a clear message" || bad "no .github/workflows: rc=$rc out=$out"
bash "$here/pin-refs.sh" --dir >/dev/null 2>&1; rc=$?
[ "$rc" -eq 2 ] && ok "--dir without a path rc 2" || bad "--dir without a path rc=$rc"
bash "$here/pin-refs.sh" v0.0.10 v0.0.11 >/dev/null 2>&1; rc=$?
[ "$rc" -eq 2 ] && ok "two refs rc 2" || bad "two refs rc=$rc"

# never lower (--dir, per `uses:` line): a line on a HIGHER tag stays, @main / SHA / older tags are raised, even when another line already has the new tag
nolower_checks() {
  local s=$1 l=$2 d="$tmp/n-$2"
  mkdir -p "$d/.github/workflows"
  printf 'jobs:\n  a:\n    uses: FreeSynergy/fs-release/.github/workflows/release.yml@v0.0.11\n  b:\n    uses: FreeSynergy/fs-release/.github/workflows/check.yml@v0.0.9\n  c:\n    uses: FreeSynergy/fs-release/actions/sbom@v0.1.0 # newer minor\n  d:\n    uses: FreeSynergy/fs-release/actions/x@v0.0.10\n  e:\n    uses: FreeSynergy/fs-release/actions/y@main\n  f:\n    uses: FreeSynergy/fs-release/actions/z@v0.0.100\n' >"$d/.github/workflows/release.yml"
  git -C "$d" init -q && git -C "$d" add -A && git -C "$d" -c user.name=t -c user.email=t@t commit -qm base
  bash "$s" --dir "$d" v0.0.10 >/dev/null 2>&1; rc=$?
  [ "$rc" -eq 0 ] && ok "$l never-lower run rc 0" || bad "$l never-lower rc=$rc"
  git -C "$d" grep -q 'release.yml@v0.0.11$' && git -C "$d" grep -q 'sbom@v0.1.0 # newer minor' && git -C "$d" grep -q 'actions/z@v0.0.100$' && ok "$l higher tags (v0.0.11, v0.1.0, v0.0.100) stay" || bad "$l a higher tag was lowered"
  git -C "$d" grep -q 'check.yml@v0.0.10$' && git -C "$d" grep -q 'actions/y@v0.0.10$' && git -C "$d" grep -q 'actions/x@v0.0.10$' && ok "$l v0.0.9 and @main are raised to v0.0.10" || bad "$l old pin not raised"
  bash "$s" --dir "$d" 0123456789abcdef0123456789abcdef01234567 >/dev/null 2>&1
  [ "$(git -C "$d" grep -c '@0123456789abcdef0123456789abcdef01234567' | awk -F: '{s+=$NF} END{print s}')" -eq 6 ] && ok "$l a SHA target replaces every ref (no tag comparison)" || bad "$l SHA target"
}
nolower_checks "$here/pin-refs.sh" real

# mutations of the --dir path: each must turn dir_checks (or the never-lower checks) red
red_by() { # <label> <sed expression on the script>
  local m="$tmp/m-$1.sh"
  sed -E "$2" "$here/pin-refs.sh" >"$m"
  if cmp -s "$m" "$here/pin-refs.sh"; then bad "mutation $1 did not change the script"; return; fi
  local res
  res="$(dir_checks "$m" "$1" 2>&1; nolower_checks "$m" "$1" 2>&1)"
  if echo "$res" | grep -q '^FAIL'; then ok "mutation $1 turns the --dir checks red"; else bad "mutation $1 stayed green"; fi
}
red_by noanchor 's@\$0 ~ "\^\[\[:space:\]\]\*\(-\[\[:space:\]\]\+\)\?uses:"@$0 ~ "uses:"@'
red_by widened 's@re = "\(" owners "\)/fs-release/@re = "[A-Za-z0-9_-]+/fs-release/@'
red_by only-old-owner "s@^owners=.*@owners='FreeSynergy'@"
red_by lowers 's@if \(!\(keep == 1.*\)\) tok@if (1) tok@'
red_by semver-as-text 's@ver\(old\) > ver\(ref\)@old > ref@'
red_by dirignored 's@^if \[ -n "\$dir" \]; then@if false; then@'

if [ "$fails" -eq 0 ]; then echo "all ok"; else echo "$fails failed"; exit 1; fi
