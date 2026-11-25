#!/usr/bin/env bash
# Self test of scripts/pin-refs.sh with counter-checks on a copy of the repo (never the real files):
# a valid SHA changes exactly the `uses:` lines (not comments); 'x#e', 'a&b', empty, too short, uppercase = rc 2 and NO file changed.
# A mutation (the ref check removed) must turn the bad-ref cases red.
# `--dir <repo>` (a foreign repo): only fs-release `uses:` lines of .github/workflows/*.yml change (@main, @v0.0.6, @sha), foreign actions,
# comments and other lines stay, a second run changes nothing, no .github/workflows = rc 2; mutations (anchor removed, pattern widened,
# --dir ignored, chain gate removed or made report-only) must turn it red; the chain gate (stub `fs-release`) is hard: rc != 0 = rc 4, nothing written.
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
  # the pins of the copy point to commits that do not exist in the throw-away repo: `@main` (always raised), the real pins are tested in self_checks
  grep -rlE 'fs-release/.+@' "$1/.github" "$1/actions" | while read -r f; do sed -i -E '/^[[:space:]]*(-[[:space:]]+)?uses:/ s|(fs-release/[A-Za-z0-9_./-]+)@[A-Za-z0-9_.-]+|\1@main|' "$f"; done
  git -C "$1" init -q && git -C "$1" add -A && git -C "$1" -c user.name=t -c user.email=t@t commit -qm base
  git -C "$1" tag v9.8.7
}
# bad_refs <dir> <label>: every bad ref must give rc 2 and leave the tree untouched
bad_refs() {
  local r rc
  for r in 'x#e' 'a&b' '' 'abc123' 'ABCDEF0123456789ABCDEF0123456789ABCDEF01' 'v1.2' '0123456789abcdef0123456789abcdef0123456;'; do
    bash "$1/scripts/pin-refs.sh" "$r" >/dev/null 2>&1; rc=$?
    if [ "$rc" -eq 2 ] && [ -z "$(git -C "$1" status --porcelain)" ]; then ok "$2 bad ref '$r' rc 2, nothing changed"; else bad "$2 bad ref '$r' rc=$rc or files changed"; git -C "$1" checkout -q . ; fi
  done
}

mk "$tmp/real" "$here/pin-refs.sh"
sha=$(git -C "$tmp/real" rev-parse HEAD)
uses_before="$(git -C "$tmp/real" grep -hE '^[[:space:]]*(-[[:space:]]+)?uses:.*FreeSynergy-Net/fs-release/' | wc -l)"
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
# A throw-away fs-release clone (FS_RELEASE_REPO) with a linear history and tags, plus a side commit unrelated to the tags' line.
mkrel() {
  local r=$1 t
  git init -q "$r"
  for t in v0.0.6 v0.0.9 v0.0.10 v0.0.11 v0.0.100 v0.1.0; do
    echo "$t" >"$r/f" && git -C "$r" add f && git -C "$r" -c user.name=t -c user.email=t@t commit -qm "$t" && git -C "$r" tag "$t"
  done
  git -C "$r" checkout -q --orphan side && git -C "$r" -c user.name=t -c user.email=t@t commit -qm side --allow-empty
  git -C "$r" checkout -q -f v0.1.0
}
mkrel "$tmp/rel"
export FS_RELEASE_REPO="$tmp/rel"
# Stand-in for `fs-release` (the throw-away clone has no pin chains): logs its arguments, rc from $tmp/stub.rc (default 0).
cat >"$tmp/stub-fs-release" <<STUB
#!/usr/bin/env bash
echo "\$*" >>"$tmp/stub.log"
exit "\$(cat "$tmp/stub.rc" 2>/dev/null || echo 0)"
STUB
chmod +x "$tmp/stub-fs-release"
export FS_RELEASE_BIN="$tmp/stub-fs-release"
tagsha() { git -C "$tmp/rel" rev-parse "$1^{commit}"; }
side="$(git -C "$tmp/rel" rev-parse side)"

 foreign_sha=$(tagsha v0.0.6)
