#!/usr/bin/env bash
# Self test of scripts/umbenennen with small fixtures: dry run (plan, clashes, retired names), after-check (leftovers, replaces,
# retired list) and the name lock, with counter-checks (every rule must really turn the result red).
#   bash scripts/test_umbenennen.sh
set -uo pipefail
here="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
tool=(python3 -I "$here/umbenennen")
tmp="$(mktemp -d)"
trap 'rm -rf "$tmp"' EXIT
fails=0

expect() { # expect <rc> <needle or -> <name> -- args...
  local want="$1" needle="$2" name="$3"
  shift 4
  local out rc
  out="$("${tool[@]}" "$@" 2>&1)"
  rc=$?
  if [ "$rc" -ne "$want" ] || { [ "$needle" != "-" ] && ! grep -qF -- "$needle" <<<"$out"; }; then
    echo "FAIL $name: rc=$rc (want $want), needle '$needle'"
    printf "    %s\n" "${out//$'\n'/$'\n    '}"
    fails=$((fails + 1))
  else
    echo "ok   $name"
  fi
}

mk() { # mk <dir>: the world before the rename
  mkdir -p "$1/fs-wiki/crates/fs-wiki-core" "$1/fs-wiki/.github/workflows" "$1/fs-app" "$1/fs-catalog" "$1/fs-documentation/docs/de"
  printf '[package]\nname = "fs-wiki-core"\nversion = "0.0.10"\n' >"$1/fs-wiki/crates/fs-wiki-core/Cargo.toml"
  printf '[package]\nname = "fs-wiki"\n' >"$1/fs-wiki/Cargo.toml"
  printf 'jobs:\n  b:\n    steps:\n      - run: cargo build -p fs-wiki-*\n' >"$1/fs-wiki/.github/workflows/release.yml"
  printf '[dependencies]\nfs-wiki-core = { git = "https://github.com/FreeSynergy/fs-wiki", tag = "v0.0.10" }\n' >"$1/fs-app/Cargo.toml"
  printf '[[package]]\nname = "fs-wiki"\nkind = "program"\n' >"$1/fs-catalog/packages.toml"
  printf 'Das Wiki fs-wiki ist ein Programm. Nicht fs-wikipedia.\n' >"$1/fs-documentation/docs/de/wiki.md"
  : >"$1/retired.txt"
}
args() { echo --old fs-wiki --new fs-text --repos "$1/fs-wiki" "$1/fs-app" --catalog "$1/fs-catalog" --docs "$1/fs-documentation" --retired "$1/retired.txt" --state "$1/state.$RANDOM"; }

w="$tmp/w1"; mk "$w"
# shellcheck disable=SC2046
expect 0 "crates: 2" "dry run lists crates, CI, pins, catalog, docs" -- dry-run $(args "$w")
# shellcheck disable=SC2046
expect 0 "new=fs-text-core" "dry run plans the new crate name" -- dry-run $(args "$w")
# shellcheck disable=SC2046
expect 0 "docs: 1" "fs-wikipedia is not the old name" -- dry-run $(args "$w")

echo "fs-text -> fs-other  # test" >"$w/retired.txt"
# shellcheck disable=SC2046
expect 1 "never given out again" "red: the new name was given up before" -- dry-run $(args "$w")
printf 'fs-wiki -> fs-x\n' >"$w/retired.txt"
# shellcheck disable=SC2046
expect 1 "already retired" "red: the old name is already retired" -- dry-run $(args "$w")
: >"$w/retired.txt"

mkdir "$w/fs-text"
# shellcheck disable=SC2046
expect 1 "already exists" "red: a checkout of the new name exists" -- dry-run --old fs-wiki --new fs-text --repos "$w/fs-wiki" "$w/fs-app" "$w/fs-text"
rmdir "$w/fs-text"
printf '[[package]]\nname = "fs-text"\n' >>"$w/fs-catalog/packages.toml"
# shellcheck disable=SC2046
expect 1 "catalog already has an entry" "red: catalog entry of the new name exists" -- dry-run $(args "$w")

