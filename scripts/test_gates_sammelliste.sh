#!/usr/bin/env bash
# Pattern: none (test).
# Self test of the Sammelliste ([GATES-SAMMELLISTE]) and the TUeV mode `gates-komplett --nur-fehlerliste`, with a STUB cargo (no real build): no gate stops at the first red, ALL findings land in ONE
# list with their exact `nachpruefung:` commands, the exit code stays red, no finding = empty list and exit 0; the partial check ticks green items, runs only allowed word lists (never a shell, an error
# text never becomes a command) and is NEVER a gate result. Mutants of gates.sh and gates-komplett must each turn a case red.
#   bash scripts/test_gates_sammelliste.sh
set -uo pipefail
here="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
mkdir -p "$here/../.test-tmp"; tmp="$(TMPDIR="${TEST_TMPDIR:-$here/../.test-tmp}" mktemp -d)"  # never /tmp (RAM, rule [PLATTE])
trap '[ -n "${KEEP:-}" ] || { rm -rf "$tmp"; rmdir "$here/../.test-tmp" 2>/dev/null; }' EXIT
fails=0
: >"$tmp/.fails"
ok() { echo "ok   $1"; }
bad() { echo "FAIL $1"; fails=$((fails + 1)); echo "$1" >>"$tmp/.fails"; }
chk() { local name="$1"; shift; if "$@"; then ok "$name"; else bad "$name"; fi; }
export GIT_AUTHOR_NAME=t GIT_AUTHOR_EMAIL=t@t GIT_COMMITTER_NAME=t GIT_COMMITTER_EMAIL=t@t
unset CI CARGO_TERM_COLOR FS_GATES_SKIP_TEST
export TMPDIR_GATES="$tmp"  # the local run (no CI) of gates.sh takes its TMPDIR from here: not /srv/... (it does not exist on a runner)
export PYTHONDONTWRITEBYTECODE=1  # a __pycache__ the scripts write into the repo would be a change of the tree for the guard

# the stub cargo: RED_FMT, RED_CLIPPY, RED_TEST make the step red; every call is logged
mkdir -p "$tmp/bin"
cat >"$tmp/bin/cargo" <<'S'
#!/usr/bin/env bash
echo "cargo $*" >>"$STUBLOG"
case "$1" in
  nextest) [ "$2" = --version ] && { echo "cargo-nextest 0.9.151 (stub)"; exit 0; }; [ -n "${RED_TEST:-}" ] && { echo "        FAIL [   0.010s] fs-demo::gold it_breaks"; echo "        FAIL [   0.010s] fs-demo::gold it_breaks"; echo "        FAIL [   0.010s] (  2/818) fs-demo::gold it_breaks_too"; echo "        FAIL [   0.010s] fs-demo evil;touch_pwned"; exit 100; } ;;
  fmt) [ -n "${RED_FMT:-}" ] && { echo "Diff in src/lib.rs:1:"; exit 1; } ;;
  clippy) [ -n "${RED_CLIPPY:-}" ] && { echo "error: a lint | with a pipe"; echo " --> crates/fs-demo/src/lib.rs:3:1"; exit 101; } ;;
  test) [ -n "${RED_TEST:-}" ] && [[ "$*" != *--doc* ]] && { echo "test it_breaks ... FAILED"; echo "test evil;touch_pwned ... FAILED"; echo 'error: test failed, to rerun pass `-p fs-demo --test gold`'; exit 101; } ;;
esac
exit 0
S
printf '#!/usr/bin/env bash\necho rustc-stub\n' >"$tmp/bin/rustc"; chmod +x "$tmp/bin/cargo" "$tmp/bin/rustc"
printf '#!/usr/bin/env bash\nexec "$@"\n' >"$tmp/slot.sh"; chmod +x "$tmp/slot.sh"
export STUBLOG="$tmp/cargo.log"
# fixture repos have no origin/main: every fixture repo names HEAD as identity base in its LOCAL config (freesynergy.identity-test-base); no environment variable can do that in CI

# a mechanism folder (scripts + RUST_VERSION) and a repo with its own copy of it (no fs-release tag is pinned: the repo's own scripts count)
mkmech() { rm -rf "$1"; mkdir -p "$1"; cp -r "$here" "$1/scripts"; mkdir -p "$1/configs"; cp "$here/../configs/nextest.toml" "$1/configs/"; rm -rf "$1/scripts/__pycache__"; printf '#!/usr/bin/env bash\n[ -n "${RED_LICENCE:-}" ] && { echo "licence gate NOT RUN: stub" >&2; exit 64; }\nexit 0\n' >"$1/scripts/licence-gate.sh"; echo 1.0.0 >"$1/RUST_VERSION"; }
mkrepo() { mkmech "$1"; git init -q -b main "$1"; echo x >"$1/a.txt"; echo "__pycache__/" >"$1/.gitignore"; git -C "$1" add -A; git -C "$1" commit -qm one; git -C "$1" config --local freesynergy.identity-test-base HEAD; }
app="$tmp/app"; mkrepo "$app"
gates() { # <gates.sh> <list dir> [env...]: stdout+stderr in $tmp/o.txt; the rc is returned
  local g="$1" d="$2"; shift 2
  : >"$STUBLOG"; env "$@" PATH="$tmp/bin:$PATH" CI=true FS_GATES_FEHLERLISTE_DIR="$d" bash "$g" "$app" >"$tmp/o.txt" 2>&1
}
list() { cat "$1"/fehlerliste-*.md 2>/dev/null; }

