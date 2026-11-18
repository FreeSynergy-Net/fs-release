#!/usr/bin/env bash
# Runs the packaging tests (tests/packaging/*.sh). Needs bash, jq, python3, cargo. Run it under scripts/limited.sh.
set -euo pipefail
here=$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)
# shellcheck source=../tests/packaging/harness.sh
source "$here/../tests/packaging/harness.sh"
# the CARGO_TARGET_DIR test must not silently go missing
[[ -f "$ROOT/tests/packaging/target_dir.sh" ]] || { echo "FAIL tests/packaging/target_dir.sh is missing"; exit 1; }
# ... and it must really run: pass() also appends a line to $T/tdcount for every "ok target_dir:" (pass runs in the main shell
# and in subshells, hence a file). target_dir.sh prints 8 such lines; fewer than 8 means it was emptied or cut short.
TD_MIN=8
eval "_orig_$(declare -f pass)"
pass() { _orig_pass "$@"; [[ "$1" == "target_dir:"* ]] && echo >> "$T/tdcount"; return 0; }
for t in "$ROOT"/tests/packaging/[a-z]*.sh; do
  [[ "$t" == */harness.sh ]] && continue
  echo "== $(basename "$t")"
  # shellcheck source=/dev/null
  source "$t"
done
td=0; [[ -f "$T/tdcount" ]] && td=$(wc -l < "$T/tdcount")
[[ $td -ge $TD_MIN ]] || { echo "FAIL target_dir.sh did not run ($td ok lines, expected at least $TD_MIN)"; exit 1; }
[[ $FAILS -eq 0 ]] || { echo "$FAILS test(s) failed"; exit 1; }
echo "all tests passed"
