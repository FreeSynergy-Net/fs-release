#!/usr/bin/env bash
# Pattern: none (test).
# Self test of the data mode of scripts/gates.sh (workflow workflow-pflege + bausatz-neu, order CI data mode): a pure data repo (scaffold.toml, NO Cargo.toml) runs identity, rust-rules, lock and files,
# the Cargo gates (fmt, clippy, test, doctest, licence) do NOT run, and the output says so ("Cargo-Gates entfallen (kind data)", `gates-beleg: lokal NICHT gelaufen`): never silently green. A STUB cargo
# logs every call: in data mode it is never called. Regression: a repo WITH Cargo.toml runs all Cargo gates as before. A scaffold.toml repo that has Cargo.lock/crates/src but lost its Cargo.toml is red.
# Mutants of gates.sh must each turn a case red.
#   bash scripts/test_gates_data_mode.sh
set -uo pipefail
here="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
mkdir -p "$here/../.test-tmp"; tmp="$(TMPDIR="${TEST_TMPDIR:-$here/../.test-tmp}" mktemp -d)"  # never /tmp (RAM, rule [PLATTE])
trap '[ -n "${KEEP:-}" ] || { rm -rf "$tmp"; rmdir "$here/../.test-tmp" 2>/dev/null; }' EXIT
fails=0
ok() { echo "ok   $1"; }
bad() { echo "FAIL $1"; fails=$((fails + 1)); }
chk() { local name="$1"; shift; if "$@"; then ok "$name"; else bad "$name"; fi; }
export GIT_AUTHOR_NAME=t GIT_AUTHOR_EMAIL=t@t GIT_COMMITTER_NAME=t GIT_COMMITTER_EMAIL=t@t
unset CI CARGO_TERM_COLOR FS_GATES_SKIP_TEST
export TMPDIR_GATES="$tmp" PYTHONDONTWRITEBYTECODE=1
mkdir -p "$tmp/bin"
printf '#!/usr/bin/env bash\necho "cargo $*" >>"$STUBLOG"\n[ "$1" = nextest ] && [ "$2" = --version ] && echo "cargo-nextest 0.9.151 (stub)"\nexit 0\n' >"$tmp/bin/cargo"
printf '#!/usr/bin/env bash\necho rustc-stub\n' >"$tmp/bin/rustc"; chmod +x "$tmp/bin/cargo" "$tmp/bin/rustc"
export STUBLOG="$tmp/cargo.log"
# mkrepo <dir> <gates.sh to copy or empty>: a repo with its own copy of the scripts (no fs-release tag is pinned), identity base = HEAD
mkrepo() {
  rm -rf "$1"; mkdir -p "$1/configs"; cp -r "$here" "$1/scripts"; cp "$here/../configs/nextest.toml" "$1/configs/"; rm -rf "$1/scripts/__pycache__" "$1/scripts/typing-test"; : # the helper crate of fs-release is not part of a data repo
  printf '#!/usr/bin/env bash\nexit 0\n' >"$1/scripts/licence-gate.sh"; echo 1.0.0 >"$1/RUST_VERSION"
  [ -z "${2:-}" ] || cp "$2" "$1/scripts/gates.sh"
  git init -q -b main "$1"; echo "__pycache__/" >"$1/.gitignore"
}
commit() { git -C "$1" add -A; git -C "$1" commit -qm one; git -C "$1" config --local freesynergy.identity-test-base HEAD; }
datarepo() { mkrepo "$1" "${2:-}"; printf 'schema = 2\n\n[program]\nname = "leer"\nversion = "0.0.1"\n' >"$1/scaffold.toml"; commit "$1"; }
run() { : >"$STUBLOG"; env PATH="$tmp/bin:$PATH" CI=true FS_GATES_FEHLERLISTE_DIR="$tmp/lists" bash "$1/scripts/gates.sh" "$1" >"$tmp/o.txt" 2>&1; }

# 1. a pure data repo: green, says so, cargo never called
d="$tmp/data"; datarepo "$d"; run "$d"; rc=$?
chk "data repo: gates rc 0" [ "$rc" -eq 0 ]
chk "data repo: says 'Cargo-Gates entfallen (kind data'" grep -q 'Cargo-Gates entfallen (kind data' "$tmp/o.txt"
chk "data repo: identity, daten-modus, rust-rules, lock, files ran with rc=0" bash -c "for g in identity daten-modus rust-rules lock files; do grep -q \"^gate \$g: rc=0\" '$tmp/o.txt' || exit 1; done"
chk "data repo: no Cargo gate has an rc line (fmt clippy test doctest licence)" bash -c "! grep -qE '^gate (fmt|clippy|test|doctest|licence): ' '$tmp/o.txt'"
chk "data repo: the Beleg lists fmt, clippy, test, licence as NICHT gelaufen" bash -c "for g in fmt clippy test licence; do grep -q \"gates-beleg: lokal NICHT gelaufen: \$g (Cargo-Gate, kind data\" '$tmp/o.txt' || exit 1; done"
chk "data repo: cargo was never called" bash -c "[ ! -s '$STUBLOG' ]"
chk "data repo: the tree is unchanged" bash -c "[ -z \"\$(git -C '$d' status --porcelain)\" ]"

