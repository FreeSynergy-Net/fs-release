#!/usr/bin/env bash
# Self test of the step `Blueprint check` of .github/workflows/check.yml (S5): the run block is taken out of the workflow and run with a STUB cargo and a STUB
# fs-blueprint (no network, no build). The good block passes every case; every mutant of the block fails at least one case; a mutant that changes nothing is a failure.
#   bash scripts/test_blueprint_check_step.sh
set -uo pipefail
here="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
repo="$(cd "$here/.." && pwd)"
tmp="$(mktemp -d)"
trap 'rm -rf "$tmp"' EXIT
fails=0
ok() { echo "ok   $1"; }
bad() { echo "FAIL $1"; fails=$((fails + 1)); }

# the run block of the step, as the runner would run it (python reads the workflow; the step is found by its name)
python3 -I - "$repo/.github/workflows/check.yml" >"$tmp/step.sh" <<'PY'
import sys, yaml
doc = yaml.safe_load(open(sys.argv[1]))
steps = doc["jobs"]["check"]["steps"]
hit = [s for s in steps if s.get("name", "").startswith("Blueprint check")]
assert len(hit) == 1, "the step must be there exactly once"
sys.stdout.write(hit[0]["run"])
PY
[ -s "$tmp/step.sh" ] || { echo "FAIL no run block"; exit 1; }
default_rev="$(sed -nE 's/.*blueprint-rev: .*default: "([0-9a-f]{40})".*/\1/p' "$repo/.github/workflows/check.yml")"
[ -n "$default_rev" ] || { echo "FAIL no default rev"; exit 1; }

# the stubs: cargo logs its words and makes a stub fs-blueprint under --root; the stub fs-blueprint logs its words and ends with $STUB_RC
mkdir -p "$tmp/stub"
cat >"$tmp/stub/cargo" <<'STUB'
#!/usr/bin/env bash
echo "cargo $*" >>"$STUB_LOG"
root=""
while [ "$#" -gt 0 ]; do [ "$1" = --root ] && root="$2"; shift; done
mkdir -p "$root/bin"
cat >"$root/bin/fs-blueprint" <<'FB'
#!/usr/bin/env bash
echo "fs-blueprint $*" >>"$STUB_LOG"
exit "${STUB_RC:-0}"
FB
chmod +x "$root/bin/fs-blueprint"
STUB
chmod +x "$tmp/stub/cargo"

# case <script> <name> <mode> <rev> <stub rc> <wanted rc> <log must contain (grep -E, "" = nothing)> <log must NOT contain> <output must contain>
cases_fail=0
case_run() {
  local script=$1 name=$2 mode=$3 rev=$4 srun=$5 want=$6 has=$7 hasnot=$8 outhas=$9
  local w="$tmp/w.$RANDOM"
  mkdir -p "$w/repo" "$w/rt"
  : >"$w/log"
  local rc
  (cd "$w/repo" && PATH="$tmp/stub:$PATH" STUB_LOG="$w/log" STUB_RC="$srun" RUNNER_TEMP="$w/rt" MODE="$mode" REV="$rev" DOCS="$w/docs" \
    bash -eo pipefail "$script" >"$w/out" 2>&1 </dev/null; echo $? >"$w/rc") || true
  rc=$(cat "$w/rc")
  local why=""
  [ "$rc" = "$want" ] || why="rc $rc, wanted $want"
  [ -z "$has" ] || grep -Eq -- "$has" "$w/log" || why="$why; log lacks '$has'"
  [ -z "$hasnot" ] || ! grep -Eq -- "$hasnot" "$w/log" || why="$why; log has '$hasnot'"
  [ -z "$outhas" ] || grep -Fq -- "$outhas" "$w/out" || why="$why; output lacks '$outhas'"
  [ ! -e "$w/repo/pwned" ] || why="$why; an injected command ran"
  if [ -n "$why" ]; then cases_fail=$((cases_fail + 1)); [ -z "${VERBOSE:-}" ] || echo "     $name: $why"; fi
}
all_cases() { # <script>: runs every case against it; counts failures in cases_fail
  local s=$1 r=$default_rev
  cases_fail=0
  case_run "$s" "warn, clean" warn "$r" 0 0 "cargo install --git https://github.com/FreeSynergy/fs-release --rev $r --locked --root .*/fsbp fs-blueprint" "" ""
  case_run "$s" "the check words" warn "$r" 0 0 "fs-blueprint check --repo .+ --talks .+/docs --drift" "" ""
  case_run "$s" "warn, errors" warn "$r" 1 0 "" "" "::warning::"
  case_run "$s" "warn, usage error" warn "$r" 2 0 "" "" "::warning::"
  case_run "$s" "block, clean" block "$r" 0 0 "" "" ""
  case_run "$s" "block, errors" block "$r" 1 1 "" "" ""
  case_run "$s" "block, usage error" block "$r" 2 2 "" "" ""
  for m in off "" bogus "warn;touch pwned" 'warn$(touch pwned)' "Warn" "block "; do
    case_run "$s" "bad mode '$m'" "$m" "$r" 0 2 "" "cargo|fs-blueprint" "::error::"
  done
  for v in main v0.0.12 "${r:0:39}" "${r}0" "${r^^}" "" 'abc;touch pwned' "\$(touch pwned)" "$(printf '%s\nmain' "$r")" "${r:0:39}g"; do
    case_run "$s" "bad rev '${v:0:20}'" warn "$v" 0 2 "" "cargo|fs-blueprint" "::error::"
  done
}

