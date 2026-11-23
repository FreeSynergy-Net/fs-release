#!/usr/bin/env bash
# Pattern: none (test). Self test of the step `Test environment` of .github/workflows/check.yml and of scripts/check-test-env (N1a, N1b, S12 of bundle 6). The run block is taken out of the
# workflow (awk, no Python) and run exactly as the runner does (bash -eo pipefail) with a made-up GITHUB_ENV; the script runs the same cases. Every red case checks rc 1 AND that nothing
# was written; the green cases check rc 0 and the exact text. The block must equal the body of the script (drift = red). Mutants of the block (denylist instead of allowlist, no exit,
# no TEST_ prefix) must each fail a case. TEST_BLOCK_FILE=<file> runs the cases against another block (used to show the cases red against the old denylist).
#   bash scripts/test_check_test_env.sh
set -uo pipefail
here="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
repo="$(cd "$here/.." && pwd)"
tmp="$(mktemp -d "${TEST_TMPDIR:-${TMPDIR:-/tmp}}/test-env.XXXXXX")"
trap 'rm -rf "$tmp"' EXIT
fails=0
ok() { echo "ok   $1"; }
bad() { echo "FAIL $1"; fails=$((fails + 1)); }
wf="$repo/.github/workflows/check.yml"
# the run block of the step "Test environment", 10 blanks of indent removed
awk '
  /^      - name: Test environment$/ { s = 1; next }
  s && /^        run: \|$/ { r = 1; next }
  s && r { if ($0 ~ /^          /) { print substr($0, 11); next } else exit }
' "$wf" >"$tmp/block.sh"
[ -s "$tmp/block.sh" ] || { echo "FAIL no run block for the step Test environment"; exit 1; }
[ "$(grep -c 'name: Test environment$' "$wf")" = 1 ] || { echo "FAIL the step is not there exactly once"; exit 1; }
sed -n '/^# --- body/,$p' "$repo/scripts/check-test-env" | tail -n +2 >"$tmp/body.sh"
if cmp -s "$tmp/block.sh" "$tmp/body.sh"; then ok "the check.yml block equals the body of scripts/check-test-env"; else bad "the check.yml block differs from the body of scripts/check-test-env"; diff "$tmp/block.sh" "$tmp/body.sh" | head -5; fi
BOM=$'\xef\xbb\xbf'
# run <block file> <TEST_ENV text>: rc in $rc, what was written in $tmp/genv
run() { : >"$tmp/genv"; TEST_ENV="$2" GITHUB_ENV="$tmp/genv" bash -eo pipefail "$1" >"$tmp/out" 2>&1; rc=$?; }
red() { # <name> <text>
  run "$blk" "$2"
  if [ "$rc" = 1 ] && [ ! -s "$tmp/genv" ]; then ok "$tag red: $1"; else bad "$tag red: $1 (rc $rc, written: $(head -c 60 "$tmp/genv"))"; fi
}
green() { # <name> <text>
  run "$blk" "$2"
  if [ "$rc" = 0 ] && [ "$(cat "$tmp/genv")" = "$2" ]; then ok "$tag green: $1"; else bad "$tag green: $1 (rc $rc)"; fi
}
cases() {
  # the trusted keys of bundle 5 (denylist before)
  red "CI=false" "CI=false"; red "ci=false" "ci=false"; red "blank before the key" " CI = false"; red "second line CI=false" $'TEST_MY=1\nCI=false'; red "CI<<EOF" "CI<<EOF"
  red "TEST_A<<EOF (no heredoc form)" $'TEST_A<<EOF\nx\nEOF'
  red "GATES_ONLY_GUARD=1" "GATES_ONLY_GUARD=1"; red "gates_only_guard=1" "gates_only_guard=1"; red "GATES_GUARD_MIDDLE" "GATES_GUARD_MIDDLE=id"
  for k in FS_IDENTITY_BASE PATH LD_PRELOAD BASH_ENV ENV GITHUB_PATH GITHUB_ENV RUNNER_TEMP GIT_DIR NODE_OPTIONS; do red "$k" "$k=x"; done
  # N1a: what the denylist let through
  red "BASH_FUNC_git%% (exported bash function)" 'BASH_FUNC_git%%=() { echo 0; }'
  red "SHELLOPTS=xtrace" "SHELLOPTS=xtrace"; red "PS4=\$(...)" 'PS4=$(id)'; red "BASHOPTS" "BASHOPTS=x"; red "HOME" "HOME=/tmp/x"
  red "XDG_CONFIG_HOME" "XDG_CONFIG_HOME=/tmp/x"; red "RUSTC_WRAPPER" "RUSTC_WRAPPER=/tmp/x"; red "RUSTFLAGS" "RUSTFLAGS=-Cx"; red "CARGO_HOME" "CARGO_HOME=/tmp/x"
  red "PYTHONPATH" "PYTHONPATH=/tmp/x"; red "GLOBIGNORE" "GLOBIGNORE=x"; red "CDPATH" "CDPATH=/x"
  red "lower case test_a" "test_a=1"; red "TEST without the underscore" "TEST=1"; red "name with a blank: TEST_A =1" "TEST_A =1"; red "no = at all" "TEST_A"
  # N1b: BOM and non-ASCII at the start of the file and in names
  red "BOM + GATES_ONLY_GUARD=1 as the first line" "${BOM}GATES_ONLY_GUARD=1"; red "BOM + TEST_A=1 as the first line" "${BOM}TEST_A=1"
  red "non-ASCII letter in a name" "TEST_é=1"; red "non-ASCII look-alike key" "ＣＩ=false"; red "CR inside the name" $'TEST_A\r=1'
  # green
  green "TEST_MY=1" "TEST_MY=1"; green "two lines, empty value, blank in the value" $'TEST_A=\nTEST_B=x y'; green "empty line between" $'TEST_A=1\n\nTEST_B=2'
}
blk="${TEST_BLOCK_FILE:-$tmp/block.sh}"; tag=block; cases
if [ -z "${TEST_BLOCK_FILE:-}" ]; then
  blk="$repo/scripts/check-test-env"; tag=script; cases
  # mutants of the block: each must fail at least one case (the test sees the difference)
  mut() { # <name> <sed expr>
    sed "$2" "$tmp/block.sh" >"$tmp/mut.sh"
    if cmp -s "$tmp/block.sh" "$tmp/mut.sh"; then bad "mutant $1 does not change the block"; return; fi
    local before=$fails; blk="$tmp/mut.sh"; tag="mutant $1"; QUIETM=1 cases >"$tmp/mut.out" 2>&1; local hit=$((fails - before)); fails=$before
    if [ "$hit" -gt 0 ]; then ok "mutant $1 is seen ($hit case(s) red)"; else bad "mutant $1: no case turned red"; fi
  }
  mut "no TEST_ prefix (any valid name)" 's/ || ! \[\[ "\$line" =~ \^TEST_\[A-Za-z0-9_\]+= \]\]//'
  mut "no exit 1 (written anyway)" 's/^  exit 1$/  :/'
  mut "bad collection off" 's/bad="\$bad \$n"/:/'
fi
if [ "$fails" -eq 0 ]; then echo "all ok"; else echo "$fails FAILED"; exit 1; fi
