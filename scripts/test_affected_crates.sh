#!/usr/bin/env bash
# Pattern: none (test). Self test of scripts/affected_crates.py and of the narrowed TEST gate of scripts/gates.sh (CI cost on unreviewed/* branches).
# A fixture workspace  app -> mid -> base (+ an independent `lone`, and `devonly` that uses base only as a dev-dependency): the files of one crate give that crate plus every
# crate that depends on it (dev-dependencies and transitively), a global file or a file no crate owns gives FULL, errors give rc 2 (the gate then tests the whole workspace).
# gates.sh with FS_GATES_DIFF_BASE runs `cargo test -p ...` for the narrowed set and keeps clippy/fmt on --workspace; unset, nothing changes. Mutants must each fail a case.
#   bash scripts/test_affected_crates.sh      (needs bash, python3, git, cargo for `metadata`; no network)
set -uo pipefail
here="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
repo="$(cd "$here/.." && pwd)"
real_cargo="$(command -v cargo)"
tmp="$(mktemp -d "${TEST_TMPDIR:-${TMPDIR:-/tmp}}/affected.XXXXXX")"
trap 'rm -rf "$tmp"' EXIT
fails=0
ok() { echo "ok   $1"; }
bad() { echo "FAIL $1"; fails=$((fails + 1)); }

# crate DIR NAME [DEP[:dev]...]: a library crate with path dependencies on its siblings (crates/<dep>)
crate() {
  local d=$1 n=$2; shift 2
  mkdir -p "$d/src"; : >"$d/src/lib.rs"
  printf '[package]\nname = "%s"\nversion = "0.1.0"\nedition = "2021"\n\n[dependencies]\n' "$n" >"$d/Cargo.toml"
  local x; for x in "$@"; do [[ "$x" == *:dev ]] || printf '%s = { path = "../%s" }\n' "$x" "$x" >>"$d/Cargo.toml"; done
  printf '\n[dev-dependencies]\n' >>"$d/Cargo.toml"
  for x in "$@"; do [[ "$x" != *:dev ]] || printf '%s = { path = "../%s" }\n' "${x%:dev}" "${x%:dev}" >>"$d/Cargo.toml"; done
}
W="$tmp/ws"; mkdir -p "$W/crates" "$W/scripts" "$W/bin"
crate "$W/crates/base" base; crate "$W/crates/mid" mid base; crate "$W/crates/app" app mid; crate "$W/crates/lone" lone; crate "$W/crates/devonly" devonly base:dev
crate "$W/crates/ap" ap; crate "$W/crates/outer" outer; crate "$W/crates/outer/inner" inner   # `ap` starts like `app`; `inner` lies inside the directory of `outer`
printf '[workspace]\nmembers = ["crates/*", "crates/outer/inner"]\nresolver = "2"\n' >"$W/Cargo.toml"
(cd "$W" && "$real_cargo" generate-lockfile >/dev/null 2>&1) || { echo "FAIL fixture: cargo generate-lockfile"; exit 1; }
git -C "$W" init -q -b main
G=(git -C "$W" -c user.name=t -c user.email=t@t)
"${G[@]}" add -A; "${G[@]}" commit -q -m base; git -C "$W" config --local freesynergy.identity-test-base HEAD

