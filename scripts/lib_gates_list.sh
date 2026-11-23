#!/usr/bin/env bash
# Helpers of scripts/gates.sh for the Fehlerliste ([GATES-SAMMELLISTE]); sourced, own file so the rc rule has a test (scripts/test_gates_sammelliste.sh).

# gates_read_rc <file>: the rc a step wrote into <file>. A file that is missing, empty or no number (disk full, killed step) is RED (1), never green.
gates_read_rc() {
  local v=""
  [ -r "$1" ] && v="$(cat "$1")"
  if [[ "$v" =~ ^[0-9]+$ ]]; then echo "$v"; else echo 1; fi
}

# --- [GATE-BELEG]: the proof that goes into every worker report; sourced by gates.sh. Local gates = CI: one source (RUST_VERSION, this script, the shellcheck line of lint.yml), and a run that is NOT CI-equal says so, never silently green. "ja" only when EVERY CI step really ran.
ACTIONLINT_PIN=1.7.12 # the version lint.yml installs
# gates_tool_version <tool>: first line of its version output, or "missing".
gates_tool_version() { command -v "$1" >/dev/null 2>&1 && "$1" "${2:---version}" 2>&1 | sed -n 1,2p | tr '\n' ' ' || echo missing; }
# gates_ci_diff <want> <have> <root> <like>: one line per reason why this run is not CI-equal (empty = CI-equal).
gates_ci_diff() {
  case "$2" in "rustc $1 "*) ;; *) echo "toolchain: local '$2' != CI $1 (rustup missing: RUSTUP_TOOLCHAIN has no effect); lints of the newer toolchain are not CI lints" ;; esac
  [ -n "$4" ] || echo "mode standard: the lint job (shellcheck, actionlint) and the MSRV check did not run (use --like-ci)"
  [ -z "${FS_GATES_SKIP_TEST:-}" ] || echo "test $FS_GATES_SKIP_TEST SKIPPED (FS_GATES_SKIP_TEST)"
  [ -z "${FS_GATES_DIFF_BASE:-}" ] || echo "test scope narrowed (FS_GATES_DIFF_BASE=$FS_GATES_DIFF_BASE): not the whole workspace"
  git -C "$3" merge-base HEAD origin/main >/dev/null 2>&1 || echo "NICHT GEPRUEFT: origin/main fehlt (changed files unknown: clippy findings cannot be assigned to the branch)"
  command -v shellcheck >/dev/null 2>&1 || echo "NICHT GEPRUEFT: shellcheck fehlt (CI job lint)"
  if [ -d "$3/.github/workflows" ]; then
    if ! command -v actionlint >/dev/null 2>&1; then echo "NICHT GEPRUEFT: actionlint fehlt (CI job lint)"
    elif ! gates_tool_version actionlint -version | grep -q "$ACTIONLINT_PIN"; then echo "actionlint version differs from the CI pin $ACTIONLINT_PIN"; fi
  fi
  if grep -qE '^rust-version[[:space:]]*=' "$3/Cargo.toml" 2>/dev/null && ! command -v rustup >/dev/null 2>&1; then echo "NICHT GEPRUEFT: MSRV (rust-version in Cargo.toml; no rustup; CI takes the msrv INPUT of the caller, which may differ from Cargo.toml)"; fi
}
# gates_lint_shell / gates_lint_actions <root>: the lint job of CI: the shellcheck line of .github/workflows/lint.yml itself (same files, same flags), else all tracked *.sh; actionlint over the workflows.
gates_lint_shell() {
  local cmd; cd "$1" || return 1
  cmd="$(sed -n 's/^ *run: \(shellcheck .*\)$/\1/p' .github/workflows/lint.yml 2>/dev/null || true)"
  if [ -n "$cmd" ]; then bash -c "$cmd"; else git ls-files -z '*.sh' | xargs -0 -r shellcheck -x -S warning; fi
}
gates_lint_actions() { cd "$1" && actionlint; }
# gates_lint_selftests <root>: every self test / compile line of the CI jobs in lint.yml (`run: bash scripts/test_*.sh`, `scripts/test-packaging.sh`, `python3 -I -m py_compile ...`), same lines, run one by one;
# the rc of each is printed, the step is red when one failed. (CI runs them in the jobs lint, changelog-released, packaging-tests.) Byte code goes to the scratch folder, never into the tree.
# shellcheck disable=SC2154 # finding_work is set by gates.sh (sourced)
gates_lint_selftests() {
  local c bad=0 n=0; cd "$1" || return 1
  while read -r c; do
    n=$((n + 1)); local rc=0
    PYTHONPYCACHEPREFIX="$finding_work/pyc" bash -c "$c" >"$finding_work/st.out" 2>&1 || rc=$?
    if [ "$rc" != 0 ]; then bad=1; cat "$finding_work/st.out"; fi
    echo "selftest rc=$rc: $c" | cut -c1-150
  done < <(sed -nE 's/^ *run: (bash scripts\/test_[A-Za-z0-9_]+\.sh|scripts\/test-packaging\.sh|python3 -I -m py_compile .*)$/\1/p' .github/workflows/lint.yml 2>/dev/null)
  echo "selftests run: $n"
  return "$bad"
}
# gates_clippy_probe <root>: under a foreign toolchain the lints of the LIB errors (-D warnings) hide the test targets (AW18). Re-run clippy with -A ONLY for lints whose every location in the first run is in an
# UNCHANGED file (changed against origin/main); a lint that also occurs in a changed file is never allowed. Findings of the probe are listed, files changed against origin/main marked "CHANGED file". Never green by
# itself: the first clippy rc stays a finding. No origin/main = "NICHT GEPRUEFT" (and gates_ci_diff says NEIN).
# shellcheck disable=SC2154 # finding_work is set by gates.sh (sourced)
gates_clippy_probe() {
  local root="$1" out="$finding_work/clippy.out" po="$finding_work/probe.out" lints=() l f rc=0 base changed
  declare -A bad=() seen=()
  [ -r "$out" ] && [ "$(gates_read_rc "$out.rc")" != 0 ] || return 0
  base="$(git -C "$root" merge-base HEAD origin/main 2>/dev/null || true)"
  if [ -z "$base" ]; then echo "gates-beleg: clippy: Testziele nicht geprueft (NICHT GEPRUEFT: origin/main fehlt, changed files unknown)"; return 0; fi
  changed="$(git -C "$root" diff --name-only "$base")"
  while read -r l f; do
    seen[$l]=1
    if printf '%s\n' "$changed" | grep -qFx -- "$f"; then bad[$l]=1; fi
  done < <(awk '/^ *--> /{f=$2; sub(/:.*/,"",f)} /index\.html#/{l=$0; sub(/.*#/,"",l); print l, f}' "$out")
  for l in "${!seen[@]}"; do if [ -z "${bad[$l]:-}" ]; then lints+=(-A "clippy::$l"); fi; done
  if [ "${#lints[@]}" -eq 0 ]; then echo "gates-beleg: clippy: Testziele nicht geprueft (no lint that is only in unchanged files to allow); lints in changed files: ${!bad[*]}"; return 0; fi
  (cd "$root" && cargo clippy --workspace --all-targets --all-features --locked --keep-going -- -D warnings -W clippy::pedantic "${lints[@]}") >"$po" 2>&1 || rc=$?
  echo "gates-beleg: clippy-gegenprobe rc=$rc with ${lints[*]} (only lints found solely in unchanged files; test targets checked; first clippy stays red)"
  if [ "$rc" = 0 ]; then echo "gates-beleg: clippy-gegenprobe: no finding beyond the foreign-toolchain lints in unchanged files"; return 0; fi
  echo "gates-beleg: clippy-gegenprobe ROT/gefunden: files with findings (never 'toolchain'):"
  grep -oE -- '--> [^:]+' "$po" | sed 's/^--> //' | sort -u | while read -r f; do
    if printf '%s\n' "$changed" | grep -qFx -- "$f"; then echo "gates-beleg:   CHANGED file: $f"; else echo "gates-beleg:   $f"; fi
  done
  for l in "${!bad[@]}"; do echo "gates-beleg:   lint in a CHANGED file, not allowed: clippy::$l"; done
}
# gates_beleg <root> <want> <have> <like-ci> <step rcs...>: the Beleg block: head + time (a Beleg of an old head cannot pass), toolchain, tool versions, flags, every step with its rc, `CI-gleich: ja/NEIN` + reasons; NEIN = the branch CI at the head is the proof.
gates_beleg() {
  local root="$1" want="$2" have="$3" like="$4" diff
  shift 4
  diff="$(gates_ci_diff "$want" "$have" "$root" "$like")"
  echo "gates-beleg: head=$(git -C "$root" rev-parse HEAD 2>/dev/null || echo unknown); time=$(date -u +%Y-%m-%dT%H:%M:%SZ); mode=$([ -n "$like" ] && echo like-ci || echo standard)"
  echo "gates-beleg: rustc=${have}; ci-toolchain=${want}; shellcheck=$(gates_tool_version shellcheck); actionlint=$(gates_tool_version actionlint -version); clippy=--workspace --all-targets --all-features --locked --keep-going -- -D warnings -W clippy::pedantic; steps: $*"
  if [ -z "$diff" ]; then echo "gates-beleg: CI-gleich: ja"; return 0; fi
  echo "gates-beleg: CI-gleich: NEIN"
  printf 'gates-beleg: grund: %s\n' "$diff"
  case "$diff" in *toolchain:*) gates_clippy_probe "$root" ;; esac
  echo "gates-beleg: green here is NOT proof of a green CI: the branch CI at the head is the proof; the report names what did not run here and the CI run id"
}