# 1. two intentional errors in different steps: both are on the list, the run goes to the end, the exit code is red
gates "$app/scripts/gates.sh" "$tmp/l1" RED_FMT=1 RED_TEST=1; rc=$?
chk "two errors: exit code 1" [ "$rc" -eq 1 ]
chk "two errors: every step ran (an rc line for each of the 10 gates)" [ "$(grep -c '^gate [a-z-]*: rc=' "$tmp/o.txt")" -eq 10 ]
chk "two errors: both findings are on the list" bash -c "grep -q '\*\*fmt\*\*' '$tmp/l1'/fehlerliste-*.md && grep -q '\*\*test\*\*' '$tmp/l1'/fehlerliste-*.md"
chk "two errors: the list says 2 findings" grep -q '^2 findings' "$tmp/l1"/fehlerliste-main.md
chk "two errors: the red rc is printed (fmt rc=1, test rc=100 as nextest says)" bash -c "grep -q '^gate fmt: rc=1' '$tmp/o.txt' && grep -q '^gate test: rc=100' '$tmp/o.txt'"
chk "two errors: every item has its exact nachpruefung command" bash -c "grep -q 'nachpruefung: cargo fmt --check' '$tmp/l1'/*.md && grep -q 'nachpruefung: cargo test -p fs-demo --test gold --all-features --locked -- it_breaks' '$tmp/l1'/*.md"
chk "two errors: a FAIL line with the nextest counter (  2/818) is read too (real format of nextest 0.9.151)" bash -c "grep -q 'nachpruefung: cargo test -p fs-demo --test gold --all-features --locked -- it_breaks it_breaks_too' '$tmp/l1'/*.md"
chk "two errors: a test name with a shell character never reaches the list" bash -c "! grep -q 'touch_pwned' '$tmp/l1'/*.md | grep nachpruefung; ! grep 'nachpruefung' '$tmp/l1'/*.md | grep -q 'touch'"
chk "two errors: cargo test runs with --no-fail-fast and clippy with --keep-going" bash -c "grep -q 'cargo nextest run .*--no-fail-fast' '$STUBLOG' && grep -q 'cargo test --doc .*--no-fail-fast' '$STUBLOG' && grep -q 'cargo clippy .*--keep-going' '$STUBLOG'"
chk "profile: no FS_GATES_PROFILE = no --profile flag on nextest" bash -c "! grep 'cargo nextest run' '$STUBLOG' | grep -q -- --profile"
chk "two errors: the list is not in the repo tree" bash -c "[ -z \"\$(git -C '$app' status --porcelain)\" ] && [ ! -e '$app'/fehlerliste-main.md ]"
# 1a. FS_GATES_PROFILE reaches nextest (repo without .config/nextest.toml: default + visible line, no flag; with the profile defined: the flag)
gates "$app/scripts/gates.sh" "$tmp/l1p" FS_GATES_PROFILE=quick; rc=$?
chk "profile quick, not defined in the repo: green, visible line, no --profile flag" bash -c "[ $rc -eq 0 ] && grep -q 'profile quick not defined here' '$tmp/o.txt' && ! grep 'cargo nextest run' '$STUBLOG' | grep -q -- --profile"
gates "$app/scripts/gates.sh" "$tmp/l1s" FS_GATES_PROFILE=quick FS_GATES_ALSO_PROFILE=security; rc=$?
chk "second profile (security on main) not defined: green, but the Beleg says NICHT gelaufen" bash -c "[ $rc -eq 0 ] && grep -q 'gates-beleg: lokal NICHT gelaufen: test second run with profile security' '$tmp/o.txt'"
mkdir -p "$app/.config"; printf '[profile.quick]\nfail-fast = true\n' >"$app/.config/nextest.toml"
gates "$app/scripts/gates.sh" "$tmp/l1q" FS_GATES_PROFILE=quick; rc=$?
chk "profile quick, defined in the repo: nextest gets --profile quick, doc tests do not" bash -c "[ $rc -eq 0 ] && grep -q 'cargo nextest run --color never --profile quick ' '$STUBLOG' && ! grep 'cargo test --doc' '$STUBLOG' | grep -q -- --profile"
gates "$app/scripts/gates.sh" "$tmp/l1r" "FS_GATES_PROFILE=quick;id"; rc=$?
chk "profile with a shell character: refused, red, nothing run with it" bash -c "[ $rc -eq 1 ] && ! grep -q 'cargo nextest run' '$STUBLOG'"
rm -rf "$app/.config"
# 1b. the licence gate red (cargo-deny finding or NOT RUN, rc 64): it is a finding of its own with its exact nachpruefung, the exit code is red
gates "$app/scripts/gates.sh" "$tmp/l1b" RED_LICENCE=1; rc=$?
chk "licence red: exit code 1, rc 64 printed, finding on the list with its nachpruefung" bash -c "[ $rc -eq 1 ] && grep -q '^gate licence: rc=64' '$tmp/o.txt' && grep -q '\*\*licence\*\*' '$tmp/l1b'/fehlerliste-*.md && grep -q 'nachpruefung: bash {SCRIPTS}/licence-gate.sh {ROOT}' '$tmp/l1b'/fehlerliste-*.md"
# 2. no error: empty list, exit 0
gates "$app/scripts/gates.sh" "$tmp/l2"; rc=$?
chk "no error: exit code 0" [ "$rc" -eq 0 ]
chk "no error: the list says 0 findings and has no item" bash -c "grep -q '^0 findings' '$tmp/l2'/*.md && ! grep -q '^- \[' '$tmp/l2'/*.md"
# 3. one clippy error: one item with the crates named in the output
gates "$app/scripts/gates.sh" "$tmp/l3" RED_CLIPPY=1; rc=$?
chk "clippy only: exit code 1 and one finding with -p fs-demo" bash -c "[ $rc -eq 1 ] && grep -q '^1 findings' '$tmp/l3'/*.md && grep -q 'nachpruefung: cargo clippy -p fs-demo --all-targets' '$tmp/l3'/*.md"
chk "a pipe character in an error line does not break the list" bash -c "grep -q 'a lint / with a pipe' '$tmp/l3'/*.md || grep -q 'a lint | with a pipe' '$tmp/l3'/*.md"