checkout=actions/checkout@11bd71901bbe5b1630ceea73d27597364c9af683
# mkforeign <dir>: two workflows with every kind of line the script must treat differently
mkforeign() {
  mkdir -p "$1/.github/workflows"
  cat >"$1/.github/workflows/check.yml" <<YML
# comment: FreeSynergy/fs-release/.github/workflows/check.yml@main stays
# note: uses: FreeSynergy-Net/fs-release/.github/workflows/check.yml@main (old)
jobs:
  check:
    uses: FreeSynergy-Net/fs-release/.github/workflows/check.yml@main
  other:
    with:
      note: FreeSynergy/fs-release/x@main is no uses
YML
  cat >"$1/.github/workflows/release.yml" <<YML
jobs:
  steps:
    - uses: FreeSynergy-Net/fs-release/.github/workflows/release.yml@v0.0.6
    - uses: $checkout
    - uses: FreeSynergy-Net/fs-release/actions/sbom@$foreign_sha # pinned
    - uses: FreeSynergy/fs-release/actions/oldowner@main # the old owner is pinned too (until part 2 of the address wave is done)
    - uses: Evil/fs-release/actions/x@main # a third owner never
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
  n="$(git -C "$d" grep -hE '^[[:space:]]*(-[[:space:]]+)?uses: (FreeSynergy-Net|FreeSynergy)/fs-release/[^@]*@v0.0.10( |$)' | wc -l)"
  [ "$n" -eq 4 ] && ok "$l 4 fs-release uses: lines (both owners) now @v0.0.10" || bad "$l uses: lines @v0.0.10: $n (want 4)"
  git -C "$d" grep -q 'uses: Evil/fs-release/actions/x@main # a third owner never' && ok "$l a third owner's uses: line untouched" || bad "$l a third owner's line was changed"
  git -C "$d" grep -q "uses: $checkout" && ok "$l foreign action untouched" || bad "$l foreign action changed"
  git -C "$d" grep -q '# comment: FreeSynergy/fs-release/.github/workflows/check.yml@main' && git -C "$d" grep -q '# note: uses: FreeSynergy-Net/fs-release/.github/workflows/check.yml@main (old)' && git -C "$d" grep -q 'note: FreeSynergy/fs-release/x@main' && git -C "$d" grep -q '  ref: FreeSynergy/fs-release/.github/workflows/check.yml@main' && ok "$l comment and non-uses lines untouched" || bad "$l comment or non-uses line changed"
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

# never lower (--dir): target equal or descendant of every existing pin = ok; older, unrelated, unknown, or a foreign org = rc 3 and NOTHING written
nolower_checks() {
  local s=$1 l=$2 d="$tmp/n-$2" rc
  nl_case() { # <name> <old pin line ref or full owner/ref> <target> <want: ok|noop|red>
    local want=$4
    rm -rf "$d"; mkdir -p "$d/.github/workflows"
    printf 'jobs:\n  a:\n    uses: %s/fs-release/.github/workflows/check.yml@%s\n  b:\n    uses: %s/fs-release/actions/x@%s\n' "${2%%@*}" "${2##*@}" "${2%%@*}" "${2##*@}" >"$d/.github/workflows/c.yml"
    git -C "$d" init -q && git -C "$d" add -A && git -C "$d" -c user.name=t -c user.email=t@t commit -qm base
    bash "$s" --dir "$d" "$3" >/dev/null 2>&1; rc=$?
    case $want in
      ok) { [ "$rc" -eq 0 ] && git -C "$d" grep -q "FreeSynergy-Net/fs-release/.github/workflows/check.yml@$3\$" && ok "$l $1"; } || bad "$l $1 (rc=$rc)" ;;
      noop) { [ "$rc" -eq 0 ] && [ -z "$(git -C "$d" status --porcelain)" ] && ok "$l $1"; } || bad "$l $1 (rc=$rc)" ;;
      red) { [ "$rc" -eq 3 ] && [ -z "$(git -C "$d" status --porcelain)" ] && ok "$l $1"; } || bad "$l $1 (rc=$rc, want 3 and untouched)" ;;
    esac
  }
  nl_case "newer tag over an older tag is raised" FreeSynergy-Net@v0.0.9 v0.0.11 ok
  nl_case "SHA descendant over an older tag is raised" FreeSynergy-Net@v0.0.9 "$(tagsha v0.0.10)" ok
  nl_case "equal pin is a no-op" FreeSynergy-Net@v0.0.11 v0.0.11 noop
  nl_case "the same commit as a SHA pin is rewritten to the tag" FreeSynergy-Net@"$(tagsha v0.0.11)" v0.0.11 ok
  nl_case "old org name is rewritten to FreeSynergy-Net" FreeSynergy@v0.0.9 v0.0.10 ok
  nl_case "@main is raised" FreeSynergy-Net@main v0.0.10 ok
  nl_case "older tag target is red (v0.0.10 over v0.0.11)" FreeSynergy-Net@v0.0.11 v0.0.10 red
  nl_case "older SHA target is red" FreeSynergy-Net@v0.1.0 "$(tagsha v0.0.100)" red
  nl_case "v0.0.17-style lowering of a SHA pin is red" FreeSynergy-Net@"$(tagsha v0.1.0)" v0.0.10 red
  nl_case "unrelated commit as pin is red" FreeSynergy-Net@"$side" v0.0.10 red
  nl_case "unknown pin (not in the clone) is red, not overwritten" FreeSynergy-Net@v9.9.9 v0.0.10 red
  nl_case "unknown SHA pin is red" FreeSynergy-Net@0123456789abcdef0123456789abcdef01234567 v0.0.10 red
  nl_case "target not in the clone is red" FreeSynergy-Net@v0.0.9 v7.7.7 red
  nl_case "foreign org line stays untouched, no pin check on it" Evil@v0.0.11 v0.0.10 noop
  nl_case "an owner that only ENDS in FreeSynergy stays untouched (left boundary), no pin check" Not-FreeSynergy@v9.9.9 v0.0.10 noop
  nl_case "an owner that only ENDS in FreeSynergy-Net stays untouched (left boundary)" Not-FreeSynergy-Net@v0.0.11 v0.0.10 noop
  nl_case "only @main and a target the clone does not know is red" FreeSynergy-Net@main v7.7.7 red
  # two DIFFERENT pins in one file (one older, one newer than the target): the whole run is red, nothing written (a check of only one pin must not pass)
  nl_two() { # <pin a> <pin b> <target> <want rc>
    rm -rf "$d"; mkdir -p "$d/.github/workflows"
    printf 'jobs:\n  a:\n    uses: FreeSynergy-Net/fs-release/.github/workflows/check.yml@%s\n  b:\n    uses: FreeSynergy-Net/fs-release/actions/x@%s\n' "$1" "$2" >"$d/.github/workflows/c.yml"
    git -C "$d" init -q && git -C "$d" add -A && git -C "$d" -c user.name=t -c user.email=t@t commit -qm base
    bash "$s" --dir "$d" "$3" >/dev/null 2>&1; rc=$?
    if [ "$rc" -eq "$4" ] && { [ "$4" -eq 0 ] || [ -z "$(git -C "$d" status --porcelain)" ]; }; then ok "$l two pins $1 / $2 against $3 rc $4"; else bad "$l two pins $1 / $2 against $3 (rc=$rc, want $4)"; fi
  }
  nl_two v0.0.9 v0.1.0 v0.0.10 3
  nl_two v0.1.0 v0.0.9 v0.0.10 3
  nl_two v0.0.9 v0.0.11 v0.0.10 3
  nl_two v0.0.9 v0.0.6 v0.0.10 0
}
nolower_checks "$here/pin-refs.sh" real