AC="$repo/scripts/affected_crates.py"
# files_case NAME WANT FILE...  (files relative to the workspace)
files_case() {
  local name=$1 want=$2 s=${CRATES_SCRIPT:-$AC}; shift 2
  printf '%s\n' "$@" >"$tmp/files"
  local got; got=$(python3 -I "$s" "$W" --files "$tmp/files" 2>/dev/null)
  if [ "$got" = "$want" ]; then [ -n "${QUIET:-}" ] || ok "$name"; else
    if [ -n "${QUIET:-}" ]; then cases_fail=$((cases_fail + 1)); else bad "$name (got: $got, wanted: $want)"; fi
  fi
}
all_cases() {
  files_case "leaf crate: only itself" "PACKAGES app" crates/app/src/lib.rs
  files_case "a crate and everything that depends on it (transitively)" "PACKAGES app mid" crates/mid/src/lib.rs
  files_case "the bottom crate: all dependants, dev-dependencies included" "PACKAGES app base devonly mid" crates/base/src/lib.rs
  files_case "independent crate: only itself" "PACKAGES lone" crates/lone/Cargo.toml
  files_case "a dev-dependency user does not pull its dependency in" "PACKAGES devonly" crates/devonly/tests/x.rs
  files_case "two crates: union" "PACKAGES app lone" crates/app/src/lib.rs crates/lone/src/lib.rs
  files_case "a directory that only starts like a crate directory is no crate (FULL)" "FULL crates/ap-extra/x.rs belongs to no crate" crates/ap-extra/x.rs
  files_case "the crate named like the start of another" "PACKAGES ap" crates/ap/src/lib.rs
  files_case "a crate inside another crate's directory: the inner one (longest match)" "PACKAGES inner" crates/outer/inner/src/lib.rs
  files_case "the outer crate of that nest" "PACKAGES outer" crates/outer/src/lib.rs
  files_case "Cargo.lock is global" "FULL Cargo.lock affects every crate" Cargo.lock crates/app/src/lib.rs
  files_case "the workspace manifest is global" "FULL Cargo.toml affects every crate" Cargo.toml
  files_case "rust-toolchain.toml is global" "FULL rust-toolchain.toml affects every crate" rust-toolchain.toml
  files_case ".cargo/config.toml is global" "FULL .cargo/config.toml affects every crate" .cargo/config.toml
  files_case "a file no crate owns (scripts/) means FULL" "FULL scripts/x.sh belongs to no crate" scripts/x.sh crates/app/src/lib.rs
  files_case "a crate manifest is that crate (not global)" "PACKAGES app" crates/app/Cargo.toml
  files_case "no file at all: FULL" "FULL no changed file against the base (nothing to narrow)" ""
  files_case "every crate affected means FULL" "FULL every crate is affected" crates/base/src/lib.rs crates/lone/src/lib.rs crates/ap/src/lib.rs crates/outer/src/lib.rs crates/outer/inner/src/lib.rs
}
cases_fail=0
all_cases

# git mode: the diff against a base rev; an unknown rev is rc 2
printf 'x\n' >"$W/crates/mid/src/more.rs"; "${G[@]}" add -A; "${G[@]}" commit -q -m mid
got=$(python3 -I "$AC" "$W" HEAD~1 2>/dev/null); [ "$got" = "PACKAGES app mid" ] && ok "git mode: diff base...HEAD" || bad "git mode: got '$got'"
rc=0; python3 -I "$AC" "$W" no-such-rev >/dev/null 2>&1 || rc=$?; [ "$rc" = 2 ] && ok "an unknown base rev is rc 2 (the gate then tests everything)" || bad "unknown rev rc $rc"
printf 'x\n' >"$W/stray.txt"; "${G[@]}" add -A; "${G[@]}" commit -q -m stray
got=$(python3 -I "$AC" "$W" HEAD~2 2>/dev/null); [ "$got" = "FULL stray.txt belongs to no crate" ] && ok "git mode: a stray top-level file is FULL" || bad "git mode stray: got '$got'"
# a workspace in a sub directory of the repo: a change outside it is FULL
S="$tmp/mono"; mkdir -p "$S"; git -C "$S" init -q -b main; mkdir -p "$S/sub"; cp -r "$W/crates" "$W/Cargo.toml" "$W/Cargo.lock" "$S/sub/"
git -C "$S" add -A; git -C "$S" -c user.name=t -c user.email=t@t commit -q -m a
echo y >"$S/sub/crates/lone/src/a.rs"; echo z >"$S/other.txt"; git -C "$S" add -A; git -C "$S" -c user.name=t -c user.email=t@t commit -q -m b
got=$(python3 -I "$AC" "$S/sub" HEAD~1 2>/dev/null); [ "$got" = "FULL a changed file lies outside the workspace directory" ] && ok "sub directory workspace: a change outside it is FULL" || bad "mono outside: got '$got'"
git -C "$S" rm -q --cached other.txt; git -C "$S" -c user.name=t -c user.email=t@t commit -q -m c
got=$(python3 -I "$AC" "$S/sub" HEAD~2 2>/dev/null); [ "$got" = "PACKAGES lone" ] && ok "sub directory workspace: paths are made relative to it" || bad "mono: got '$got'"

