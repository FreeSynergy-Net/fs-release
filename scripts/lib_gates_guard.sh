#!/usr/bin/env bash
# Guard of scripts/gates.sh (rule [GATES-WAECHTER]; sourced by gates.sh and by scripts/test_gates_guard.sh).
# 1. Lock: `.gates.lock` in the worktree root holds the PID of the running gates.sh. A second run in the same worktree stops
#    ("gates läuft bereits"); a lock whose PID is dead is replaced.
# 2. Snapshot: at the start and at the end the guard hashes the working tree: `git status --porcelain=v1 --untracked-files=all`,
#    `git rev-parse HEAD^{tree}` and sha256 of every file that git knows (tracked + untracked, ignored files such as target/ are
#    not listed; `.gates.lock` is left out). Both snapshots must be equal, otherwise gates is red with a fixed text and the full
#    list of changed files (never cut). Tools: git, sha256sum, diff only.

GATES_GUARD_LOCK_NAME=".gates.lock"
GATES_GUARD_TEXT="Arbeitsbaum während der Prüfung geändert"

# gates_guard_is_gates <pid>: rc 0 unless /proc shows a process that is not a gates.sh
gates_guard_is_gates() {
  [ -r "/proc/$1/cmdline" ] || return 0
  tr '\0' ' ' < "/proc/$1/cmdline" | grep -q 'gates\.sh'
}

# gates_guard_snapdir <root>: prints (and creates) the snapshot folder <git dir>/gates-guard
gates_guard_snapdir() {
  local d
  d="$(git -C "$1" rev-parse --absolute-git-dir)/gates-guard"
  mkdir -p "$d" && echo "$d"
}

# gates_guard_sweep <snapshot folder>: removes snapshots (snap.<pid>[.end]) of dead PIDs; only in that folder, only that name form
gates_guard_sweep() {
  local f pid
  for f in "$1"/snap.*; do
    [ -f "$f" ] || continue
    pid="${f##*/snap.}"; pid="${pid%.end}"
    case "$pid" in ''|*[!0-9]*) continue ;; esac
    kill -0 "$pid" 2>/dev/null || rm -f -- "$f"
  done
  return 0
}

# gates_guard_lock <root>: rc 0 = lock taken, rc 1 = another gates.sh lives in this worktree
gates_guard_lock() {
  local lock="$1/$GATES_GUARD_LOCK_NAME" pid
  for _ in 1 2; do
    if (set -o noclobber; echo "$$" > "$lock") 2>/dev/null; then
      return 0
    fi
    pid="$(tr -d '[:space:]' < "$lock" 2>/dev/null || true)"
    # Note (H2): a PID can be reused by an unrelated process after a SIGKILL; so a live PID counts only if it is a gates.sh
    # (/proc cmdline; without /proc the check falls back to "alive"). Two runs replacing the same stale lock at once is accepted.
    if [ -n "$pid" ] && [ "$pid" != "$$" ] && kill -0 "$pid" 2>/dev/null && gates_guard_is_gates "$pid"; then
      echo "gates FAILED: gates läuft bereits in $1 (PID $pid, $lock)" >&2
      return 1
    fi
    rm -f "$lock" # orphaned lock (PID dead or unreadable): replace it
  done
  echo "gates FAILED: cannot take $lock" >&2
  return 1
}

# gates_guard_unlock <root>: removes the lock only if it is ours
gates_guard_unlock() {
  local lock="$1/$GATES_GUARD_LOCK_NAME"
  if [ -f "$lock" ] && [ "$(tr -d '[:space:]' < "$lock" 2>/dev/null)" = "$$" ]; then
    rm -f "$lock"
  fi
  return 0
}

# gates_guard_snapshot <root> <out file>: deterministic description of the working tree
gates_guard_snapshot() {
  local root="$1" out="$2" f
  {
    echo "== status"
    git -C "$root" status --porcelain=v1 --untracked-files=all | grep -v -F -- " $GATES_GUARD_LOCK_NAME" || true
    echo "== tree"
    git -C "$root" rev-parse 'HEAD^{tree}'
    echo "== files"
    git -C "$root" ls-files -z --cached --others --exclude-standard | LC_ALL=C sort -z | while IFS= read -r -d '' f; do
      [ "$f" = "$GATES_GUARD_LOCK_NAME" ] && continue
      if [ -L "$root/$f" ]; then
        echo "link  $f -> $(readlink "$root/$f")"
      elif [ -f "$root/$f" ]; then
        echo "$(sha256sum < "$root/$f" | cut -d' ' -f1)  $f"
      else
        echo "MISSING  $f"
      fi
    done
  } > "$out"
}

# gates_guard_compare <begin snapshot> <end snapshot>: rc 0 = equal; otherwise fixed text + every changed line
gates_guard_compare() {
  if cmp -s "$1" "$2"; then
    return 0
  fi
  echo "gates FAILED: $GATES_GUARD_TEXT" >&2
  echo "changed (< at start, > at end):" >&2
  diff "$1" "$2" | grep -E '^[<>]' >&2 || true
  return 1
}
