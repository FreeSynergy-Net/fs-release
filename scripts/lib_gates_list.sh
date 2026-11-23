#!/usr/bin/env bash
# Helpers of scripts/gates.sh for the Fehlerliste ([GATES-SAMMELLISTE]); sourced, own file so the rc rule has a test (scripts/test_gates_sammelliste.sh).

# gates_read_rc <file>: the rc a step wrote into <file>. A file that is missing, empty or no number (disk full, killed step) is RED (1), never green.
gates_read_rc() {
  local v=""
  [ -r "$1" ] && v="$(cat "$1")"
  if [[ "$v" =~ ^[0-9]+$ ]]; then echo "$v"; else echo 1; fi
}

# --- [GATE-BELEG]: the proof that goes into every worker report; sourced by gates.sh. Local gates = CI: one source (RUST_VERSION, this script), and a run that is NOT CI-equal says so, never silently green.
# gates_ci_diff <want> <have> <root>: one line per reason why this run is not CI-equal (empty = CI-equal). Toolchain: rustc must be exactly the pinned one (without rustup the pin has no effect).
gates_ci_diff() {
  case "$2" in "rustc $1 "*) ;; *) echo "toolchain: local '$2' != CI $1 (rustup missing: RUSTUP_TOOLCHAIN has no effect); lints of the newer toolchain are not CI lints" ;; esac
  command -v shellcheck >/dev/null 2>&1 || echo "NICHT GEPRUEFT: shellcheck fehlt (CI job lint)"
  if [ -d "$3/.github/workflows" ] && ! command -v actionlint >/dev/null 2>&1; then echo "NICHT GEPRUEFT: actionlint fehlt (CI job lint)"; fi
  if grep -qE '^rust-version[[:space:]]*=' "$3/Cargo.toml" 2>/dev/null && ! command -v rustup >/dev/null 2>&1; then echo "NICHT GEPRUEFT: MSRV (rust-version in Cargo.toml; no rustup, CI msrv job decides)"; fi
}
# gates_lint_shell / gates_lint_actions <root>: the lint job of CI (shellcheck -x -S warning over the tracked shell files, actionlint over the workflows); only called when the tool exists.
gates_lint_shell() { cd "$1" && git ls-files -z '*.sh' | xargs -0 -r shellcheck -x -S warning; }
gates_lint_actions() { cd "$1" && actionlint; }
# gates_beleg <root> <want> <have> <like-ci> <step rcs...>: the Beleg block (toolchain, flags, every step with its rc, `CI-gleich: ja/nein, reason`); a not CI-equal run adds the rule: the branch CI at the head is the proof.
gates_beleg() {
  local root="$1" want="$2" have="$3" like="$4" diff base
  shift 4
  diff="$(gates_ci_diff "$want" "$have" "$root")"
  echo "gates-beleg: rustc=${have}; ci-toolchain=${want}; mode=$([ -n "$like" ] && echo like-ci || echo standard); clippy=--workspace --all-targets --all-features --locked --keep-going -- -D warnings -W clippy::pedantic; steps: $*"
  if [ -z "$diff" ]; then echo "gates-beleg: CI-gleich: ja"; return 0; fi
  echo "gates-beleg: CI-gleich: NEIN"
  printf 'gates-beleg: grund: %s\n' "$diff"
  base="$(git -C "$root" merge-base HEAD origin/main 2>/dev/null || true)"
  if [ -n "$base" ] && [ -r "${finding_work:-/nonexistent}/clippy.out" ]; then
    { grep -oE -- '--> [^:]+' "$finding_work/clippy.out" | sed 's/^--> //' | sort -u | grep -Fxf <(git -C "$root" diff --name-only "$base") || true; } | sed 's/^/gates-beleg: clippy finding in a CHANGED file (fix it, never "toolchain"): /'
  fi
  echo "gates-beleg: green here is NOT proof of a green CI: the branch CI at the head is the proof; the report must name what did not run here"
}