# gates.sh: copy with stub cargo (metadata goes to the real one), CI=true; the cargo log shows the narrowed test and the full clippy
GK="$tmp/gk"; mkdir -p "$GK/scripts" "$GK/bin"
cp "$repo/scripts/gates.sh" "$repo/scripts/lib_gates_guard.sh" "$repo/scripts/lib_gates_list.sh" "$AC" "$repo/scripts/identity-check" "$GK/scripts/"
echo 1.0.0 >"$GK/RUST_VERSION"; printf '#!/usr/bin/env bash\nexit 0\n' >"$GK/scripts/gates-lock.sh"; cp "$GK/scripts/gates-lock.sh" "$GK/scripts/gates-files.sh"; : >"$GK/scripts/check_rust_rules.py"
printf '#!/usr/bin/env bash\nif [ "$1" = metadata ]; then unset RUSTUP_TOOLCHAIN; exec "%s" "$@"; fi\necho "cargo $*" >> "%s/cargo.log"\n' "$real_cargo" "$GK" >"$GK/bin/cargo"
printf '#!/usr/bin/env bash\nunset RUSTUP_TOOLCHAIN\nexec "%s" "$@"\n' "$(command -v rustc)" >"$GK/bin/rustc"; chmod +x "$GK/bin/cargo" "$GK/bin/rustc"
gates_run() { # <script> <diff base or empty>; the repo under test is $W (HEAD = the stray commit); output in $GK/out, cargo words in $GK/cargo.log
  : >"$GK/cargo.log"; rm -f "$W/stray.txt"; "${G[@]}" commit -q -am unstray >/dev/null 2>&1
  PATH="$GK/bin:$PATH" CI=true TMPDIR="$tmp" FS_GATES_FEHLERLISTE_DIR="$tmp" FS_GATES_DIFF_BASE="$2" bash "$1" "$W" >"$GK/out" 2>&1
}
gate_cases() { # <script>; counts failures in cases_fail when QUIET
  local s=$1 rc=0 why=""
  gates_run "$s" "HEAD~3" || rc=$?
  cp "$GK/out" "$GK/out1"
  # HEAD~3 is the base commit's child (mid change, stray, unstray): the diff is crates/mid + stray.txt (net: stray gone) -> mid
  grep -qx 'cargo test -p app -p mid --all-features --locked --no-fail-fast' "$GK/cargo.log" || why="$why; narrowed test line missing"
  ! grep -q -- 'cargo test --workspace' "$GK/cargo.log" || why="$why; whole-workspace test ran"
  grep -q 'cargo clippy --workspace --all-targets --all-features --locked --keep-going' "$GK/cargo.log" || why="$why; clippy not on the workspace"
  grep -qx 'cargo fmt --check' "$GK/cargo.log" || why="$why; fmt missing"
  grep -q 'test NARROWED' "$GK/out" || why="$why; no NARROWED line"
  [ "$rc" = 0 ] || why="$why; rc $rc"
  gates_run "$s" "" || rc=$?
  grep -qx 'cargo test --workspace --all-features --locked --no-fail-fast' "$GK/cargo.log" || why="$why; unset: whole workspace test missing"
  ! grep -q 'NARROWED' "$GK/out" || why="$why; unset: narrowed"
  gates_run "$s" "no-such-rev" || true
  grep -qx 'cargo test --workspace --all-features --locked --no-fail-fast' "$GK/cargo.log" || why="$why; bad rev: no fallback to the whole workspace"
  grep -q 'test runs the WHOLE workspace' "$GK/out" || why="$why; bad rev: fallback not said"
  if [ -n "$why" ]; then if [ -n "${QUIET:-}" ]; then cases_fail=$((cases_fail + 1)); else bad "gates.sh narrowed test: ${why#; }"; sed 's/^/     /' "$GK/out1"; fi
  elif [ -z "${QUIET:-}" ]; then ok "gates.sh: FS_GATES_DIFF_BASE narrows ONLY the test gate (clippy, fmt stay full); unset or a bad rev = whole workspace, said"; fi
}
gate_cases "$GK/scripts/gates.sh"

