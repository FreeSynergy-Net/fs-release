#!/usr/bin/env bash
# Pattern: none (test). The reusable Windows gate (_windows-check.yml) and its caller example keep their rules: explicit runner (no *-latest),
# no moving internal ref (@main), same flags as check.yml, caller without secrets. Each rule has a counter-test on a mutated copy.
WF="$ROOT/.github/workflows/_windows-check.yml"
EX="$ROOT/examples/callers/windows.yml"
TW="$T/windows_check"; mkdir -p "$TW"

rules_ok() { # <workflow> <example>: 0 when every rule holds
  local wf=$1 ex=$2 cwf cex
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
  return 0
}
if rules_ok "$WF" "$EX"; then pass "windows_check: real files keep every rule"; else fail "windows_check: real files break a rule"; fi

mutant() { # <name> <sed expression> <wf|ex>: the mutated file must break a rule
  local f="$TW/$1.yml" wf=$WF ex=$EX
  if [[ $3 == ex ]]; then sed -E "$2" "$EX" > "$f"; ex=$f; else sed -E "$2" "$WF" > "$f"; wf=$f; fi
  if rules_ok "$wf" "$ex"; then fail "windows_check: mutant '$1' stays green"; else pass "windows_check: mutant '$1' is red"; fi
}
mutant runner-latest 's/windows-2022/windows-latest/' wf
mutant moving-ref 's#setup-rust@ceddb45[0-9a-f]+#setup-rust@main#' wf
mutant no-locked 's/ --locked//g' wf
mutant caller-main 's#@v0\.0\.11#@main#' ex
mutant caller-secrets 's#^permissions:#secrets: inherit\npermissions:#' ex