all_cases "$tmp/step.sh"
if [ "$cases_fail" = 0 ]; then ok "the step: every case passes"; else bad "the step: $cases_fail cases fail (VERBOSE=1 shows them)"; VERBOSE=1 all_cases "$tmp/step.sh" >/dev/null; fi

# mutants of the block: each one must fail a case
mutant() { # <name> <sed -E expression>
  sed -E "$2" "$tmp/step.sh" >"$tmp/m.sh"
  if cmp -s "$tmp/m.sh" "$tmp/step.sh"; then bad "mutant '$1' changed nothing (NOT APPLIED)"; return; fi
  all_cases "$tmp/m.sh"
  if [ "$cases_fail" -gt 0 ]; then ok "mutant '$1' is red ($cases_fail cases)"; else bad "mutant '$1' SURVIVES"; fi
}
mutant "mode check off" 's/^case "\$MODE" in warn\|block\) ;; \*\) .*esac$/true/'
mutant "mode check lets everything pass" 's/warn\|block\) ;; \*\)/*) ;; *)/'
mutant "rev check off" 's/^( *)if ! \[\[ "\$REV" =~ .*; then (.*)$/\1if false; then \2/'
mutant "rev check without the length" 's/\{40\}/+/'
mutant "rev check accepts upper case" 's/\[0-9a-f\]/[0-9a-fA-F]/'
mutant "rev check without the end anchor" 's/\{40\}\$ \]\]/{40} ]]/'
mutant "no --locked" 's/ --locked//'
mutant "no --rev" 's/ --rev "\$REV"//'
mutant "another repo" 's#FreeSynergy/fs-release#FreeSynergy/fs-types#'
mutant "no --drift" 's/ --drift//'
mutant "no --talks" 's/ --talks "\$DOCS"//'
mutant "no --repo" 's/ --repo "\$PWD"//'
mutant "error of the command lost" 's/ \|\| rc=\$\?//'
mutant "block does not fail" 's/if \[ "\$MODE" = block \]; then exit "\$rc"; fi//'
mutant "block always fails with 1" 's/exit "\$rc"; fi/exit 1; fi/'
mutant "warn without the warning" 's/echo "::warning::[^"]*"/true/'
mutant "warn fails" 's/echo "::warning::fs-blueprint check found errors \(mode warn: the job stays green\)"/exit 1/'
mutant "result ignored" 's/if \[ "\$rc" != 0 \]; then/if false; then/'

echo
if [ "$fails" = 0 ]; then echo "test_blueprint_check_step: all ok"; else echo "test_blueprint_check_step: $fails failures"; exit 1; fi