# 4. the partial check (TEILPRUEFUNG): a fixed error is ticked, an open one stays, nothing looks like a gate result
export FS_GATES_BUILD_SLOT="$tmp/slot.sh" FS_GATES_SCRATCH="$tmp/scratch" FS_GATES_FMT=true
teil() { PATH="$tmp/bin:$PATH" FS_GATES_FEHLERLISTE_DIR="$(dirname "$1")" python3 -I "$app/scripts/gates-komplett" --repo-dir "$app" --nur-fehlerliste "$1" >"$tmp/t.txt" 2>&1; }
mkdir -p "$tmp/pl"; cp "$tmp/l1/fehlerliste-main.md" "$tmp/pl/fehlerliste-liste.md"
: >"$STUBLOG"; RED_TEST=1 teil "$tmp/pl/fehlerliste-liste.md"; rc=$?
chk "partial: the fixed fmt item is ticked with a UTC time" bash -c "grep -q '^- \[x\] \*\*fmt\*\*' '$tmp/pl/fehlerliste-liste.md' && grep -Eq 'nachgeprueft: [0-9]{4}-[0-9]{2}-[0-9]{2}T[0-9:]{8}Z' '$tmp/pl/fehlerliste-liste.md'"
chk "partial: the open test item stays open with its rc and error lines, exit 1" bash -c "[ $rc -eq 1 ] && grep -q '^- \[ \] \*\*test\*\*' '$tmp/pl/fehlerliste-liste.md' && grep -q 'letzte pruefung: .* rc 101' '$tmp/pl/fehlerliste-liste.md'"
chk "partial: only the commands of the list ran (no clippy, no gates.sh)" bash -c "! grep -q 'cargo clippy' '$STUBLOG' && grep -q 'cargo fmt --check' '$STUBLOG' && grep -q 'cargo test -p fs-demo --test gold' '$STUBLOG'"
chk "partial: the last line says TEILPRUEFUNG, no gate result" bash -c "[ \"\$(tail -n 1 '$tmp/t.txt')\" = 'TEILPRÜFUNG: kein Gate-Ergebnis, volle CI nötig' ]"
chk "partial: the JSON is no gate result (status teilpruefung, red, no head, no runs)" python3 -I - "$tmp/t.txt" <<'PY'
import json, sys
d = json.loads([l for l in open(sys.argv[1], encoding="utf-8") if l.startswith("{")][0])
assert d["status"] == "teilpruefung" and d["red"] is True and d["gate_ergebnis"] is False
assert "kopf" not in d and "laeufe" not in d and "befunde" not in d
PY
RED_TEST='' teil "$tmp/pl/fehlerliste-liste.md"; rc=$?
chk "partial: all items green ends with exit 3 (never 0) and STILL says TEILPRUEFUNG and is no gate result" bash -c "[ $rc -eq 3 ] && [ \"\$(tail -n 1 '$tmp/t.txt')\" = 'TEILPRÜFUNG: kein Gate-Ergebnis, volle CI nötig' ] && grep -q '\"red\": true' '$tmp/t.txt'"
chk "partial: no gate marker file is written (repo tree and list folder hold only the list)" bash -c "[ -z \"\$(git -C '$app' status --porcelain)\" ] && ! ls '$tmp'/l1 '$tmp/scratch' 2>/dev/null | grep -qiE 'gates.json|gate-ergebnis|\.ok$'"
# 5. injection: a hand-edited list (or an error text) cannot run anything but the allowed word lists
rm -f "$tmp/PWNED"
mkdir -p "$tmp/pe"; cat >"$tmp/pe/fehlerliste-evil.md" <<E
# Fehlerliste
- [ ] **a**: x
  - nachpruefung: cargo test; touch $tmp/PWNED