expect 1 "not of the form" "red: bad name form" -- dry-run --old wiki --new fs-text --repos "$w/fs-wiki"
expect 1 "are equal" "red: equal names" -- dry-run --old fs-wiki --new fs-wiki --repos "$w/fs-wiki"
expect 2 "not a folder" "bad input: missing folder" -- dry-run --old fs-wiki --new fs-text --repos "$w/nope"
mkdir "$tmp/empty"
expect 1 "nothing in the given places" "red: wrong folders" -- dry-run --old fs-wiki --new fs-text --repos "$tmp/empty"

# --- after: the finished state
v="$tmp/w2"; mk "$v"
rm -rf "$v/fs-wiki/crates/fs-wiki-core"; mkdir -p "$v/fs-wiki/crates/fs-text-core"
printf '[package]\nname = "fs-text-core"\n' >"$v/fs-wiki/crates/fs-text-core/Cargo.toml"
printf '[package]\nname = "fs-text"\n' >"$v/fs-wiki/Cargo.toml"
printf 'jobs:\n  b:\n    steps:\n      - run: cargo build -p fs-text-*\n' >"$v/fs-wiki/.github/workflows/release.yml"
printf '[dependencies]\nfs-text-core = { git = "https://github.com/FreeSynergy/fs-text", tag = "v0.0.11" }\n' >"$v/fs-app/Cargo.toml"
printf '[[package]]\nname = "fs-text"\nreplaces = ["fs-wiki"]\n' >"$v/fs-catalog/packages.toml"
printf 'Das Programm fs-text.\n' >"$v/fs-documentation/docs/de/wiki.md"
printf 'fs-wiki -> fs-text  # ADR 0080\n' >"$v/retired.txt"
# shellcheck disable=SC2046
expect 0 "green" "after: everything renamed" -- after $(args "$v")

printf '[package]\nname = "fs-wiki-core"\n' >"$v/fs-wiki/crates/fs-text-core/Cargo.toml"
# shellcheck disable=SC2046
expect 1 "crate 'fs-wiki-core' still named" "red: old crate name left" -- after $(args "$v")
printf '[package]\nname = "fs-text-core"\n' >"$v/fs-wiki/crates/fs-text-core/Cargo.toml"

printf 'fs-wiki-core = { git = "https://github.com/FreeSynergy/fs-wiki", tag = "v0.0.10" }\n' >"$v/fs-app/Cargo.toml"
# shellcheck disable=SC2046
expect 1 "pin to the old name" "red: old pin left" -- after $(args "$v")
printf 'fs-text-core = { git = "https://github.com/FreeSynergy/fs-text", tag = "v0.0.11" }\n' >"$v/fs-app/Cargo.toml"

printf 'cargo build -p fs-wiki-core\n' >"$v/fs-wiki/.github/workflows/release.yml"
# shellcheck disable=SC2046
expect 1 "old name is still in" "red: old name in CI" -- after $(args "$v")
printf 'cargo build -p fs-text-core\n' >"$v/fs-wiki/.github/workflows/release.yml"

printf 'Siehe fs-wiki.\n' >"$v/fs-documentation/docs/de/wiki.md"
# shellcheck disable=SC2046
expect 1 "old name still in the docs" "red: old name in docs" -- after $(args "$v")
# shellcheck disable=SC2046
expect 0 "green" "old name in an allowed history page" -- after $(args "$v") --allow 'docs/de/wiki.md'
printf 'Das Programm fs-text.\n' >"$v/fs-documentation/docs/de/wiki.md"

printf '[[package]]\nname = "fs-text"\n' >"$v/fs-catalog/packages.toml"
# shellcheck disable=SC2046
expect 1 "has no replaces" "red: replaces missing" -- after $(args "$v")
printf '[[package]]\nname = "fs-text"\nreplaces = ["fs-wiki"]\n' >"$v/fs-catalog/packages.toml"

: >"$v/retired.txt"
# shellcheck disable=SC2046
expect 1 "not on the retired list" "red: old name not retired" -- after $(args "$v")