# chain gate (--dir): `check-pin-chain --repo <clone> <full sha>` runs WITHOUT --melden before anything is written; rc != 0 (1 red, 2 bad input, tool missing) = rc 4, nothing written.
chain_checks() {
  local s=$1 l=$2 d="$tmp/c-$2" rc want
  want="check-pin-chain --repo $tmp/rel $(tagsha v0.0.10)"
  for stubrc in 0 1 2; do
    mkforeign "$d.$stubrc"; echo "$stubrc" >"$tmp/stub.rc"; : >"$tmp/stub.log"
    bash "$s" --dir "$d.$stubrc" v0.0.10 >/dev/null 2>&1; rc=$?
    if [ "$stubrc" -eq 0 ]; then
      { [ "$rc" -eq 0 ] && [ -n "$(git -C "$d.$stubrc" status --porcelain)" ]; } && ok "$l chain gate green: written" || bad "$l chain gate green (rc=$rc)"
    else
      { [ "$rc" -eq 4 ] && [ -z "$(git -C "$d.$stubrc" status --porcelain)" ]; } && ok "$l chain gate rc $stubrc: rc 4, nothing written" || bad "$l chain gate rc $stubrc (rc=$rc, want 4 and untouched)"
    fi
    [ "$(cat "$tmp/stub.log")" = "$want" ] && ok "$l chain gate rc $stubrc called as '$want' (no --melden)" || bad "$l chain gate call was: $(cat "$tmp/stub.log")"
  done
  mkforeign "$d.missing"; : >"$tmp/stub.log"
  FS_RELEASE_BIN="$tmp/no-such-tool" bash "$s" --dir "$d.missing" v0.0.10 >/dev/null 2>&1; rc=$?
  { [ "$rc" -eq 4 ] && [ -z "$(git -C "$d.missing" status --porcelain)" ]; } && ok "$l chain gate tool missing: rc 4, nothing written" || bad "$l chain gate tool missing (rc=$rc)"
  if ! command -v fs-release >/dev/null 2>&1; then
    mkforeign "$d.notool"
    out=$(env -u FS_RELEASE_BIN bash "$s" --dir "$d.notool" v0.0.10 2>&1); rc=$?
    { [ "$rc" -eq 4 ] && [ -z "$(git -C "$d.notool" status --porcelain)" ] && echo "$out" | grep -q 'FS_RELEASE_BIN'; } && ok "$l no tool at all: rc 4 with a clear message, nothing written, no cargo run" || bad "$l no tool (rc=$rc): $out"
  fi
  echo 0 >"$tmp/stub.rc"
}
chain_checks "$here/pin-refs.sh" real