- [ ] **b**: x
  - nachpruefung: cargo fmt --check && touch $tmp/PWNED
- [ ] **c**: x
  - nachpruefung: sh -c touch
- [ ] **d**: x
  - nachpruefung: cargo test \$(touch $tmp/PWNED)
- [ ] **e**: x
  - nachpruefung: cargo build
- [ ] **f**: x
  - nachpruefung: python3 -I {SCRIPTS}/../../etc/passwd
- [ ] **g**: x
  - first error lines: x
  - nachpruefung: cargo fmt --check
E
: >"$STUBLOG"; teil "$tmp/pe/fehlerliste-evil.md"; rc=$?
chk "injection: only the one allowed command ran, nothing was created" bash -c "[ ! -e '$tmp/PWNED' ] && [ \"\$(cat '$STUBLOG')\" = 'cargo fmt --check' ]"
chk "injection: the refused items are open with the reason" bash -c "[ \$(grep -c 'command refused' '$tmp/pe/fehlerliste-evil.md') -ge 6 ] && [ $rc -eq 1 ]"
# a finding whose error text imitates a nachpruefung line cannot become one (the text is one line behind `first error lines`)
chk "injection: error text that looks like a command stays in the first-error line" bash -c "! grep -q '^  - nachpruefung: touch' '$tmp/l1'/*.md"

# 6. a fixed error and a new one: the partial check ticks the first, the full check finds the second
mkrepo "$app"
gates "$app/scripts/gates.sh" "$tmp/l6" RED_FMT=1 >/dev/null; mkdir -p "$tmp/p6"; cp "$tmp/l6"/fehlerliste-main.md "$tmp/p6/fehlerliste-l6.md"
: >"$STUBLOG"; RED_FMT='' RED_CLIPPY=1 teil "$tmp/p6/fehlerliste-l6.md"; rc=$?
chk "fixed + new: the partial check reports the fixed error green (ticked), exit 3 (never 0)" bash -c "grep -q '^- \[x\] \*\*fmt\*\*' '$tmp/p6/fehlerliste-l6.md' && [ $rc -eq 3 ]"
PATH="$tmp/bin:$PATH" RED_CLIPPY=1 FS_GATES_FEHLERLISTE_DIR="$tmp/l6k" python3 -I "$app/scripts/gates-komplett" --repo-dir "$app" --log-dir "$tmp/logs6" --ohne-flachprobe >"$tmp/k.json" 2>"$tmp/k.err"; rc=$?
chk "fixed + new: the full check (gates-komplett) is red and its list has the NEW clippy error, not the fixed fmt" bash -c "[ $rc -eq 1 ] && grep -q '\*\*clippy\*\*' '$tmp/l6k'/fehlerliste-main.md && ! grep -q '\*\*fmt\*\*' '$tmp/l6k'/fehlerliste-main.md"
# 7. gates-komplett: two errors in different steps, three runs all go to the end, ONE merged list, exit 1; clean = empty list, exit 0
mkrepo "$app"
PATH="$tmp/bin:$PATH" RED_FMT=1 RED_TEST=1 FS_GATES_FEHLERLISTE_DIR="$tmp/l7" python3 -I "$app/scripts/gates-komplett" --repo-dir "$app" --log-dir "$tmp/logs7" >"$tmp/k.json" 2>"$tmp/k.err"; rc=$?
chk "gates-komplett: red (rc 1), ALL THREE runs ran to the end" bash -c "[ $rc -eq 1 ] && python3 -I -c 'import json,sys; d=json.load(open(sys.argv[1])); assert [l[\"rc\"] for l in d[\"laeufe\"]] == [1,1,1], d[\"laeufe\"]' '$tmp/k.json'"
chk "gates-komplett: the merged list has both findings once, with their runs" bash -c "[ \$(grep -c '^- \[ \] \*\*fmt\*\*' '$tmp/l7'/fehlerliste-main.md) -eq 1 ] && [ \$(grep -c '^- \[ \] \*\*test\*\*' '$tmp/l7'/fehlerliste-main.md) -eq 1 ] && grep -q 'runs: lokal, ci, flach-ohne-tags' '$tmp/l7'/fehlerliste-main.md"
mkrepo "$app"
PATH="$tmp/bin:$PATH" FS_GATES_FEHLERLISTE_DIR="$tmp/l7b" python3 -I "$app/scripts/gates-komplett" --repo-dir "$app" --log-dir "$tmp/logs7b" >"$tmp/k.json" 2>"$tmp/k.err"; rc=$?
[ "$rc" -eq 0 ] || { echo "--- diagnostics (7b): rc=$rc"; cat "$tmp/k.err" | head -20; head -c 600 "$tmp/k.json"; echo; ls -R "$tmp/l7b" "$tmp/logs7b" 2>&1 | head -20; }
chk "gates-komplett: no error: exit 0, empty list" bash -c "[ $rc -eq 0 ] && grep -q '^0 findings' '$tmp/l7b'/fehlerliste-main.md && ! grep -q '^- \[' '$tmp/l7b'/fehlerliste-main.md"