# --- retire
expect 0 "retired: fs-wiki -> fs-text" "retire appends" -- retire --old fs-wiki --new fs-text --retired "$v/retired.txt"
expect 0 "already retired, same pair" "retire twice for the same pair is a no-op (the loop back to umbau)" -- retire --old fs-wiki --new fs-text --retired "$v/retired.txt"
if [ "$(grep -c '^fs-wiki -> fs-text' "$v/retired.txt")" -eq 1 ]; then echo "ok   retire twice: still exactly one entry"; else echo "FAIL retire twice wrote a second entry"; fails=$((fails + 1)); fi
expect 1 "already retired" "retire for the same old name with ANOTHER new name is refused" -- retire --old fs-wiki --new fs-other --retired "$v/retired.txt"
expect 1 "never given out again" "retire onto a retired name is refused" -- retire --old fs-text --new fs-wiki --retired "$v/retired.txt"
# shellcheck disable=SC2046
expect 0 "green" "after is green once retired" -- after $(args "$v")

# --- round counter of `after` (the script counts, rc 3 from the third red round, green resets)
printf 'Siehe fs-wiki.\n' >"$v/fs-documentation/docs/de/wiki.md"
st=(--old fs-wiki --new fs-text --repos "$v/fs-wiki" "$v/fs-app" --catalog "$v/fs-catalog" --docs "$v/fs-documentation" --retired "$v/retired.txt" --state "$v/rounds")
expect 1 "runde=1" "after: red round 1" -- after "${st[@]}"
expect 1 "runde=2" "after: red round 2" -- after "${st[@]}"
expect 3 "runde=3" "after: red round 3 escalates (rc 3)" -- after "${st[@]}"
printf 'Das Programm fs-text.\n' >"$v/fs-documentation/docs/de/wiki.md"
expect 0 "runde=0" "after: green resets the counter" -- after "${st[@]}"
printf 'Siehe fs-wiki.\n' >"$v/fs-documentation/docs/de/wiki.md"
expect 1 "runde=1" "after: red starts at 1 again after green" -- after "${st[@]}"
js="$tmp/after.json"
"${tool[@]}" after "${st[@]}" --json "$js" >/dev/null
if python3 -I -c 'import json,sys; d=json.load(open(sys.argv[1])); sys.exit(0 if d["red"] is True and d["runde"]==2 else 1)' "$js"; then
  echo "ok   after: the JSON carries red and runde"; else echo "FAIL after: JSON"; fails=$((fails + 1)); fi
"${tool[@]}" dry-run --old fs-wiki --new fs-text --repos "$tmp/nope" --json "$js" >/dev/null
if python3 -I -c 'import json,sys; d=json.load(open(sys.argv[1])); sys.exit(0 if d["red"] is None else 1)' "$js"; then
  echo "ok   bad input: the JSON has red = null (never green)"; else echo "FAIL bad input JSON"; fails=$((fails + 1)); fi

# --- state file: the default is a private per-user folder (never a predictable name in /tmp), and a symlink is never followed
xs="$tmp/xdg"; mkdir -p "$xs"
nostate=(--old fs-wiki --new fs-text --repos "$v/fs-wiki" "$v/fs-app" --catalog "$v/fs-catalog" --docs "$v/fs-documentation" --retired "$v/retired.txt")
XDG_STATE_HOME="$xs" "${tool[@]}" after "${nostate[@]}" >/dev/null 2>&1
if [ -f "$xs/freesynergy/umbenennen-fs-wiki-fs-text-runde" ] && [ "$(stat -c %a "$xs/freesynergy")" = 700 ]; then
  echo "ok   default state file lies in the private state folder (0700)"; else echo "FAIL default state folder"; fails=$((fails + 1)); fi
victim="$tmp/victim"; : >"$victim"; rm -f "$xs/freesynergy/umbenennen-fs-wiki-fs-text-runde"
ln -s "$victim" "$xs/freesynergy/umbenennen-fs-wiki-fs-text-runde"
out="$(XDG_STATE_HOME="$xs" "${tool[@]}" after "${nostate[@]}" 2>&1)"; rc=$?
if [ "$rc" -eq 2 ] && [ ! -s "$victim" ]; then echo "ok   a symlinked state file is refused (rc 2) and the target stays untouched"; else
  echo "FAIL symlink state: rc=$rc"; fails=$((fails + 1)); fi

if [ "$fails" -eq 0 ]; then echo "all ok"; else echo "$fails failed"; exit 1; fi
