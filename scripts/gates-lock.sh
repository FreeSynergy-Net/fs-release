#!/usr/bin/env bash
# The lock check of scripts/gates.sh (called by it, and by tests): every shared library in exactly ONE source per Cargo.lock
# (scripts/check_lock_sources.py, list in scripts/lock-sources.toml). Introduction "warning first, error later": while the switch
# `strict` is false (scripts/lock-sources.toml, a repo's own .fs-lock-sources.toml, or env FS_LOCK_STRICT=1 for true) a violation is
# printed as a WARNING and the gate stays green; with strict it is red. rc 2 of the checker (unreadable lock) is always red.
#   bash scripts/gates-lock.sh <repo root>
set -uo pipefail
here="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
root="$(cd "$1" && pwd)" || exit 1
strict="$(python3 -I "$here/check_lock_sources.py" --print-strict "$root")" || { echo "gates FAILED: lock check configuration" >&2; exit 1; }
rc=0
python3 -I "$here/check_lock_sources.py" "$root" || rc=$?
if [ "$rc" -eq 0 ]; then
  echo "gates: lock check ok (one source per library)"
elif [ "$rc" -eq 1 ] && [ "$strict" = "false" ]; then
  echo "gates WARNING: lock check found a library in two sources; not red yet (strict = false), will be red once the repo is lifted to V1+ and strict is switched on" >&2
else
  echo "gates FAILED: lock check (strict = $strict, rc $rc)" >&2
  exit 1
fi
