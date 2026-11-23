#!/usr/bin/env bash
# Pattern: none (test). Self test of the step `Package names (allowlist)` of .github/workflows/check.yml and of scripts/check-apt-packages (N1d, way 2 of the review of bundle 7). The run block
# is taken out of the workflow (awk, no Python) and run as the runner does (bash -eo pipefail); the script runs the same cases. Red cases: rc 1; green: rc 0. The block must equal the body of
# the script, and the step must stand BEFORE the step `System packages` and run whenever apt-packages is set. Mutants (no check, options allowed, step after the install) must fail a case.
# TEST_WORKFLOW=<file> runs the checks against another check.yml (used to show them red against the old state).
#   bash scripts/test_check_apt_packages.sh
set -uo pipefail
here="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
repo="$(cd "$here/.." && pwd)"
tmp="$(mktemp -d "${TEST_TMPDIR:-${TMPDIR:-/tmp}}/apt-pkg.XXXXXX")"
trap 'rm -rf "$tmp"' EXIT
fails=0
ok() { echo "ok   $1"; }
bad() { echo "FAIL $1"; fails=$((fails + 1)); }
body() { sed -n '/^# --- body/,$p' "$repo/scripts/check-apt-packages" | tail -n +2; }
# structure <check.yml> -> prints one line per problem
structure() {
  local wf="$1" blk="$tmp/blk.$$"
  awk '
    /^      - name: Package names \(allowlist\)$/ { s = 1; next }
    s && /^        run: \|$/ { r = 1; next }
    s && r { if ($0 ~ /^          /) { print substr($0, 11); next } else exit }
  ' "$wf" >"$blk"
  [ -s "$blk" ] || { echo "no run block for the step Package names (allowlist)"; return; }
  body | cmp -s - "$blk" || echo "the check.yml block differs from the body of scripts/check-apt-packages"
  local a b
  a=$(grep -n '^      - name: Package names (allowlist)$' "$wf" | cut -d: -f1); b=$(grep -n '^      - name: System packages$' "$wf" | cut -d: -f1)
  { [ -n "$a" ] && [ -n "$b" ] && [ "$a" -lt "$b" ]; } || echo "the step Package names is not before System packages"
  awk '/^      - name: Package names \(allowlist\)$/ { s = 1; next } s && /^      - / { exit } s && /^        if: inputs.apt-packages != ..$/ { f = 1 } END { exit f ? 0 : 1 }' "$wf" || echo "the step Package names does not run when apt-packages is set"
  awk '/^      - name: Package names \(allowlist\)$/ { s = 1; next } s && /^      - / { exit } s && /PACKAGES: \$\{\{ inputs.apt-packages \}\}/ { f = 1 } END { exit f ? 0 : 1 }' "$wf" || echo "the step Package names does not read inputs.apt-packages"
}
wf="${TEST_WORKFLOW:-$repo/.github/workflows/check.yml}"
out="$(structure "$wf")"; if [ -z "$out" ]; then ok "structure: block = script body, step before the install, runs with apt-packages"; else bad "structure: $out"; fi
awk '
  /^      - name: Package names \(allowlist\)$/ { s = 1; next }
  s && /^        run: \|$/ { r = 1; next }
  s && r { if ($0 ~ /^          /) { print substr($0, 11); next } else exit }
' "$wf" >"$tmp/block.sh"
run() { PACKAGES="$2" bash -eo pipefail "$1" >"$tmp/out" 2>&1; rc=$?; }
red() { run "$blk" "$2"; if [ "$rc" = 1 ]; then ok "$tag red: $1"; else bad "$tag red: $1 (rc $rc)"; fi; }
green() { run "$blk" "$2"; if [ "$rc" = 0 ]; then ok "$tag green: $1"; else bad "$tag green: $1 (rc $rc)"; fi; }
cases() {
  red "apt option -oDpkg::Pre-Invoke" './tools/x.deb -oDpkg::Pre-Invoke::=./tools/evil.sh'
  red "option alone" "-oDpkg::Pre-Invoke::=id"; red "option --allow-unauthenticated" "libfoo --allow-unauthenticated"; red "-y" "-y"
  red "local deb ./x.deb" "./x.deb"; red "absolute path" "/tmp/x.deb"; red "pkg=version" "libfoo=1.0"; red "pkg/release" "libfoo/sid"; red "glob" "lib*"
  red "upper case" "Libfoo"; red "starts with a dot" ".foo"; red "starts with a plus" "+foo"; red "starts with a hyphen" "-foo"; red "semicolon" "libfoo;id"; red "dollar" 'libfoo$(id)'
  red "second line is an option" $'libfoo\n-oAPT::Update::Pre-Invoke::=id'; red "tab separated option" $'libfoo\t-y'; red "non-ASCII" "libfoé"; red "backtick" 'a`id`'
  green "one name" "libfoo"; green "names with + . -" "g++ libstdc++6 libx11-dev python3.12 libgtk-3-dev"; green "several lines and blanks" $'libssl-dev  pkg-config\nlibwayland-dev'; green "digit first" "7zip"
  green "empty" ""
}
blk="$tmp/block.sh"; tag=block; cases
blk="$repo/scripts/check-apt-packages"; tag=script; cases
if [ -z "${TEST_WORKFLOW:-}" ]; then
  mut() { # <name> <sed expr on the block>
    sed "$2" "$tmp/block.sh" >"$tmp/mut.sh"; before=$fails; blk="$tmp/mut.sh"; tag="mutant $1"; cases >"$tmp/mut.out"; after=$fails; fails=$before
    if [ "$after" -gt "$before" ]; then ok "mutant caught: $1 ($((after - before)) case(s) red)"; else bad "mutant NOT caught: $1"; fi
  }
  mut "no exit on a bad word" 's/^  exit 1$/  :/'
  mut "options allowed (leading hyphen in the class)" 's/\^\[a-z0-9\]\[a-z0-9+.-\]\*\$/^[-a-z0-9][-a-z0-9+.=\/]*$/'
  mut "denylist for -o only" 's/if ! \[\[ "\$word" =~ .* \]\]; then/if [[ "$word" == -o* ]]; then/'
  cp "$wf" "$tmp/wf-mut.yml"; sed -i 's/^      - name: Package names (allowlist)$/      - name: Package names (zz)/' "$tmp/wf-mut.yml"
  out="$(structure "$tmp/wf-mut.yml")"; if [ -n "$out" ]; then ok "mutant caught: step renamed/removed"; else bad "mutant NOT caught: step renamed"; fi
fi
if [ "$fails" -eq 0 ]; then echo "all ok"; else echo "$fails FAIL"; exit 1; fi
