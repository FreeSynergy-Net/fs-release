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
  [ -z "${FS_GATES_DIFF_BASE:-}" ] || echo "test NARROWED to the crates changed against $FS_GATES_DIFF_BASE"
  command -v shellcheck >/dev/null 2>&1 || echo "NICHT GEPRUEFT: shellcheck fehlt (CI job lint)"
  if [ -d "$3/.github/workflows" ]; then
    if ! command -v actionlint >/dev/null 2>&1; then echo "NICHT GEPRUEFT: actionlint fehlt (CI job lint)"
    elif ! gates_tool_version actionlint -version | grep -q "$ACTIONLINT_PIN"; then echo "actionlint version differs from the CI pin $ACTIONLINT_PIN"; fi
  fi
  if grep -qE '^rust-version[[:space:]]*=' "$3/Cargo.toml" 2>/dev/null && ! command -v rustup >/dev/null 2>&1; then echo "NICHT GEPRUEFT: MSRV (rust-version in Cargo.toml; no rustup, CI msrv job decides)"; fi
}
# gates_lint_shell / gates_lint_actions <root>: the lint job of CI: the shellcheck line of .github/workflows/lint.yml itself (same files, same flags), else all tracked *.sh; actionlint over the workflows.
gates_lint_shell() {
  local cmd; cd "$1" || return 1
  cmd="$(sed -n 's/^ *run: \(shellcheck .*\)$/\1/p' .github/workflows/lint.yml 2>/dev/null || true)"
  if [ -n "$cmd" ]; then bash -c "$cmd"; else git ls-files -z '*.sh' | xargs -0 -r shellcheck -x -S warning; fi
}
gates_lint_actions() { cd "$1" && actionlint; }
# gates_clippy_probe <root>: under a foreign toolchain the lints of the LIB errors (-D warnings) hide the test targets (AW18). Re-run clippy with exactly those lints allowed (-A), so the test targets are checked;
# prints the result and every finding in a file CHANGED against origin/main. Never green by itself: the first clippy rc stays a finding.
gates_clippy_probe() {
  local root="$1" out="$finding_work/clippy.out" po="$finding_work/probe.out" lints=() l rc=0 base changed
  [ -r "$out" ] && [ "$(gates_read_rc "$out.rc")" != 0 ] || return 0
  while read -r l; do [ -n "$l" ] && lints+=(-A "clippy::$l"); done < <(grep -oE 'index\.html#[a-z0-9_]+' "$out" | sed 's/.*#//' | sort -u)
  if [ "${#lints[@]}" -eq 0 ]; then echo "gates-beleg: clippy: Testziele nicht geprueft (red without a lint name to allow)"; return 0; fi
  (cd "$root" && cargo clippy --workspace --all-targets --all-features --locked --keep-going -- -D warnings -W clippy::pedantic "${lints[@]}") >"$po" 2>&1 || rc=$?
  echo "gates-beleg: clippy-gegenprobe rc=$rc with ${lints[*]} (test targets checked; first clippy stays red)"
  base="$(git -C "$root" merge-base HEAD origin/main 2>/dev/null || true)"
  if [ "$rc" = 0 ]; then echo "gates-beleg: clippy-gegenprobe: no finding beyond the foreign-toolchain lints"; return 0; fi
  changed=""; [ -z "$base" ] || changed="$(git -C "$root" diff --name-only "$base")"
  echo "gates-beleg: clippy-gegenprobe RED: Testziele/Dateien mit Befund (never 'toolchain'):"
  grep -oE -- '--> [^:]+' "$po" | sed 's/^--> //' | sort -u | while read -r f; do
    if printf '%s\n' "$changed" | grep -qFx -- "$f"; then echo "gates-beleg:   CHANGED file: $f"; else echo "gates-beleg:   $f"; fi
  done
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