# 2. regression: a repo WITH Cargo.toml runs every Cargo gate as before, no data mode
c="$tmp/cargo"; mkrepo "$c"; printf '[package]\nname = "x"\nversion = "0.1.0"\n' >"$c/Cargo.toml"; printf 'schema = 2\n' >"$c/scaffold.toml"; commit "$c"; run "$c"; rc=$?; cp "$tmp/o.txt" "$tmp/cargo.out"
chk "cargo repo (even with a scaffold.toml): gates rc 0" [ "$rc" -eq 0 ]
chk "cargo repo: fmt, clippy, test, doctest, licence ran" bash -c "for g in fmt clippy test doctest licence; do grep -q \"^gate \$g: rc=0\" '$tmp/o.txt' || exit 1; done"
chk "cargo repo: no daten-modus step, no 'entfallen' line" bash -c "! grep -qE 'daten-modus|Cargo-Gates entfallen' '$tmp/o.txt'"
chk "cargo repo: cargo fmt, clippy were called" bash -c "grep -q 'cargo fmt' '$STUBLOG' && grep -q 'cargo clippy' '$STUBLOG'"

# 3. scaffold.toml + Cargo.lock but no Cargo.toml (a Rust repo that lost its manifest): red, the Cargo gates run (and would fail for real)
l="$tmp/lost"; mkrepo "$l"; printf 'schema = 2\n' >"$l/scaffold.toml"; : >"$l/Cargo.lock"; commit "$l"; run "$l"; rc=$?
chk "lost manifest: gates rc 1" [ "$rc" -eq 1 ]
chk "lost manifest: daten-modus is red and says why" bash -c "grep -q '^gate daten-modus: rc=1' '$tmp/o.txt' && grep -q 'not a pure data repo' '$tmp/o.txt'"
chk "lost manifest: the Cargo gates were not skipped" grep -q '^gate fmt: ' "$tmp/o.txt"
# 3b. crates/ without Cargo.toml
rm "$l/Cargo.lock"; mkdir -p "$l/crates/a"; echo x >"$l/crates/a/lib.rs"; commit "$l"; run "$l"; rc=$?
chk "lost manifest (crates/): gates rc 1" [ "$rc" -eq 1 ]

# 3c. ways around the data mode (review M1): a scaffold.toml repo that hides Rust code or a manifest must be red, the Cargo gates must run
hide() { # <name> <file> [<file>...]: scaffold.toml + the files, committed (untracked case below)
  local name="$1" f; shift
  datarepo "$tmp/h-$name"; for f in "$@"; do mkdir -p "$tmp/h-$name/$(dirname "$f")"; echo "x" >"$tmp/h-$name/$f"; done; commit "$tmp/h-$name"
  run "$tmp/h-$name"; rc=$?
  chk "hidden Rust ($name): gates rc 1, daten-modus red, fmt ran, no 'entfallen'" bash -c "[ $rc -eq 1 ] && grep -q '^gate daten-modus: rc=1' '$tmp/o.txt' && grep -q '^gate fmt: ' '$tmp/o.txt' && ! grep -q 'Cargo-Gates entfallen' '$tmp/o.txt'"
}
hide sub-crate sub/Cargo.toml sub/src/main.rs
hide lib-rs-in-root lib.rs
hide build-rs-and-tests build.rs tests/a.rs
hide lowercase-manifest cargo.toml
hide lock-in-subfolder sub/Cargo.lock
hide uppercase-ext sub/X.rs
hide umlaut-dir "ü/Cargo.toml" "ü/src/main.rs"
hide umlaut-lib "ä/lib.rs"
hide tab-in-name "$(printf 'a\tb.rs')"
datarepo "$tmp/h-untracked"; echo x >"$tmp/h-untracked/lib.rs"; run "$tmp/h-untracked"; rc=$?
chk "hidden Rust (untracked, not ignored lib.rs): rc 1" bash -c "[ $rc -eq 1 ] && grep -q '^gate daten-modus: rc=1' '$tmp/o.txt'"
datarepo "$tmp/h-doc"; mkdir -p "$tmp/h-doc/docs"; echo "fn main() {}" >"$tmp/h-doc/docs/readme.md"; commit "$tmp/h-doc"; run "$tmp/h-doc"; rc=$?
chk "a .md or other text with Rust inside is no Rust file: data mode stays green" [ "$rc" -eq 0 ]