# 8. mutants: each one must turn a case red
probes_check() { # <scripts dir>: the argument whitelist and the rules for the list file (the probes of the review)
      python3 -I - "$1/kaskade_lib" "$tmp" <<'PY'
import os, sys, tempfile
sys.path.insert(0, sys.argv[1])
import teil
refused = ["cargo test --config=/tmp/evil.toml", "cargo test --manifest-path=/tmp/evil/Cargo.toml", "cargo clippy -Zunstable-options", "cargo test --target-dir=/",
           "cargo fmt -- --config-path=/tmp/x", "bash {SCRIPTS}/gates-lock.sh /etc", "scripts/gates-komplett --repo-dir=/ --nur-fehlerliste=/tmp/x.md", "cargo test -C /tmp",
           "cargo test -p x RUSTC=evil", "cargo test /etc/passwd", "cargo test ../x", "cargo test -p /abs", "cargo test -- --test-threads=1;", "cargo test --config build.rustc-wrapper=x",
           "cargo test -Zunstable", "cargo test -p x -- --include-ignored", "python3 -I {SCRIPTS}/check_rust_rules.py /etc", "python3 -c x", "bash {SCRIPTS}/gates-lock.sh {ROOT} extra", "bash -c x", "bash {SCRIPTS}/licence-gate.sh /etc", "bash {SCRIPTS}/licence-gate.sh {ROOT} extra",
           "cargo test -p x -- /etc/passwd", "cargo test -p x -- ../x", "cargo test it_works", "cargo test --workspace --", "cargo test -p x --", "cargo clippy -p x", "cargo clippy --workspace --all-targets", "cargo clippy -p x -- -D warnings", "cargo fmt -- --check", "cargo build", "cargo test -p x $(touch y)", "cargo test | touch y", "cargo clippy -p x -- -A warnings", "cargo clippy -- -D warnings --cap-lints allow", "cargo test --features=evil",
           "cargo test --release", "cargo fmt --check --config-path=/x", "cargo test -p ../x", "cargo test -p x -p"]
good = ["cargo fmt --check", "cargo clippy -p fs-demo --all-targets --all-features --locked -- -D warnings -W clippy::pedantic", "cargo test -p fs-demo --test gold --all-features --locked -- it_breaks mod_a::case",
        "cargo test --workspace --all-features --locked --no-fail-fast", "cargo test -p x --lib", "cargo test -p x --bin fs-x --locked -q", "python3 -I {SCRIPTS}/check_rust_rules.py {ROOT}",
        "bash {SCRIPTS}/gates-lock.sh {ROOT}", "bash {SCRIPTS}/gates-files.sh {ROOT}", "bash {SCRIPTS}/licence-gate.sh {ROOT}", "cargo fmt --all --check", "cargo clippy --workspace --all-targets --all-features --locked --keep-going -- -D warnings -W clippy::pedantic"]
bad = [c for c in refused if teil.check_command(c, "/s", "/r") is not None]
miss = [c for c in good if teil.check_command(c, "/s", "/r") is None]
assert not bad and not miss, (bad, miss)
assert teil.check_command("cargo test -p x --lib", "/s", "/r") == ["cargo", "test", "-p", "x", "--lib"]
assert teil.check_command("bash {SCRIPTS}/gates-lock.sh {ROOT}", "/s", "/r") == ["bash", "/s/gates-lock.sh", "/r"]
d = tempfile.mkdtemp(dir=sys.argv[2])
f = os.path.join(d, "fehlerliste-main.md"); open(f, "w").write("x\n"); os.chmod(f, 0o600)
uid = os.getuid()
assert teil.read_list_file(f, d, uid) == ("x\n", None)
assert teil.read_list_file(f, d, uid + 1)[1], "foreign owner"
os.chmod(f, 0o666); assert teil.read_list_file(f, d, uid)[1], "writable by others"; os.chmod(f, 0o600)
link = os.path.join(d, "fehlerliste-link.md"); os.symlink(f, link)
assert teil.read_list_file(link, d, uid)[1], "a symlink to a good file is refused (O_NOFOLLOW)"
assert teil.read_list_file(f, os.path.join(d, "other"), uid)[1], "other folder"
g = os.path.join(d, "list.md"); open(g, "w").write("x"); os.chmod(g, 0o600); assert teil.read_list_file(g, d, uid)[1], "name"
os.mkdir(os.path.join(d, "fehlerliste-dir.md")); assert teil.read_list_file(os.path.join(d, "fehlerliste-dir.md"), d, uid)[1], "a folder"
assert teil.read_list_file(os.path.join(d, "fehlerliste-none.md"), d, uid)[1], "missing"
PY
}
rc_check() { # <mechanism folder>: the reading of the rc of a step
      bash -c '. "$1/scripts/lib_gates_list.sh"; d=$(mktemp -d); : > "$d/e"; echo abc > "$d/a"; echo 0 > "$d/z"; echo 101 > "$d/n"; [ "$(gates_read_rc "$d/e")" = 1 ] && [ "$(gates_read_rc "$d/a")" = 1 ] && [ "$(gates_read_rc "$d/none")" = 1 ] && [ "$(gates_read_rc "$d/z")" = 0 ] && [ "$(gates_read_rc "$d/n")" = 101 ]' _ "$1" 
}
chk "whitelist: every probe of the review is refused, every normal form of the list is accepted; the list file must be ours, plain, not writable by others, in its folder" probes_check "$here"
chk "rc of a step: missing, empty or no number is red, a number is that number" rc_check "$(dirname "$here")"