# the wiring: only unreviewed/* branches get a diff base (never main, never a tag), `full: true` switches it off, the gates action hands it to gates.sh
python3 -I - "$repo/.github/workflows/check.yml" "$repo/actions/gates/action.yml" <<'PY' && ok "check.yml: diff-base only on unreviewed/* and without full; the gates action sets FS_GATES_DIFF_BASE" || bad "check.yml / actions/gates wiring"
import sys, yaml
chk = yaml.safe_load(open(sys.argv[1], encoding="utf-8"))
trig = chk.get("on") or chk[True]
assert trig["workflow_call"]["inputs"]["full"] == {"type": "boolean", "default": False, "description": trig["workflow_call"]["inputs"]["full"]["description"]}
steps = chk["jobs"]["check"]["steps"]
g = [s for s in steps if s.get("id") == "gates-fmt-clippy"][0]
expr = g["with"]["diff-base"]
for need in ("!inputs.full", "startsWith(github.ref, 'refs/heads/unreviewed/')", "'origin/main'"):
    assert need in expr, need
assert expr.rstrip().endswith("|| '' }}"), expr   # everything else: empty = the whole workspace
f = [s for s in steps if s.get("name", "").startswith("Fetch main")][0]
assert "!inputs.full" in f["if"] and "refs/heads/unreviewed/" in f["if"]
assert steps.index(f) < steps.index(g)
act = yaml.safe_load(open(sys.argv[2], encoding="utf-8"))
assert act["inputs"]["diff-base"]["default"] == ""
assert act["runs"]["steps"][0]["env"]["FS_GATES_DIFF_BASE"] == "${{ inputs.diff-base }}"
PY

# mutants: each must fail a case
mutant() { # <name> <file in $GK/scripts> <sed -E expression> <kind: gates|crates>
  local name=$1 f=$2 expr=$3 kind=$4
  sed -E "$expr" "$repo/scripts/$f" >"$tmp/mut"
  if cmp -s "$tmp/mut" "$repo/scripts/$f"; then bad "mutant '$name' changed nothing (NOT APPLIED)"; return; fi
  cases_fail=0
  if [ "$kind" = crates ]; then CRATES_SCRIPT="$tmp/mut" QUIET=1 all_cases; else cp "$tmp/mut" "$GK/scripts/gates-m.sh"; QUIET=1 gate_cases "$GK/scripts/gates-m.sh"; fi
  if [ "$cases_fail" -gt 0 ]; then ok "mutant '$name' is red ($cases_fail cases)"; else bad "mutant '$name' SURVIVES"; fi
}
mutant "dependants not followed" affected_crates.py 's/if d\["pkg"\] in pkgs:/if False:/' crates
mutant "shortest instead of longest directory match" affected_crates.py 's/hit.add\(max\(owners\)\[1\]\)/hit.add(min(owners)[1])/' crates
mutant "Cargo.lock not global" affected_crates.py 's/"Cargo.lock", //' crates
mutant "no-owner file ignored" affected_crates.py 's/if not owners:/if False:/' crates
mutant "prefix match without the slash" affected_crates.py 's/d \+ "\/"/d/' crates
mutant "every crate affected still narrows" affected_crates.py 's/if len\(seen\) == len\(pkgs\):/if False:/' crates
mutant "gates: narrowing also for clippy" gates.sh 's/clippy --workspace --all-targets/clippy -p mid --all-targets/' gates
mutant "gates: narrowing ignored" gates.sh 's/^( *)testsel=\(\)$/\1testsel=(--workspace)/' gates
mutant "gates: fallback not said" gates.sh 's/test runs the WHOLE workspace/test runs it/' gates
mutant "gates: -p dropped from the list" gates.sh 's/testsel\+=\(-p "\$sel_name"\)/testsel+=("$sel_name")/' gates
mutant "gates: narrowed line not said" gates.sh 's/test NARROWED/test went on/' gates
# NOT testable (equivalent): dropping the `$sel_rc = 0` condition or the `-n FS_GATES_DIFF_BASE` check changes nothing observable (an error prints nothing on stdout, an empty base is an
# error of affected_crates.py): both end in the whole-workspace test.

echo
if [ "$fails" = 0 ]; then echo "test_affected_crates: all ok"; else echo "test_affected_crates: $fails failures"; exit 1; fi