# 4. neither scaffold.toml nor Cargo.toml: unchanged behaviour, the Cargo gates run (red by themselves with the real cargo)
n="$tmp/neither"; mkrepo "$n"; echo x >"$n/a.txt"; commit "$n"; run "$n"
chk "neither file: no data mode (fmt ran)" bash -c "grep -q '^gate fmt: ' '$tmp/o.txt' && ! grep -q 'Cargo-Gates entfallen' '$tmp/o.txt'"

# 5. a data repo with a broken workflow yaml: the files gate stays red (the other gates still count)
y="$tmp/yaml"; datarepo "$y"; mkdir -p "$y/.github/workflows"; printf 'a: [unclosed\n' >"$y/.github/workflows/x.yml"; commit "$y"; run "$y"; rc=$?
if python3 -c 'import yaml' 2>/dev/null; then chk "data repo with broken yaml: red in the files gate" bash -c "[ $rc -eq 1 ] && grep -q '^gate files: rc=1' '$tmp/o.txt'"; else echo "skip yaml case (no PyYAML)"; fi

# mutants of gates.sh: each must turn a case red
mutant() { # <name> <sed -E expression>
  sed -E "$2" "$here/gates.sh" >"$tmp/m.sh"
  if cmp -s "$tmp/m.sh" "$here/gates.sh"; then bad "mutant '$1' changed nothing (NOT APPLIED)"; return; fi
  local m="$tmp/mut"; datarepo "$m" "$tmp/m.sh"
  run "$m"; rc=$?
  { [ "$rc" -ne 0 ] || grep -q "^gate fmt: " "$tmp/o.txt" || ! grep -q 'Cargo-Gates entfallen' "$tmp/o.txt" || ! grep -q 'NICHT gelaufen: fmt' "$tmp/o.txt" || [ -s "$STUBLOG" ]; } && ok "mutant '$1' is red" || bad "mutant '$1' SURVIVES"
}
mutant "data mode never entered" 's/data_mode=1; fi/true; fi/'
mutant "no announcement step" 's/^    step daten-modus error gates_data_mode_check "\$root"$/    true/'
mutant_hide() { # the detection looks only at the root: the sub-crate case must turn red (the mutant run is green = it survives)
  sed -E '/^gates_rust_files\(\) \{$/,/^\}$/c\
gates_rust_files() { ls "$1" | grep -iE "^cargo\\.(toml|lock)\$|\\.rs\$" || true; }' "$here/lib_gates_list.sh" >"$tmp/ml.sh"
  if cmp -s "$tmp/ml.sh" "$here/lib_gates_list.sh"; then bad "mutant 'detection root only' changed nothing (NOT APPLIED)"; return; fi
  local m="$tmp/mutl"; datarepo "$m"; cp "$tmp/ml.sh" "$m/scripts/lib_gates_list.sh"; mkdir -p "$m/sub/src"; printf '[package]\nname = "x"\nversion = "0.1.0"\n' >"$m/sub/Cargo.toml"; printf "//! Pattern: none (test).\\nfn main() {}\\n" >"$m/sub/src/main.rs"; commit "$m"
  run "$m"; rc=$?
  if [ "$rc" -eq 0 ]; then ok "mutant 'detection root only' lets the sub-crate case through (the real case above is red)"; else bad "mutant 'detection root only' SURVIVES (still red)"; fi
}
mutant_hide
mutant_noz() { # git without -z quotes umlaut/tab names: the umlaut case must go through (green = survives; the real case above is red)
  sed -E 's/ls-files -z -co/ls-files -co/; s/grep -ziE/grep -iE/' "$here/lib_gates_list.sh" >"$tmp/mz.sh"
  if cmp -s "$tmp/mz.sh" "$here/lib_gates_list.sh"; then bad "mutant 'without -z' changed nothing (NOT APPLIED)"; return; fi
  local m="$tmp/mutz"; datarepo "$m"; cp "$tmp/mz.sh" "$m/scripts/lib_gates_list.sh"; mkdir -p "$m/ä"; printf '//! Pattern: none (test).\n' >"$m/ä/lib.rs"; commit "$m"
  run "$m"; rc=$?
  if [ "$rc" -eq 0 ]; then ok "mutant 'without -z' lets the umlaut case through (the real case above is red)"; else bad "mutant 'without -z' SURVIVES (still red)"; fi
}
mutant_noz
# K4: a failing git must not look like "no Rust files": a repo whose git cannot list (broken index) is red, never data mode
g="$tmp/gitfail"; datarepo "$g"; printf 'garbage' >"$g/.git/index"; run "$g"; rc=$?
chk "git ls-files fails: not data mode, rc 1 (never green)" bash -c "[ $rc -ne 0 ] && ! grep -q 'Cargo-Gates entfallen' '$tmp/o.txt'"
mutant "fmt not guarded" 's/^  if \[ -z "\$data_mode" \]; then$/  if true; then/'
echo
if [ "$fails" = 0 ]; then echo "test_gates_data_mode: all ok"; else echo "test_gates_data_mode: $fails failures"; exit 1; fi