mut() { # <name> <file> <python expression old> <new> <suite>: makes the mutated copy in a mechanism folder and runs the cases that must catch it
  local name="$1" file="$2" old="$3" new="$4" want="$5"
  local m="$tmp/mut-$$-$RANDOM"; mkmech "$m"
  if ! python3 -I - "$m/scripts/$file" "$old" "$new" <<'PY'
import sys
p, old, new = sys.argv[1:4]
t = open(p, encoding="utf-8").read()
for o, n in zip(old.split("@@@"), new.split("@@@")):  # several replacements of ONE mutant are separated by @@@
    if t.count(o) < 1:
        sys.exit(1)
    t = t.replace(o, n, 1)
open(p, "w", encoding="utf-8").write(t)
PY
  then bad "mutant $name: NOT APPLIED (pattern gone)"; return; fi
  local r=0
  case "$want" in
    gates) # two errors: a list with both, exit 1, 9 rc lines, nachpruefung
      local dd="$tmp/mm-$RANDOM"; mkrepo "$tmp/app-m"; cp -r "$m/scripts" "$tmp/app-m/"; git -C "$tmp/app-m" add -A; git -C "$tmp/app-m" commit -qm m
      : >"$STUBLOG"; env RED_FMT=1 RED_TEST=1 PATH="$tmp/bin:$PATH" CI=true FS_GATES_FEHLERLISTE_DIR="$dd" bash "$tmp/app-m/scripts/gates.sh" "$tmp/app-m" >"$tmp/mo.txt" 2>&1; local rc=$?
      [ "$rc" -eq 1 ] && [ "$(grep -c '^gate [a-z-]*: rc=' "$tmp/mo.txt")" -eq 10 ] && grep -q '\*\*fmt\*\*' "$dd"/*.md 2>/dev/null && grep -q '\*\*test\*\*' "$dd"/*.md && grep -q 'nachpruefung: cargo fmt --check' "$dd"/*.md && grep -q 'cargo nextest run .*--no-fail-fast' "$STUBLOG" && grep -q 'cargo test --doc .*--no-fail-fast' "$STUBLOG" && [ -z "$(git -C "$tmp/app-m" status --porcelain)" ] && [ ! -e "$tmp/app-m"/fehlerliste-main.md ] || r=1
      ;;
    teil) # the partial check: tick, injection, no gate result, no abort at the first green
      mkrepo "$tmp/app-m"; cp -r "$m/scripts" "$tmp/app-m/"; git -C "$tmp/app-m" add -A; git -C "$tmp/app-m" commit -qm m; rm -f "$tmp/PWNED"
      mkdir -p "$tmp/pm"; printf -- '- [ ] **a**: x\n  - nachpruefung: cargo fmt --check\n- [ ] **b**: x\n  - nachpruefung: cargo test -p fs-demo --test gold -- it_breaks\n- [ ] **c**: x\n  - nachpruefung: cargo test -p x --lib;touch %s\n- [ ] **d**: x\n  - nachpruefung: sh -c touch\n- [ ] **e**: x\n  - nachpruefung: cargo clippy -p fs-demo --all-targets\n' "$tmp/PWNED" >"$tmp/pm/fehlerliste-ml.md"
      : >"$STUBLOG"; PATH="$tmp/bin:$PATH" FS_GATES_FEHLERLISTE_DIR="$tmp/pm" RED_TEST=1 python3 -I "$tmp/app-m/scripts/gates-komplett" --repo-dir "$tmp/app-m" --nur-fehlerliste "$tmp/pm/fehlerliste-ml.md" >"$tmp/mt.txt" 2>&1; local rc=$?
      printf -- '- [ ] **a**: x\n  - nachpruefung: cargo fmt --check\n' >"$tmp/pm/fehlerliste-mg.md"
      PATH="$tmp/bin:$PATH" FS_GATES_FEHLERLISTE_DIR="$tmp/pm" python3 -I "$tmp/app-m/scripts/gates-komplett" --repo-dir "$tmp/app-m" --nur-fehlerliste "$tmp/pm/fehlerliste-mg.md" >/dev/null 2>&1; [ "$?" -eq 3 ] || r=1  # all ticked: exit 3, never 0
      python3 -I - "$tmp/mt.txt" "$tmp/pm/fehlerliste-ml.md" "$STUBLOG" "$tmp/PWNED" "$rc" <<'PY' || r=1
import json, os, sys
out, lst, log, pwned, rc = sys.argv[1:6]
d = json.loads([l for l in open(out, encoding="utf-8") if l.startswith("{")][0])
t = open(lst, encoding="utf-8").read()
assert d["status"] == "teilpruefung" and d["red"] is True and "kopf" not in d and "laeufe" not in d
assert open(out, encoding="utf-8").read().rstrip("\n").endswith("TEILPRÜFUNG: kein Gate-Ergebnis, volle CI nötig")
assert t.count("- [x]") == 2 and t.count("- [ ]") == 3, t  # a and e are green and ticked; b is red (its command really ran); c and d are refused; no abort at the first green
assert "rc 101" in t and t.count("command refused") == 2 and not os.path.exists(pwned)
assert int(rc) == 1
runs = open(log, encoding="utf-8").read().splitlines()
assert len(runs) == 3, runs  # a, b, e ran (c and d did not)
PY
      ;;
    probes) probes_check "$m/scripts" || r=1 ;;
    rc) rc_check "$m" || r=1 ;;
    full) # gates-komplett with two errors: the three runs all go to the end
      mkrepo "$tmp/app-m"; cp -r "$m/scripts" "$tmp/app-m/"; git -C "$tmp/app-m" add -A; git -C "$tmp/app-m" commit -qm m
      PATH="$tmp/bin:$PATH" RED_FMT=1 RED_TEST=1 FS_GATES_FEHLERLISTE_DIR="$tmp/mm-l" python3 -I "$tmp/app-m/scripts/gates-komplett" --repo-dir "$tmp/app-m" --log-dir "$tmp/mm-logs" >"$tmp/mk.json" 2>/dev/null
      python3 -I -c 'import json,sys; d=json.load(open(sys.argv[1])); assert [l["rc"] for l in d["laeufe"]] == [1,1,1]' "$tmp/mk.json" || r=1
      ;;
  esac
  if [ "$r" -ne 0 ]; then ok "mutant $name is red"; else bad "mutant $name SURVIVED"; fi
  rm -rf "$m" "$tmp/app-m" "$tmp/mm-"* 2>/dev/null
}
mut "step swallows the rc"            gates.sh 'rc="$(gates_read_rc "$out.rc")"' 'rc=0' gates
mut "stop at the first red"           gates.sh '    finding_names+=("$name")' '    finding_names+=("$name"); echo "gates FAILED: stop" >&2; exit 1' gates
mut "list incomplete (first only)"    gates.sh '    findings+=("$name|$severity|$rc|$*")' '    [ "${#findings[@]}" -eq 0 ] && findings+=("$name|$severity|$rc|$*")' gates
mut "exit 0 although findings"        gates.sh '    exit 1
  fi
fi
echo "gates ok' '    exit 0
  fi
fi
echo "gates ok' gates
mut "list without nachpruefung"       gates.sh 'nachpruefung_of "$name" "$finding_work/$name.out" | awk' 'true | awk' gates
mut "doctest without --no-fail-fast"  gates.sh '--doc "${testsel[@]}" --all-features --locked --no-fail-fast' '--doc "${testsel[@]}" --all-features --locked' gates
mut "nextest without --no-fail-fast"  test-gate.sh '--no-tests=pass --no-fail-fast "$@"' '--no-tests=pass "$@"' gates
mut "list inside the repo tree"       gates.sh '  case "$(cd "$dir" && pwd)/" in "$root"/*) dir="${TMPDIR:-/tmp}" ;; esac' '  dir="$root"' gates
mut "guard step removed"             gates.sh '  step guard error gates_guard_compare "$guard_snap" "$guard_snap.end"' '  true' gates
mut "list file not written"           gates.sh '  } > "$findings_file"' '  } > /dev/null' gates
mut "tick without running"            kaskade_lib/teil.py '            if rc != 0:
                reds.append((cmd, rc, " // ".join(err)))' '            if False:
                reds.append((cmd, rc, " // ".join(err)))' teil
mut "partial run writes a gate result" gates-komplett '"status": "teilpruefung", "red": True' '"status": "ok", "red": False' teil
mut "partial without the TEILPRUEFUNG line" gates-komplett '    print(teil.LAST_LINE)' '    pass' teil
mut "partial exits 0 when all is ticked" kaskade_lib/teil.py 'EXIT_OPEN, EXIT_PART_GREEN = 1, 3' 'EXIT_OPEN, EXIT_PART_GREEN = 1, 0' teil
mut "injection through a shell"       kaskade_lib/teil.py 'rc = subprocess.run([slot, *argv], cwd=repo_dir,@@@    if not w or any(not x for x in w) or not _check_argv(w):' 'rc = subprocess.run(" ".join([slot, *argv]), shell=True, cwd=repo_dir,@@@    if not w:' teil
mut "word check off"                  kaskade_lib/teil.py '    if not w or any(not x for x in w) or not _check_argv(w):' '    if not w:' teil
mut "whitelist: every program allowed" kaskade_lib/teil.py '        return w == ["python3", "-I", "{SCRIPTS}/check_rust_rules.py", "{ROOT}"]
    return False' '        return w == ["python3", "-I", "{SCRIPTS}/check_rust_rules.py", "{ROOT}"]
    return True' teil
mut "whitelist: any option allowed"   kaskade_lib/teil.py '        else:
            return False
    return True' '        elif w.startswith("-"):
            i += 1
        else:
            return False
    return True' probes
mut "whitelist: paths allowed as filter" kaskade_lib/teil.py 'TEST = r"[A-Za-z0-9_]+(?:::[A-Za-z0-9_]+)*"' 'TEST = r"[A-Za-z0-9_/.:]+"' probes
mut "whitelist: cargo test unchecked" kaskade_lib/teil.py '    if w[0] == "cargo" and w[1] == "test":
' '    if w[0] == "cargo" and w[1] == "test":
        return True
' probes
mut "whitelist: name regex loose"      kaskade_lib/teil.py 'NAME = r"[A-Za-z0-9_][A-Za-z0-9_-]*"' 'NAME = r"[^ ]+"' probes
mut "whitelist: clippy tail not needed" kaskade_lib/teil.py 'and tail == ["-D", "warnings", "-W", "clippy::pedantic"]' 'and True' probes
mut "whitelist: a bare -- allowed"    kaskade_lib/teil.py '(tail is None or (len(tail) > 0 and all(' '(tail is None or (all(' probes
mut "whitelist: bash arguments free"  kaskade_lib/teil.py ' and w[2] == "{ROOT}"' '' probes
mut "whitelist empty"                 kaskade_lib/teil.py '    if len(w) < 2:
        return False
    if w[0] == "cargo" and w[1] == "test":' '    return False
    if w[0] == "cargo" and w[1] == "test":' probes
mut "list: symlink followed"          kaskade_lib/teil.py 'os.O_RDONLY | os.O_NOFOLLOW | ' 'os.O_RDONLY | ' probes
mut "list: no regular file needed"    kaskade_lib/teil.py '        if not stat.S_ISREG(st.st_mode):' '        if False:' probes
mut "list: foreign owner allowed"     kaskade_lib/teil.py '    if st.st_uid != uid:' '    if False:' probes
mut "list: writable by others allowed" kaskade_lib/teil.py '    if st.st_mode & 0o022:' '    if False:' probes
mut "list: any folder allowed"        kaskade_lib/teil.py '    if os.path.realpath(os.path.dirname(os.path.abspath(path))) != os.path.realpath(list_dir):' '    if False:' probes
mut "rc of a step: empty is green"    lib_gates_list.sh 'if [[ "$v" =~ ^[0-9]+$ ]]; then echo "$v"; else echo 1; fi' 'if [[ "$v" =~ ^[0-9]+$ ]]; then echo "$v"; else echo 0; fi' rc
mut "rc of a step: missing is green"  lib_gates_list.sh '  [ -r "$1" ] && v="$(cat "$1")"' '  [ -r "$1" ] && v="$(cat "$1")" || v=0' rc
mut "full check stops at the first red" gates-komplett '        for name, wd, e in plan:
' '        for name, wd, e in plan:
            if befunde:
                laeufe.append({"name": name, "rc": None, "log": None, "gates": [], "fehler": [], "funde": []}); continue
' full

if [ "$fails" -ne 0 ]; then echo; echo "test_gates_sammelliste: $fails failures"; exit 1; fi
echo; echo "test_gates_sammelliste: all ok"