# never lower, SELF mode (no --dir; review pin-chain-gate r4 S5): pins at a newer commit than the target = rc 3, nothing written, a message; the same or an
# older pin is raised. A throw-away copy whose own history gives the pins (c1 older, c2 newer).
self_checks() {
  local s=$1 l=$2 d="$tmp/self-$2" c1 c2 rc out
  mk "$d" "$s"
  c1=$(git -C "$d" rev-parse HEAD)
  git -C "$d" -c user.name=t -c user.email=t@t commit -qm two --allow-empty
  c2=$(git -C "$d" rev-parse HEAD)
  grep -rlE 'fs-release/.+@' "$d/.github" "$d/actions" | while read -r f; do sed -i -E "/^[[:space:]]*(-[[:space:]]+)?uses:/ s|(fs-release/[A-Za-z0-9_./-]+)@[A-Za-z0-9_.-]+|\1@$c2|" "$f"; done
  git -C "$d" add -A && git -C "$d" -c user.name=t -c user.email=t@t commit -qm pinned
  out=$(FS_RELEASE_REPO="$d" bash "$d/scripts/pin-refs.sh" "$c1" 2>&1); rc=$?
  if [ "$rc" -eq 3 ] && [ -z "$(git -C "$d" status --porcelain)" ] && echo "$out" | grep -q 'never lowered'; then ok "$l self mode: lowering is rc 3 with a message, nothing written"; else bad "$l self mode: lowering rc=$rc or files changed: $out"; git -C "$d" checkout -q .; fi
  out=$(FS_RELEASE_REPO="$d" bash "$d/scripts/pin-refs.sh" "$c2" 2>&1); rc=$?
  if [ "$rc" -eq 0 ] && [ -z "$(git -C "$d" status --porcelain)" ]; then ok "$l self mode: the same pin is a no-op"; else bad "$l self mode: same pin rc=$rc: $out"; fi
}
self_checks "$here/pin-refs.sh" real

# mutations of the --dir path: each must turn dir_checks (or the never-lower checks) red
red_by() { # <label> <sed expression on the script>
  local m="$tmp/m-$1.sh"
  sed -E "$2" "$here/pin-refs.sh" >"$m"
  if cmp -s "$m" "$here/pin-refs.sh"; then bad "mutation $1 did not change the script"; return; fi
  local res
  res="$(dir_checks "$m" "$1" 2>&1; nolower_checks "$m" "$1" 2>&1; self_checks "$m" "$1" 2>&1; chain_checks "$m" "$1" 2>&1)"
  if echo "$res" | grep -q '^FAIL'; then ok "mutation $1 turns the --dir checks red"; else bad "mutation $1 stayed green"; fi
}
red_by noanchor 's@\$0 ~ "\^\[\[:space:\]\]\*\(-\[\[:space:\]\]\+\)\?uses:"@$0 ~ "uses:"@'
red_by widened 's@re = "\(" owners "\)/fs-release/@re = "[A-Za-z0-9_-]+/fs-release/@'
red_by only-old-owner "s@^owners=.*@owners='FreeSynergy'@"
red_by nocheck 's@^  check_no_lower .*@  true@'
red_by noancestor 's@ \|\| git -C "\$repo" merge-base --is-ancestor "\$o" "\$n"@@'
red_by unknown-ok 's@rc=3; continue; }@continue; }@'
red_by first-pin-only 's@\| sort -u\)@| sort -u | head -1)@'
red_by last-pin-only 's@\| sort -u\)@| sort -u | tail -1)@'
red_by target-unknown-ok 's@(not found in \$repo" >&2; )return 3;@\1true;@'
red_by noboundary-awk 's@if \(RSTART > 1\) \{ c =@if (0) { c =@'
red_by noboundary-grep 's@\(\^\|[^)]*\)\(\$owners\)/fs-release@(\$owners)/fs-release@'
red_by selfnocheck 's#^\[ "\$\{\#selfs\[@\]\}" -eq 0 \] \|\| check_no_lower .*#true#'
red_by dirignored 's@^if \[ -n "\$dir" \]; then@if false; then@'
red_by nochain 's@^  chain_gate .*@  true@'
red_by cargo-fallback 's@^    echo "pin-refs: no fs-release tool.*@    true@; s@^    return 4$@    :@'
red_by chain-melden 's@check-pin-chain --repo@check-pin-chain --melden --repo@'

if [ "$fails" -eq 0 ]; then echo "all ok"; else echo "$fails failed"; exit 1; fi
