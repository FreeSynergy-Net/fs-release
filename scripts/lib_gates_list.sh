#!/usr/bin/env bash
# Helpers of scripts/gates.sh for the Fehlerliste ([GATES-SAMMELLISTE]); sourced, own file so the rc rule has a test (scripts/test_gates_sammelliste.sh).

# gates_read_rc <file>: the rc a step wrote into <file>. A file that is missing, empty or no number (disk full, killed step) is RED (1), never green.
gates_read_rc() {
  local v=""
  [ -r "$1" ] && v="$(cat "$1")"
  if [[ "$v" =~ ^[0-9]+$ ]]; then echo "$v"; else echo 1; fi
}

# --- [GATE-BELEG]: the report block of every worker report; sourced by gates.sh. A local run is NEVER "CI-equal": it prints `CI-gleich: NEIN` plus what ran and what did NOT run; the proof is the branch CI at the exact head (incl. windows).
ACTIONLINT_PIN=1.7.12 # the version lint.yml installs
# gates_tool_version <tool>: first line of its version output, or "missing".
gates_tool_version() { command -v "$1" >/dev/null 2>&1 && "$1" "${2:---version}" 2>&1 | sed -n 1,2p | tr '\n' ' ' || echo missing; }
# gates_has_windows_cfg <root>: the repo has Rust code behind a Windows cfg (CI job windows checks it, a Linux run never sees it: clippy "unused self", PKG-2).
gates_has_windows_cfg() { git -C "$1" grep -qE 'cfg\(.*windows|target_os *= *"windows"' -- '*.rs' 2>/dev/null; }
# gates_windows_target: the windows target installed in the rustc sysroot (check/clippy need no linker, only the target), empty = none.
gates_windows_target() {
  local t root; root="$(rustc --print sysroot 2>/dev/null)"
  for t in x86_64-pc-windows-msvc x86_64-pc-windows-gnu; do if [ -d "$root/lib/rustlib/$t" ]; then echo "$t"; return 0; fi; done
}
# gates_ci_diff <want> <have> <root> <like>: one line per thing that did NOT run here / differs from CI (never empty: the Windows job never runs locally).
gates_ci_diff() {
  case "$2" in "rustc $1 "*) ;; *) echo "toolchain: Toolchain $1 nicht verfuegbar -> NICHT CI-gleich (local '$2'; rustup missing: RUSTUP_TOOLCHAIN has no effect); lints of the other toolchain are not CI lints (clippy ran with the local one PLUS the counter-probe)" ;; esac
  [ -n "$4" ] || echo "mode standard: the lint job (shellcheck, actionlint, self tests) and the MSRV check (use --like-ci)"
  [ -z "${FS_GATES_SKIP_TEST:-}" ] || echo "test $FS_GATES_SKIP_TEST SKIPPED (FS_GATES_SKIP_TEST)"
  [ -z "${FS_GATES_DIFF_BASE:-}" ] || echo "test scope narrowed (FS_GATES_DIFF_BASE=$FS_GATES_DIFF_BASE): not the whole workspace"
  git -C "$3" merge-base HEAD origin/main >/dev/null 2>&1 || echo "origin/main fehlt (NICHT GEPRUEFT: changed files unknown, clippy findings cannot be assigned to the branch)"
  if [ -n "$(gates_windows_target)" ]; then echo "Windows job (native fmt/clippy/TEST on Windows: locally at most clippy --target $(gates_windows_target); cargo test, cfg!(windows), unix-only cfg and target dependencies are not run)"
  else echo "Windows job (native fmt/clippy/TEST on Windows; no windows target installed: cfg(windows) code, cfg!(windows), unix-only cfg, target dependencies unchecked): Zweig-CI Pflicht (windows)"; fi
  command -v shellcheck >/dev/null 2>&1 || echo "NICHT GEPRUEFT: shellcheck fehlt (CI job lint)"
  if [ -d "$3/.github/workflows" ]; then
    if ! command -v actionlint >/dev/null 2>&1; then echo "NICHT GEPRUEFT: actionlint fehlt (CI job lint)"
    elif ! gates_tool_version actionlint -version | grep -q "$ACTIONLINT_PIN"; then echo "actionlint version differs from the CI pin $ACTIONLINT_PIN"; fi
  fi
  if grep -qE '^rust-version[[:space:]]*=' "$3/Cargo.toml" 2>/dev/null && ! command -v rustup >/dev/null 2>&1; then echo "MSRV (rust-version in Cargo.toml; no rustup; CI takes the msrv INPUT of the caller, which may differ from Cargo.toml)"; fi
}
# gates_lint_shell / gates_lint_actions <root>: the lint job of CI: the shellcheck line of .github/workflows/lint.yml itself (same files, same flags), else all tracked *.sh; actionlint over the workflows.
gates_lint_shell() {
  local cmd; cd "$1" || return 1
  cmd="$(sed -n 's/^ *run: \(shellcheck .*\)$/\1/p' .github/workflows/lint.yml 2>/dev/null || true)"
  if [ -n "$cmd" ]; then bash -c "$cmd"; else git ls-files -z '*.sh' | xargs -0 -r shellcheck -x -S warning; fi
}
gates_lint_actions() { cd "$1" && actionlint; }
# gates_lint_selftests <root>: every self test / compile command of lint.yml: single-line `run:` AND `run: |` blocks whose lines are all of the allowed forms (`bash scripts/test_*.sh`, `scripts/test-packaging.sh`,
# `python3 -I scripts/...`, `python3 -I -m py_compile ...`), run one by one with their rc; any other block/line (curl, apt-get, grep, collecting step ...) is listed as `selftest-skipped: <step name> (mehrzeilig)` and
# ends in the Beleg as "lokal NICHT gelaufen", never silently skipped. The step is red when one failed. Byte code goes to the scratch folder, never into the tree.
# shellcheck disable=SC2154 # finding_work is set by gates.sh (sourced)
gates_lint_selftests() {
  local c kind bad=0 n=0 rc; cd "$1" || return 1
  while IFS=$'\t' read -r kind c; do
    if [ "$kind" = SKIP ]; then echo "selftest-skipped: $c"; continue; fi
    n=$((n + 1)); rc=0
    PYTHONPYCACHEPREFIX="$finding_work/pyc" bash -c "$c" >"$finding_work/st.out" 2>&1 || rc=$?
    if [ "$rc" != 0 ]; then bad=1; cat "$finding_work/st.out"; fi
    echo "selftest rc=$rc: $c" | cut -c1-150
  done < <(awk '
    function ind(s) { match(s, /^ */); return RLENGTH }
    function ok(l) { return l ~ /^ *(bash scripts\/test_[A-Za-z0-9_]+\.sh|scripts\/test-packaging\.sh|python3 -I scripts\/[A-Za-z0-9_.\/-]+( [A-Za-z0-9_.\/-]+)*|python3 -I -m py_compile [A-Za-z0-9_.\/ -]+)$/ }
    function flush(   i, all) {
      if (!blk) return
      all = (n > 0); for (i = 1; i <= n; i++) if (!ok(L[i])) all = 0
      if (all) { for (i = 1; i <= n; i++) { sub(/^ */, "", L[i]); print "RUN\t" L[i] } } else print "SKIP\t" name " (mehrzeilig)"
      blk = 0
    }
    blk { if ($0 ~ /^[ ]*$/ || ind($0) > bi) { if ($0 !~ /^[ ]*(#|$)/) L[++n] = $0; next } flush() }
    /^ *- uses:/ { name = "(uses)" }
    /^ *- name:/ { name = $0; sub(/^ *- name: */, "", name) }
    /^ *run: *[|>]/ { blk = 1; bi = ind($0); n = 0; next }
    /^ *run: / { c = $0; sub(/^ *run: */, "", c); if (c ~ /^shellcheck /) next; if (ok(c)) print "RUN\t" c; else print "SKIP\t" name; next }
    END { flush() }' .github/workflows/lint.yml 2>/dev/null)
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
# gates_beleg <root> <want> <have> <like-ci> <step rcs...>: the Beleg block: head + time (a Beleg of an old head cannot pass), tool versions, flags, `CI-gleich: NEIN` ALWAYS (no "ja" exists), what ran with its rc, what did NOT run.
gates_beleg() {
  local root="$1" want="$2" have="$3" like="$4" diff l
  shift 4
  diff="$(gates_ci_diff "$want" "$have" "$root" "$like")"
  echo "gates-beleg: head=$(git -C "$root" rev-parse HEAD 2>/dev/null || echo unknown); time=$(date -u +%Y-%m-%dT%H:%M:%SZ); mode=$([ -n "$like" ] && echo like-ci || echo standard)"
  echo "gates-beleg: rustc=${have}; ci-toolchain=${want}; shellcheck=$(gates_tool_version shellcheck); actionlint=$(gates_tool_version actionlint -version); clippy=--workspace --all-targets --all-features --locked --keep-going -- -D warnings -W clippy::pedantic"
  echo "gates-beleg: CI-gleich: NEIN - Zweig-CI am genauen Kopf (inkl. windows) ist der Beleg"
  echo "gates-beleg: lokal gelaufen: $*"
  while read -r l; do [ -z "$l" ] || echo "gates-beleg: lokal NICHT gelaufen: $l"; done <<<"$diff"
  # [NICHT-GELAUFEN-NIE-GRUEN] a run that was SKIPPED (profile not defined by the caller, nothing changed) says `gates-skipped: <what>` in its step output or in land.out: never "ran, rc=0" in the Beleg
  while IFS= read -r l; do echo "gates-beleg: lokal NICHT gelaufen: ${l#gates-skipped: }"; done < <(grep -h '^gates-skipped: ' "${finding_work:-/nonexistent}"/*.out 2>/dev/null || true)
  if [ -r "${finding_work:-/nonexistent}/selftests.out" ]; then
    while read -r l; do echo "gates-beleg: lokal NICHT gelaufen: ${l#selftest-skipped: }"; done < <(grep '^selftest-skipped: ' "$finding_work/selftests.out")
  fi
  case "$diff" in *toolchain:*) gates_clippy_probe "$root" ;; esac
  echo "gates-beleg: green here is NOT proof of a green CI; the report names what did not run here and the CI run id of the branch CI at the head"
}

# gates_land_steps: the extra test runs of the LANDING head (push on main, set by check.yml): after quick and security, the profiles of FS_GATES_LAND_PROFILES (picture platform)
# once each, as steps of their own (`test-<profile>`: own rc, own `gate ...: rc=` line, red = gate red, --no-fail-fast, no retries), only for the crates changed since FS_GATES_LAND_BASE (the head of main BEFORE
# the push = github.event.before) and their dependants (scripts/affected_crates.py, the same selection as on branches). A profile the caller does not define is skipped with a line (test-gate.sh --only-profile).
# No usable base (empty, all zeros = new branch, not 40 hex digits, git error, global file, every crate affected) = the whole workspace, said in the log; no changed file at all = skipped with a line.
# `time` stays out on purpose (minutes): tag / nightly `release` only.
gates_land_steps() {
  local d profiles="${FS_GATES_LAND_PROFILES:-}" base="${FS_GATES_LAND_BASE:-}" p out rc=0 sel=(--workspace) n
  [ -n "$profiles" ] || return 0
  d="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
  for p in $profiles; do
    [[ "$p" =~ ^[a-z][a-z0-9_-]{0,31}$ ]] || { echo "gates FAILED: FS_GATES_LAND_PROFILES: bad profile name '${p:0:40}'" >&2; exit 2; }
  done
  if [[ "$base" =~ ^[0-9a-f]{40}$ ]] && ! [[ "$base" =~ ^0+$ ]]; then
    out="$(python3 -I "$d/affected_crates.py" "$root" "$base" --empty-none)" || rc=$?
    if [ "$rc" = 0 ] && [[ "$out" == "PACKAGES "* ]]; then
      read -ra n <<<"${out#PACKAGES }"; sel=()
      for p in "${n[@]}"; do sel+=(-p "$p"); done
      echo "gates: landing-head test runs ($profiles) NARROWED to the crates changed since $base and their dependants: ${n[*]}"
    elif [ "$rc" = 0 ] && [[ "$out" == "NONE "* ]]; then
      echo "gates: landing-head test runs ($profiles) SKIPPED: ${out#NONE }"
      for p in $profiles; do echo "gates-skipped: test-$p (no changed crate since $base)" | tee -a "$finding_work/land.out"; done; return 0
    else
      echo "gates: landing-head test runs ($profiles) on the WHOLE workspace (${out:-affected_crates.py rc=$rc})"
    fi
  else
    echo "gates: landing-head test runs ($profiles) on the WHOLE workspace (no usable base commit: '${base:0:12}')"
  fi
  for p in $profiles; do
    # a profile the caller does not define: NO step (a step would print `rc=0` and the Beleg would say "ran"); the gate stays green, the Beleg says `lokal NICHT gelaufen`
    if ! grep -qE "^[[:space:]]*\[profile\.${p}(\.[A-Za-z0-9_-]+)?\][[:space:]]*(#.*)?\$" .config/nextest.toml 2>/dev/null; then
      echo "gate test-$p: SKIPPED (profile $p not defined in .config/nextest.toml)"
      echo "gates-skipped: test-$p (profile $p not defined in .config/nextest.toml)" | tee -a "$finding_work/land.out"
    else
      step "test-$p" error bash "$d/test-gate.sh" --only-profile "$p" "${sel[@]}" --all-features --locked
    fi
  done
}

# gates_rust_files <root>: the files that make a repo a Rust repo: any Cargo.toml (any case), any Cargo.lock, any *.rs (build.rs, lib.rs, tests/ ...), tracked or untracked-not-ignored; one name per line.
# The list is read with `-z` (git quotes names with umlauts or tabs otherwise); grep runs as `LC_ALL=C grep -a` (a name that is not valid UTF-8 makes GNU grep call the list binary under a UTF-8 locale: empty output, rc 0).
# A symlink to a directory that lies outside the repo hides its content from git: listed as `symlink-outside: <path>`. rc 0 = listed (output may be empty), rc 2 = git, grep, find or mktemp FAILED:
# never "no files" (only grep rc 1 = no match is fine). pipefail is set INSIDE (a subshell), the function does not rely on the caller.
gates_rust_files() {
  local t rc=0 base="${TMPDIR:-/tmp}" l r
  [ -d "$base" ] || base=/tmp
  t="$(mktemp "$base/gates-rs.XXXXXX")" || return 2
  git -C "$1" ls-files -z -co --exclude-standard >"$t" || rc=2
  if [ "$rc" = 0 ]; then
    ( set -o pipefail; { LC_ALL=C grep -zaiE '(^|/)cargo\.(toml|lock)$|\.rs$' "$t" || [ $? -eq 1 ]; } | tr '\0' '\n' ) || rc=2
  fi
  if [ "$rc" = 0 ]; then
    r="$(cd "$1" && pwd -P)" || rc=2
    while IFS= read -r -d '' l; do
      case "$(realpath -- "$1/$l" 2>/dev/null || echo "$r/missing")/" in "$r"/*) ;; *) printf 'symlink-outside: %s\n' "$l" ;; esac
    done < <(cd "$1" && find . -path ./.git -prune -o -type l -xtype d -print0) || rc=2
  fi
  rm -f "$t"
  return "$rc"
}
# gates_data_mode_check <root>: step `daten-modus` of gates.sh, run when there is a scaffold.toml but no Cargo.toml in the root. Green ONLY for a pure data repo (NO Cargo.toml/Cargo.lock/*.rs anywhere in the
# file list): it then says out loud which gates do not run (`gates-skipped:` lines are copied into the Beleg as "lokal NICHT gelaufen"). A Rust repo that lost or hid its manifest is red.
gates_data_mode_check() {
  local root="$1" what found
  found="$(gates_rust_files "$root")" || { echo "gates FAILED: the file list of $root could not be read (git ls-files), so the repo kind is unknown" >&2; return 1; }
  if [ -z "$found" ]; then
    echo "gates: Cargo-Gates entfallen (kind data: scaffold.toml, kein Cargo.toml, kein Cargo.lock, keine .rs-Datei)"
    for what in fmt clippy test doctest licence msrv clippy-windows; do echo "gates-skipped: $what (Cargo-Gate, kind data: kein Cargo.toml)"; done
    echo "gates: im Daten-Modus laufen: identity, rust-rules, lock, files (und mit --like-ci: shellcheck, actionlint, selftests)"
    return 0
  fi
  echo "gates FAILED: no Cargo.toml in the root, but Rust files exist: not a pure data repo (a Rust repo must not lose its Cargo gates by losing or hiding the manifest): ${found%%$'\n'*}" >&2
  return 1
}
