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
like_ci=""
if [ "${1:-}" = "--like-ci" ]; then like_ci=1; shift; fi # [GATE-BELEG] also the lint job of CI (shellcheck, actionlint) and the MSRV note; the Beleg is printed in every mode
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
# shellcheck source=scripts/lib_gates_list.sh
. "$here/lib_gates_list.sh"
gates_guard_lock "$root" || exit 1
guard_snap=""
finding_work="$(mktemp -d "${TMPDIR:-/tmp}/gates-list.XXXXXX")"
trap 'gates_guard_unlock "$root"; if [ -n "$guard_snap" ]; then rm -f "$guard_snap" "$guard_snap.end"; fi; rm -rf "$finding_work"' EXIT
# The snapshot lives in <git dir>/gates-guard/ (inside the worktree's own .git: not in /tmp = RAM, invisible to the tree hash).
# A run killed by SIGKILL leaves its snapshot there; the next run sweeps snapshots of dead PIDs (own folder only).
guard_dir="$(gates_guard_snapdir "$root")"
gates_guard_sweep "$guard_dir"
guard_snap="$guard_dir/snap.$$"
gates_guard_snapshot "$root" "$guard_snap"

# --- [GATES-SAMMELLISTE] ---------------------------------------------------------------------------------------------------------------------
# step <name> <severity> <command...>: runs the command, passes its WHOLE output on (never cut), prints `gate <name>: rc=<rc>` and, when red, remembers the finding.
# It never stops the run. The rc is taken from the command itself (not from tee): pipefail does not help here, a pipe would hide it.
findings=()
finding_names=()
step_rcs=()
step() {
  local name="$1" severity="$2"
  shift 2
  local out="$finding_work/$name.out" rc=0
  { rc=0; "$@" 2>&1 || rc=$?; echo "$rc" > "$out.rc"; } | tee "$out" || true
  rc="$(gates_read_rc "$out.rc")"
  echo "gate $name: rc=$rc"
  step_rcs+=("$name=$rc")
  if [ "$rc" != 0 ]; then
    findings+=("$name|$severity|$rc|$*")
    finding_names+=("$name")
  fi
}
# nachpruefung_of <step> <output file>: the EXACT commands that check this finding again, one per line (the TUeV principle: the builder works through the list with
# `gates-komplett --nur-fehlerliste` until it is empty). `{ROOT}` and `{SCRIPTS}` are filled by that mode. Only plain words: a name taken from the output that has
# any other character is dropped (the broad command stays), so an error text can never put a command into the list.
nachpruefung_of() {
  local name="$1" out="$2" crates spec
  case "$name" in
    fmt) echo "cargo fmt --check" ;;
    clippy)
      crates="$(grep -oE -- '--> crates/[A-Za-z0-9_-]+' "$out" 2>/dev/null | sed 's|--> crates/||' | sort -u | awk '{ printf " -p %s", $0 }' || true)"
      echo "cargo clippy$crates --all-targets --all-features --locked -- -D warnings -W clippy::pedantic"
      ;;
    test)
      spec="$(awk '
        /^test [A-Za-z0-9_:]+ \.\.\. FAILED$/ { names = names " " $2 }
        /^error: test failed, to rerun pass `-p [A-Za-z0-9_-]+ (--lib|--bin [A-Za-z0-9_-]+|--test [A-Za-z0-9_-]+|--doc)`$/ {
          s = $0; sub(/^[^`]*`/, "", s); sub(/`$/, "", s)
          print "cargo test " s " --all-features --locked" (names == "" ? "" : " -- " substr(names, 2)); names = ""; found = 1
        }
        END { if (!found) print "cargo test --workspace --all-features --locked --no-fail-fast" }' "$out" 2>/dev/null || true)"
      echo "$spec"
      ;;
    rust-rules) echo "python3 -I {SCRIPTS}/check_rust_rules.py {ROOT}" ;;
    lock) echo "bash {SCRIPTS}/gates-lock.sh {ROOT}" ;;
    files) echo "bash {SCRIPTS}/gates-files.sh {ROOT}" ;;
    *) ;; # guard: no local check (a change of the tree while the gates run is only found by the full run)
  esac
}
# gates_findings_write: the list of ALL findings of this run as a Markdown checklist `fehlerliste-<branch>[-<run>].md` (step, severity, rc, command, `nachpruefung:` lines, first
# error lines) in $FS_GATES_FEHLERLISTE_DIR (default: the scratch folder; in CI $RUNNER_TEMP); NEVER inside the repo tree. It is also appended to $GITHUB_STEP_SUMMARY when that
# is set, and every finding is printed as a `gates-finding:` line (read by gates-komplett). No finding: the list says so (0 findings) and the run is green.
findings_file=""
gates_findings_write() {
  local dir branch safe f name severity rc cmd lines
  dir="${FS_GATES_FEHLERLISTE_DIR:-}"
  if [ -z "$dir" ]; then
    if [ -n "${CI:-}" ]; then dir="${RUNNER_TEMP:-${TMPDIR:-/tmp}}"; else dir="${TMPDIR_GATES:-/srv/FreeSynergy/.scratch}"; fi
  fi
  mkdir -p "$dir"
  case "$(cd "$dir" && pwd)/" in "$root"/*) dir="${TMPDIR:-/tmp}" ;; esac
  branch="$(git rev-parse --abbrev-ref HEAD 2>/dev/null || echo unknown)"
  if [ "$branch" = HEAD ]; then branch="$(git rev-parse --short HEAD 2>/dev/null || echo unknown)"; fi
  safe="$(printf '%s' "$branch" | tr -c 'A-Za-z0-9._-' '-')"
  findings_file="$dir/fehlerliste-$safe${FS_GATES_LAUF:+-$FS_GATES_LAUF}.md"
  {
    echo "# Fehlerliste $branch ($(git rev-parse --short HEAD 2>/dev/null || echo '?'))"
    echo
    echo "${#findings[@]} findings, $((${#step_rcs[@]} - ${#findings[@]})) of ${#step_rcs[@]} steps green (steps and rcs: see the gates-beleg lines)."
    for f in "${findings[@]}"; do
      IFS='|' read -r name severity rc cmd <<< "$f"
      echo
      echo "- [ ] **$name** (severity $severity, rc $rc): \`$cmd\`"
      nachpruefung_of "$name" "$finding_work/$name.out" | awk 'NF { printf "  - nachpruefung: %s\n", $0 }'
      if [ -z "$(nachpruefung_of "$name" "$finding_work/$name.out")" ]; then echo "  - nachpruefung: (none: only the full run checks this)"; fi
      lines="$(grep -E 'error|FAILED|panicked|Diff in|^warning|gates FAILED' "$finding_work/$name.out" 2>/dev/null | awk 'NR<=6 { printf "%s%s", (NR > 1 ? " // " : ""), $0 }' || true)"
      echo "  - first error lines: $lines"
    done
  } > "$findings_file"
  if [ -n "${GITHUB_STEP_SUMMARY:-}" ]; then cat "$findings_file" >> "$GITHUB_STEP_SUMMARY"; fi
  for f in "${findings[@]}"; do echo "gates-finding: $f"; done
}
# Test mode (scripts/test_gates_guard.sh) skips ONLY the cargo and check steps (the if-block below); GATES_GUARD_MIDDLE stands in
# for them. The end of the run (snapshot + compare + rc line) is the SAME code as in the real run: no second compare.
if [ -n "${GATES_ONLY_GUARD:-}" ]; then
  if [ -n "${GATES_GUARD_MIDDLE:-}" ]; then bash -c "$GATES_GUARD_MIDDLE"; fi
else
  echo "gates: rustc $(rustc --version)"
  # [GATES-SAMMELLISTE] every gate runs to its end, whatever an earlier one did: `step` keeps the rc of each (never lost, never cut) and the run collects ALL findings in ONE
  # list (see the end of this file). The gate stays HARD: one red step makes the exit code 1; only the stop at the first red is gone.
  # --locked: the gate builds exactly the committed Cargo.lock (a repo without a current lock is red; `cargo update` is a visible commit)
  # FS_GATES_SKIP_TEST: set ONLY by scripts/gates-komplett for its named exception (flat clone without tags of fs-release); never in CI. Visible, never silent.
  skip=()
  if [ -n "${FS_GATES_SKIP_TEST:-}" ]; then
    # only the ONE named exception (the entry of AUSNAHMEN in scripts/gates-komplett); any other value is refused, never skipped
    if [ "$FS_GATES_SKIP_TEST" != released_changelog_sections_equal_their_tags ]; then
      echo "gates FAILED: FS_GATES_SKIP_TEST=$FS_GATES_SKIP_TEST has no exception entry" >&2
      exit 2
    fi
    skip=(-- --skip "$FS_GATES_SKIP_TEST")
    echo "gates: test $FS_GATES_SKIP_TEST SKIPPED (FS_GATES_SKIP_TEST, named exception of gates-komplett; not green for this test)"
  fi
  step fmt style cargo fmt --check
  step clippy lint cargo clippy --workspace --all-targets --all-features --locked --keep-going -- -D warnings -W clippy::pedantic
  # FS_GATES_DIFF_BASE: set ONLY by CI on `unreviewed/*` branches (actions/gates input diff-base, never locally, never on main or a tag): the TEST gate then runs the crates that own a changed
  # file plus their dependants (scripts/affected_crates.py); fmt, clippy, rust-rules, lock and files stay on the whole workspace. Anything unclear (error, global file, every crate) = whole workspace, said below.
  testsel=(--workspace)
  if [ -n "${FS_GATES_DIFF_BASE:-}" ]; then
    sel_rc=0
    sel_out="$(python3 -I "$here/affected_crates.py" "$root" "$FS_GATES_DIFF_BASE")" || sel_rc=$?
    if [ "$sel_rc" = 0 ] && [[ "$sel_out" == "PACKAGES "* ]]; then
      read -ra sel_names <<<"${sel_out#PACKAGES }"
      testsel=()
      for sel_name in "${sel_names[@]}"; do testsel+=(-p "$sel_name"); done
      echo "gates: test NARROWED to the crates changed against $FS_GATES_DIFF_BASE and their dependants: ${sel_names[*]} (fmt, clippy, rules, lock, files stay full)"
    else
      echo "gates: test runs the WHOLE workspace (${sel_out:-affected_crates.py rc=$sel_rc})"
    fi
  fi
  step test error cargo test "${testsel[@]}" --all-features --locked --no-fail-fast "${skip[@]}"
  step rust-rules rule python3 -I "$rules" "$root"
  # lock check: one source per shared library in every Cargo.lock (warning first, error later: switch `strict`); own file, so it has a test.
  step lock error bash "$here/gates-lock.sh" "$root"
  # yaml-check and the workflows/ schema check: own file, so the evaluation of their exit codes has a test.
  step files error bash "$here/gates-files.sh" "$root"
  # --like-ci: the lint job of CI (the tool missing = "NICHT GEPRUEFT" in the Beleg, never silently skipped) and, with rustup, the MSRV job
  if [ -n "$like_ci" ]; then
    if command -v shellcheck >/dev/null 2>&1; then step shellcheck lint gates_lint_shell "$root"; fi
    if [ -d .github/workflows ] && command -v actionlint >/dev/null 2>&1; then step actionlint lint gates_lint_actions "$root"; fi
    msrv="$(sed -n 's/^rust-version[[:space:]]*=[[:space:]]*"\([0-9.]*\)".*/\1/p' Cargo.toml 2>/dev/null | head -n1 || true)"
    if [ -n "$msrv" ] && command -v rustup >/dev/null 2>&1; then step msrv error env RUSTUP_TOOLCHAIN="$msrv" cargo check --workspace --all-targets --locked; fi
  fi
fi
gates_guard_snapshot "$root" "$guard_snap.end"
if [ -n "${GATES_ONLY_GUARD:-}" ]; then
  gates_guard_compare "$guard_snap" "$guard_snap.end"
  echo "gate guard: rc=0"
else
  step guard error gates_guard_compare "$guard_snap" "$guard_snap.end"
  gates_findings_write
  gates_beleg "$root" "$RUSTUP_TOOLCHAIN" "$(rustc --version)" "$like_ci" "${step_rcs[@]}" | tee -a "$findings_file"
  if [ "${#findings[@]}" -ne 0 ]; then
    echo "gates FAILED: ${#findings[@]} step(s) red: $(printf '%s ' "${finding_names[@]}")(list: $findings_file)" >&2
    exit 1
  fi
fi
echo "gates ok: $root at $(git rev-parse --short HEAD 2>/dev/null || echo '?')"
