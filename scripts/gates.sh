#!/usr/bin/env bash
# The one measure, as one command with a real exit code (ADR 0043, coding rules). This file is the ONLY definition
# of the checks and their flags: .github/workflows/check.yml runs it through actions/gates, so CI and local cannot
# drift (tests/packaging/gates.sh enforces that). The toolchain is the one in RUST_VERSION (pinned; see DESIGN.md).
#   scripts/gates.sh <repo root> && git push ... && git tag ...
# Run it on a fresh clone. Do not rely on `set -e` in a tool call: the shell wrapper runs commands inside an
# `&&` list, where bash ignores it. Inside this script it works.
set -euo pipefail
# Every gate prints its rc; a failing one prints its line and rc on stderr (the ERR trap). Never cut a gate output with `tail`/`head`
# (a pipe into them hides the exit code of the gate): rule [GATE-PIPEFAIL], pinned by crates/fs-release/tests/workflows_pflege12.rs.
trap 'echo "gates FAILED: line $LINENO: rc=$?" >&2' ERR
here="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
root="$(cd "$1" && pwd)"
rules="$here/check_rust_rules.py"
RUSTUP_TOOLCHAIN="$(tr -d '[:space:]' < "$here/../RUST_VERSION")"
export RUSTUP_TOOLCHAIN
cd "$root"
if [ -z "${CI:-}" ]; then
  export CARGO_BUILD_JOBS="${CARGO_BUILD_JOBS:-4}"
  export TMPDIR="${TMPDIR_GATES:-/srv/FreeSynergy/.scratch}"
  if [ -n "$(git status --porcelain --untracked-files=no)" ]; then
    echo "gates FAILED: uncommitted changes in $root" >&2
    exit 1
  fi
fi
# Guard (rule [GATES-WAECHTER], scripts/lib_gates_guard.sh): lock `.gates.lock` + hash of the working tree at start and end.
# shellcheck source=scripts/lib_gates_guard.sh
. "$here/lib_gates_guard.sh"
gates_guard_lock "$root" || exit 1
guard_snap=""
trap 'gates_guard_unlock "$root"; if [ -n "$guard_snap" ]; then rm -f "$guard_snap" "$guard_snap.end"; fi' EXIT
# The snapshot lives in <git dir>/gates-guard/ (inside the worktree's own .git: not in /tmp = RAM, invisible to the tree hash).
# A run killed by SIGKILL leaves its snapshot there; the next run sweeps snapshots of dead PIDs (own folder only).
guard_dir="$(gates_guard_snapdir "$root")"
gates_guard_sweep "$guard_dir"
guard_snap="$guard_dir/snap.$$"
gates_guard_snapshot "$root" "$guard_snap"
# Test mode (scripts/test_gates_guard.sh) skips ONLY the cargo and check steps (the if-block below); GATES_GUARD_MIDDLE stands in
# for them. The end of the run (snapshot + compare + rc line) is the SAME code as in the real run: no second compare.
if [ -n "${GATES_ONLY_GUARD:-}" ]; then
  if [ -n "${GATES_GUARD_MIDDLE:-}" ]; then bash -c "$GATES_GUARD_MIDDLE"; fi
else
  echo "gates: rustc $(rustc --version)"
  cargo fmt --check
  echo "gate fmt: rc=0"
  # --locked: the gate builds exactly the committed Cargo.lock (a repo without a current lock is red; `cargo update` is a visible commit)
  cargo clippy --workspace --all-targets --all-features --locked -- -D warnings -W clippy::pedantic
  echo "gate clippy: rc=0"
  cargo test --workspace --all-features --locked
  echo "gate test: rc=0"
  python3 -I "$rules" "$root"
  echo "gate rust-rules: rc=0"
  # lock check: one source per shared library in every Cargo.lock (warning first, error later: switch `strict`); own file, so it has a test.
  bash "$here/gates-lock.sh" "$root"
  echo "gate lock: rc=0"
  # yaml-check and the workflows/ schema check: own file, so the evaluation of their exit codes has a test.
  bash "$here/gates-files.sh" "$root"
  echo "gate files: rc=0"
fi
gates_guard_snapshot "$root" "$guard_snap.end"
gates_guard_compare "$guard_snap" "$guard_snap.end"
echo "gate guard: rc=0"
echo "gates ok: $root at $(git rev-parse --short HEAD 2>/dev/null || echo '?')"
