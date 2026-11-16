#!/usr/bin/env bash
# Self test of scripts/check-no-main-callers: the real tree is clean; `@main` in a uses: line is red (rc 1, file:line named), a full SHA and a tag are green,
# a line carrying the marker (same line or the line above) is green, prose / a commented line are green; a mutation (the pattern emptied) must turn the red cases green.
#   bash scripts/test_check_no_main_callers.sh
set -uo pipefail
here="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
tool="$here/check-no-main-callers"
tmp="$(mktemp -d)"
trap 'rm -rf "$tmp"' EXIT
fails=0
ok() { echo "ok   $1"; }
bad() { echo "FAIL $1"; fails=$((fails + 1)); }
u="us""es:"; w="FreeSynergy/fs-release/.github/workflows/check.yml"; a="FreeSynergy/fs-release/actions/setup-rust"
sha=f1211066d0072c6319678659e845c19ae4384bd0
marker="# negative-test: allow @main"

python3 -I "$tool" >/dev/null 2>&1 && ok "the real tree has no @main caller" || bad "the real tree has an @main caller"

# run <tool> <expected rc> <label> <line> [<line-above>]
run() {
  local t="$1" want="$2" label="$3" line="$4" above="${5:-}" rc
  rm -rf "$tmp/t"; mkdir -p "$tmp/t/.github/workflows"
  { [ -n "$above" ] && printf '%s\n' "$above"; printf '%s\n' "$line"; } >"$tmp/t/.github/workflows/x.yml"
  python3 -I "$t" "$tmp/t" >"$tmp/o" 2>&1; rc=$?
  if [ "$rc" -ne "$want" ]; then echo "FAIL $label: rc=$rc want $want"; return 1; fi
  if [ "$want" -eq 1 ] && ! grep -q 'x.yml:[0-9]*:' "$tmp/o"; then echo "FAIL $label: file:line not named"; return 1; fi
  echo "ok   $label"
}
cases() { # <tool> <prefix>
  local t="$1" p="$2"
  run "$t" 1 "$p red: workflow @main" "    $u $w@main"
  run "$t" 1 "$p red: step action @main" "      - $u $a@main"
  run "$t" 1 "$p red: @main with trailing comment" "    $u $w@main # later"
  run "$t" 1 "$p red: quoted @main" "    $u \"$w@main\""
}
red_cases() { cases "$1" "$2"; }
green_cases() {
  local t="$1" p="$2"
  run "$t" 0 "$p green: full SHA" "    $u $w@$sha"
  run "$t" 0 "$p green: tag" "    $u $w@v0.0.8"
  run "$t" 0 "$p green: branch name containing main" "    $u $w@maintenance"
  run "$t" 0 "$p green: marker on the same line" "    $u $w@main   $marker"
  run "$t" 0 "$p green: marker on the line above" "    $u $w@main" "    $marker"
  run "$t" 0 "$p green: commented-out line" "#   $u $w@main"
  run "$t" 0 "$p green: prose" "the programs call $w@main in prose"
  run "$t" 0 "$p green: other repo @main" "    $u actions/checkout@main"
}
out="$( { red_cases "$tool" real; green_cases "$tool" real; } )"
echo "$out"
echo "$out" | grep -q '^FAIL' && fails=$((fails + 1))

# the marker two lines above must NOT excuse the line
printf '%s\n' "    $marker" "    # filler" "    $u $w@main" >"$tmp/m.yml"
python3 -I "$tool" "$tmp/m.yml" >/dev/null 2>&1; [ $? -eq 1 ] && ok "marker two lines above does not excuse" || bad "marker two lines above excused the line"

# the committed negative fixture is green with its marker and red without it
fx="$here/../tests/fixtures/negative/main-caller.yml"
python3 -I "$tool" "$fx" >/dev/null 2>&1 && ok "committed negative fixture is green with its marker" || bad "committed negative fixture is red"
grep -q "$marker" "$fx" && sed "s/$marker//" "$fx" >"$tmp/nomarker.yml"
python3 -I "$tool" "$tmp/nomarker.yml" >/dev/null 2>&1; [ $? -eq 1 ] && ok "the same fixture without the marker is red" || bad "fixture without marker is not red"

# bad input
python3 -I "$tool" "$tmp/does-not-exist" >/dev/null 2>&1; [ $? -eq 2 ] && ok "missing path rc 2" || bad "missing path not rc 2"

# mutation 1: the pattern can never match -> the red cases must turn red (i.e. FAIL lines appear)
sed 's/^USES_MAIN = .*/USES_MAIN = re.compile(r"(?!x)x")/' "$tool" >"$tmp/mut1"
m1="$(red_cases "$tmp/mut1" mut)"
echo "$m1" | grep -q '^FAIL' && ok "mutation (pattern emptied): the red cases fail" || bad "mutation (pattern emptied) went unnoticed"
# mutation 2: the marker is ignored -> the marker cases must fail
sed 's/^MARKER = .*/MARKER = "\\0never"/' "$tool" >"$tmp/mut2"
m2="$(green_cases "$tmp/mut2" mut)"
echo "$m2" | grep -q '^FAIL.*marker' && ok "mutation (marker ignored): the marker cases fail" || bad "mutation (marker ignored) went unnoticed"

[ "$fails" -eq 0 ] && { echo "all ok"; exit 0; } || { echo "$fails failure(s)"; exit 1; }
