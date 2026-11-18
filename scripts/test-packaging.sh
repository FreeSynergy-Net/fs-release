#!/usr/bin/env bash
# Runs the packaging tests (tests/packaging/*.sh). Needs bash, jq, python3, cargo. Run it under scripts/limited.sh.
set -euo pipefail
here=$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)
# shellcheck source=../tests/packaging/harness.sh
source "$here/../tests/packaging/harness.sh"
# the CARGO_TARGET_DIR test must not silently go missing
[[ -f "$ROOT/tests/packaging/target_dir.sh" ]] || { echo "FAIL tests/packaging/target_dir.sh is missing"; exit 1; }
for t in "$ROOT"/tests/packaging/[a-z]*.sh; do
  [[ "$t" == */harness.sh ]] && continue
  echo "== $(basename "$t")"
  # shellcheck source=/dev/null
  source "$t"
done
[[ $FAILS -eq 0 ]] || { echo "$FAILS test(s) failed"; exit 1; }
echo "all tests passed"
