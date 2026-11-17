#!/usr/bin/env bash
# Pattern: none (test). The reusable Windows gate (_windows-check.yml) and its caller example keep their rules: explicit runner (no *-latest),
# no moving internal ref (@main), same flags as check.yml, caller without secrets. Each rule has a counter-test on a mutated copy.
WF="$ROOT/.github/workflows/_windows-check.yml"
EX="$ROOT/examples/callers/windows.yml"
SA="$ROOT/actions/setup-rust/action.yml"
TW="$T/windows_check"; mkdir -p "$TW"

rules_ok() { # <workflow> <example>: 0 when every rule holds
  local wf=$1 ex=$2 sa=${3:-$SA} cwf cex
  cwf=$(grep -vE '^[[:space:]]*#' "$wf"); cex=$(grep -vE '^[[:space:]]*#' "$ex") # comments may name the forbidden words
  grep -q 'workflow_call:' "$wf" || return 1
  grep -qE 'runner: \{ type: string, default: windows-2022' "$wf" || return 1
  grep -qF 'runs-on: ${{ inputs.runner }}' "$wf" || return 1
  if grep -qE '[-]latest' <<< "$cwf$cex"; then return 1; fi
  if grep -qE 'uses:[[:space:]]*FreeSynergy/fs-release/[^ ]*@main' <<< "$cwf$cex"; then return 1; fi
  grep -q 'cargo fmt --check' "$wf" || return 1
  grep -qF -- '-D warnings $CLIPPY_EXTRA' "$wf" || return 1
  grep -q -- '--locked' "$wf" || return 1
  grep -q 'no-fail-fast' "$wf" || return 1
  grep -qE '_windows-check.yml@v[0-9]+\.[0-9]+\.[0-9]+' "$ex" || return 1
  if grep -q 'secrets:' <<< "$cex"; then return 1; fi
  grep -qE '^permissions:' "$wf" && grep -qF 'contents: read' "$wf" || return 1
  grep -qF 'Inputs are plain words' "$wf" && grep -qF '=~ ^windows-[0-9]+$' "$wf" || return 1
  grep -qE '^    timeout-minutes:' "$wf" || return 1 # on the job (4 spaces), not only the input line
  grep -qF 'cache: ${{ inputs.cache }}' "$wf" || return 1 # the input `cache` must reach setup-rust
  # no `${{ inputs.` inside a run block (a `run: |` line and the lines indented deeper than it)
  if awk '/^[[:space:]]+run: \|[[:space:]]*$/ { match($0, /^[[:space:]]+/); ind = RLENGTH; inrun = 1; next } inrun { match($0, /^[[:space:]]*/); if (RLENGTH <= ind && $0 !~ /^[[:space:]]*$/) inrun = 0 } inrun && /\$\{\{ *inputs\./ { bad = 1 } /^[[:space:]]+run: [^|]/ && /\$\{\{ *inputs\./ { bad = 1 } END { exit bad ? 0 : 1 }' <<< "$cwf"; then return 1; fi
  grep -qF "if: inputs.cache != 'false'" "$sa" || return 1 # setup-rust: the input really skips the cache step
  grep -qF "cache: \${{ github.event_name != 'workflow_dispatch' || inputs.cache }}" "$ex" || return 1 # the caller passes `cache` on
  grep -qE '^    inputs:' "$ex" && grep -qE '^      cache: \{ type: boolean' "$ex" || return 1 # workflow_dispatch offers `cache`
  return 0
}
if rules_ok "$WF" "$EX"; then pass "windows_check: real files keep every rule"; else fail "windows_check: real files break a rule"; fi

mutant() { # <name> <sed expression> <wf|ex|sa>: the mutated file must break a rule
  local f="$TW/$1.yml" wf=$WF ex=$EX sa=$SA
  case $3 in
    ex) sed -E "$2" "$EX" > "$f"; ex=$f ;;
    sa) sed -E "$2" "$SA" > "$f"; sa=$f ;;
    *) sed -E "$2" "$WF" > "$f"; wf=$f ;;
  esac
  if rules_ok "$wf" "$ex" "$sa"; then fail "windows_check: mutant '$1' stays green"; else pass "windows_check: mutant '$1' is red"; fi
}
mutant runner-latest 's/windows-2022/windows-latest/' wf
mutant moving-ref 's#setup-rust@ceddb45[0-9a-f]+#setup-rust@main#' wf
mutant no-locked 's/ --locked//g' wf
mutant caller-main 's#@v0\.0\.11#@main#' ex
mutant caller-secrets 's#^permissions:#secrets: inherit\npermissions:#' ex
mutant no-permissions 's/^permissions:/xpermissions:/' wf
mutant no-check-step 's/Inputs are plain words/Inputs/' wf
mutant no-runner-check 's/\^windows-\[0-9\]\+\$/^windows.*$/' wf
mutant no-timeout 's/^([[:space:]]+)timeout-minutes:/\1xtimeout:/' wf
mutant inputs-in-run 's#^( +)set -euo pipefail#\1set -euo pipefail ${{ inputs.packages }}#' wf
mutant cache-no-effect 's/cache: \$\{\{ inputs\.cache \}\}/cache: true/' wf
mutant job-timeout-gone '/^    timeout-minutes:/d' wf
mutant inputs-in-oneline-run 's#run: cargo fmt --check#run: cargo fmt --check ${{ inputs.packages }}#' wf
mutant setup-rust-if-gone "/^    - if: inputs.cache != 'false'/d" sa
mutant caller-no-cache 's#^      cache: \$\{\{.*$#      x: 1#' ex
mutant caller-no-dispatch-input '/^      cache: \{ type: boolean/d' ex
