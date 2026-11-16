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
  printf '[[package]]\nid = "fs-wiki"\nkind = "program"\n' >"$1/fs-catalog/packages.toml"
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
printf '[[package]]\nid = "fs-text"\n' >>"$w/fs-catalog/packages.toml"
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
printf '[[package]]\nid = "fs-text"\nreplaces = ["fs-wiki"]\n' >"$v/fs-catalog/packages.toml"
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

printf '[[package]]\nid = "fs-text"\n' >"$v/fs-catalog/packages.toml"
# shellcheck disable=SC2046
expect 1 "has no replaces" "red: replaces missing" -- after $(args "$v")
printf '[[package]]\nid = "fs-text"\nreplaces = ["fs-wiki"]\n' >"$v/fs-catalog/packages.toml"

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

# --- A1: the same pair is idempotent only while the new name was not given up in the meantime
printf 'fs-a -> fs-b\nfs-b -> fs-c\n' >"$v/recycle.txt"
expect 1 "never given out again" "retire a -> b is refused when b was given up (b -> c)" -- retire --old fs-a --new fs-b --retired "$v/recycle.txt"
printf 'fs-a -> fs-x\nfs-a -> fs-b\n' >"$v/twice.txt"
expect 1 "already retired" "retire a -> b is refused when the list has another entry for a" -- retire --old fs-a --new fs-b --retired "$v/twice.txt"
if [ "$(wc -l <"$v/recycle.txt")" -eq 2 ] && [ "$(wc -l <"$v/twice.txt")" -eq 2 ]; then echo "ok   the refused retires wrote nothing"; else echo "FAIL a refused retire wrote"; fails=$((fails + 1)); fi
cp "$v/retired.txt" "$v/retired.keep"
printf 'fs-text -> fs-neu\n' >>"$v/retired.txt"
# shellcheck disable=SC2046
expect 1 "inconsistent" "after is red when the new name was given up since" -- after $(args "$v")
cp "$v/retired.keep" "$v/retired.txt"

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

# --- the catalog is read by `id` (not `name`): an entry with only a display name `fs-wiki` is no catalog hit
c="$tmp/w3"; mk "$c"
printf '[[package]]\nname = "fs-wiki"\n' >"$c/fs-catalog/packages.toml"
# shellcheck disable=SC2046
expect 0 "catalog: 0" "catalog by id: a display name fs-wiki is no entry" -- dry-run $(args "$c")
mk "$c"
# shellcheck disable=SC2046
expect 0 "catalog: 1" "catalog by id: id = fs-wiki is the entry (counter-check of the line above)" -- dry-run $(args "$c")

# --- more places: .desktop, Cargo.lock, source strings, FS_<OLD>_ROOT
d="$tmp/w4"; mk "$d"
mkdir -p "$d/fs-wiki/packaging" "$d/fs-wiki/src"
printf '[Desktop Entry]\nName=Wiki\nExec=fs-wiki %%U\n' >"$d/fs-wiki/packaging/net.freesynergy.Wiki.desktop"
printf '[[package]]\nname = "fs-wiki-core"\nversion = "0.0.10"\n' >"$d/fs-wiki/Cargo.lock"
printf 'const ROOT: &str = "FS_WIKI_ROOT";\nconst DIR: &str = ".local/share/fs-wiki";\nfn m() { eprintln!("fs-wiki: failed"); }\nuse fs_wiki_core::x;\nlet fs_wikipedia = 1;\n' >"$d/fs-wiki/src/main.rs"
# shellcheck disable=SC2046
expect 0 "desktop: 1" "dry run lists the .desktop file (Exec=)" -- dry-run $(args "$d")
# shellcheck disable=SC2046
expect 0 "lock: 1" "dry run lists Cargo.lock" -- dry-run $(args "$d")
# shellcheck disable=SC2046
expect 0 "source: 4" "dry run lists source strings (env, path, message, snake path), not fs_wikipedia" -- dry-run $(args "$d")
# shellcheck disable=SC2046
expect 1 "old name still in the source" "after: red for a source string" -- after $(args "$d")
# shellcheck disable=SC2046
expect 1 "old name still in the desktop file" "after: red for the .desktop file" -- after $(args "$d")
# shellcheck disable=SC2046
expect 1 "old name still in the lock" "after: red for Cargo.lock" -- after $(args "$d")

# --- keep (L2): kept with grund and bereich, the others stay open, stale patterns are red, bad input is rc 2
kj='[{"pattern":"FS_WIKI_ROOT","grund":"Datenpfad bleibt","bereich":"sonstiges"},{"pattern":"packaging/.*desktop Name|Exec=fs-wiki","grund":"Binary als Link","bereich":"PROGRAM"}]'
# shellcheck disable=SC2046
expect 0 "kept: 2" "keep: two hits kept on purpose" -- dry-run $(args "$d") --keep "$kj"
# shellcheck disable=SC2046
expect 0 "source: 3" "keep: the other source hits stay open" -- dry-run $(args "$d") --keep "$kj"
# shellcheck disable=SC2046
expect 0 "bereich=PROGRAM" "keep: bereich is in the result" -- dry-run $(args "$d") --keep "$kj"
# shellcheck disable=SC2046
expect 0 "grund=Binary als Link" "keep: grund is in the result" -- dry-run $(args "$d") --keep "$kj"
# shellcheck disable=SC2046
expect 1 "matches no hit" "keep: a stale pattern is red" -- dry-run $(args "$d") --keep '[{"pattern":"NOTHING_LIKE_THIS","grund":"x","bereich":"Typ"}]'
# shellcheck disable=SC2046
expect 2 "bereich" "keep: an unknown bereich is bad input" -- dry-run $(args "$d") --keep '[{"pattern":"a","grund":"x","bereich":"Zeug"}]'
# shellcheck disable=SC2046
expect 2 "--keep" "keep: not JSON is bad input" -- dry-run $(args "$d") --keep 'nope'
# the [[bin]] name line of a Cargo.toml is told apart from the [package] name line by its section
printf '[package]\nname = "fs-wiki"\n[[bin]]\nname = "fs-wiki"\n' >"$d/fs-wiki/Cargo.toml"
kb='[{"pattern":"\\[\\[bin\\]\\] name = \"fs-wiki\"","grund":"Binary als Link","bereich":"PROGRAM"}]'
js="$tmp/keep.json"
"${tool[@]}" dry-run $(args "$d") --keep "$kb" --json "$js" >/dev/null
if python3 -I -c 'import json,sys; d=json.load(open(sys.argv[1])); sys.exit(0 if len(d["plan"]["kept"])==1 and len([c for c in d["plan"]["crates"] if c["file"]=="Cargo.toml"])==1 else 1)' "$js"; then
  echo "ok   keep: only the [[bin]] line is kept, the [package] line stays a crate hit"; else echo "FAIL keep by section"; fails=$((fails + 1)); fi
# after: green only when every remaining hit is kept (everything else renamed)
e="$tmp/w5"; mk "$e"; rm -rf "$e/fs-wiki/crates/fs-wiki-core"
printf '[package]\nname = "fs-text"\n[[bin]]\nname = "fs-wiki"\n' >"$e/fs-wiki/Cargo.toml"
printf '[[package]]\nid = "fs-text"\nreplaces = ["fs-wiki"]\n' >"$e/fs-catalog/packages.toml"
printf 'x\n' >"$e/fs-documentation/docs/de/wiki.md"; printf 'fs-wiki -> fs-text\n' >"$e/retired.txt"
printf 'jobs: {}\n' >"$e/fs-wiki/.github/workflows/release.yml"; printf 'fs-text = 1\n' >"$e/fs-app/Cargo.toml"
# shellcheck disable=SC2046
expect 1 "crate 'fs-wiki' still named" "after: the bin name is open without keep" -- after $(args "$e")
# shellcheck disable=SC2046
expect 0 "kept: 1" "after: green when the only hit is kept" -- after $(args "$e") --keep "$kb"
# shellcheck disable=SC2046
expect 1 "stale entry" "after: a kept name that vanished is red" -- after $(args "$e") --keep '[{"pattern":"Exec=fs-wiki","grund":"x","bereich":"PROGRAM"}]'

# --- allow works in the dry run too
printf 'Siehe fs-wiki.\n' >"$d/fs-documentation/docs/de/wiki.md"
# shellcheck disable=SC2046
expect 0 "allowed: 1" "allow in the dry run: the history page is allowed" -- dry-run $(args "$d") --allow 'docs/de/wiki.md'
# shellcheck disable=SC2046
expect 0 "docs: 0" "allow in the dry run: no open docs hit" -- dry-run $(args "$d") --allow 'docs/de/wiki.md'

# --- git: only tracked files are hits, untracked ones are a warning; a repo outside --repos is searched on origin/main
g="$tmp/w6"; mk "$g"
gitc() { git -C "$1" -c user.name=t -c user.email=t@example.invalid "${@:2}"; }
mkrepo() { git init -q -b unreviewed/t "$1"; gitc "$1" add -A; gitc "$1" commit -q -m x; git -C "$1" update-ref refs/remotes/origin/main HEAD; }
printf 'fs-wiki-tmp\n' >"$g/fs-wiki/.gitignore.tmp"; rm "$g/fs-wiki/.gitignore.tmp"
mkrepo "$g/fs-wiki"
printf 'notes: fs-wiki\n' >"$g/fs-wiki/NOTES.txt"
printf 'build/\n' >"$g/fs-wiki/.gitignore"; mkdir -p "$g/fs-wiki/build"; printf 'fs-wiki\n' >"$g/fs-wiki/build/out.txt"
mkdir -p "$g/fs-other"; printf 'fs-wiki wird gebraucht\n' >"$g/fs-other/README.md"; mkrepo "$g/fs-other"
# shellcheck disable=SC2046
expect 0 "untracked: 1" "git: an untracked file naming the old name is only a warning" -- dry-run $(args "$g")
# shellcheck disable=SC2046
expect 0 "warn: 1 untracked" "git: the warning is printed" -- dry-run $(args "$g")
# shellcheck disable=SC2046
expect 0 "source: 0" "git: untracked and ignored files are not hits" -- dry-run $(args "$g")
# shellcheck disable=SC2046
expect 0 "outside: 1" "outside: fs-other on origin/main names the old name" -- dry-run $(args "$g")
# shellcheck disable=SC2046
expect 0 "repo=$g/fs-other" "outside: the repo and file are listed" -- dry-run $(args "$g")
# shellcheck disable=SC2046
expect 0 "kept: 1" "outside: kept by a pattern with the repo name" -- dry-run $(args "$g") --keep '[{"pattern":"fs-other README.md","grund":"gehoert dem anderen Repo","bereich":"sonstiges"}]'
# shellcheck disable=SC2046
expect 0 "outside: 0" "outside: --repos names the repo, so it is not searched twice" -- dry-run --old fs-wiki --new fs-text --repos "$g/fs-wiki" "$g/fs-other" --world "$g"
printf 'wieder fs-wiki\n' >"$g/fs-other/README.md"
# a change that is not on origin/main (not committed) does not count: the search reads origin/main
# shellcheck disable=SC2046
expect 0 "outside: 1" "outside: only origin/main counts, not the working tree" -- dry-run $(args "$g")

# --- rename (L1 scaffold): git mv of the crate folders, word boundary, keep lines skipped, idempotent, never on main
r="$tmp/w7"; mk "$r"; mkdir -p "$r/fs-wiki/crates/fs-wiki" "$r/fs-wiki/src"
printf '[package]\nname = "fs-wiki"\n[[bin]]\nname = "fs-wiki"\n' >"$r/fs-wiki/crates/fs-wiki/Cargo.toml"
printf 'use fs_wiki_core::x; // fs-wikipedia stays\nconst P: &str = "fs-wiki-core";\n' >"$r/fs-wiki/src/lib.rs"
printf '[[package]]\nname = "fs-wiki-core"\n' >"$r/fs-wiki/Cargo.lock"
printf 'siehe https://github.com/FreeSynergy/fs-wiki/releases und fs-wiki-core\n' >"$r/fs-wiki/README.md"
mkrepo "$r/fs-wiki"
printf 'dirty\n' >"$r/fs-wiki/untracked-but-tracked.txt"; git -C "$r/fs-wiki" add untracked-but-tracked.txt
expect 1 "uncommitted changes" "rename refuses a dirty tree" -- rename --old fs-wiki --new fs-text --repos "$r/fs-wiki"
git -C "$r/fs-wiki" -c user.name=t -c user.email=t@t commit -q -m dirty
rk='[{"pattern":"\\[\\[bin\\]\\] name = \"fs-wiki\"","grund":"Binary als Link","bereich":"PROGRAM"}]'
"${tool[@]}" rename --old fs-wiki --new fs-text --repos "$r/fs-wiki" --keep "$rk" >"$tmp/rename.out" 2>&1; rc=$?
if [ "$rc" -eq 0 ] && [ -d "$r/fs-wiki/crates/fs-text" ] && [ -d "$r/fs-wiki/crates/fs-text-core" ] && [ ! -d "$r/fs-wiki/crates/fs-wiki-core" ] \
  && grep -q 'use fs_text_core::x; // fs-wikipedia stays' "$r/fs-wiki/src/lib.rs" && grep -q '"fs-text-core"' "$r/fs-wiki/src/lib.rs" \
  && grep -q 'name = "fs-text-core"' "$r/fs-wiki/Cargo.lock" \
  && [ "$(grep -c 'name = "fs-text"' "$r/fs-wiki/crates/fs-text/Cargo.toml")" -eq 1 ] && grep -q '^name = "fs-wiki"' "$r/fs-wiki/crates/fs-text/Cargo.toml"; then
  echo "ok   rename: crates moved, words replaced, fs-wikipedia and the kept [[bin]] line untouched"; else echo "FAIL rename: rc=$rc"; cat "$tmp/rename.out"; fails=$((fails + 1)); fi
if grep -q 'github.com/FreeSynergy/fs-wiki/releases und fs-text-core' "$r/fs-wiki/README.md"; then echo "ok   rename: the pin URL stays until the GitHub rename"; else echo "FAIL rename touched the pin URL"; cat "$r/fs-wiki/README.md"; fails=$((fails + 1)); fi
git -C "$r/fs-wiki" add -A; git -C "$r/fs-wiki" -c user.name=t -c user.email=t@t commit -q -m rename
before="$(git -C "$r/fs-wiki" status --porcelain | sort | md5sum)"
expect 0 "nothing to do" "rename twice is a no-op" -- rename --old fs-wiki --new fs-text --repos "$r/fs-wiki" --keep "$rk"
if [ "$before" = "$(git -C "$r/fs-wiki" status --porcelain | sort | md5sum)" ]; then echo "ok   rename twice changes nothing"; else echo "FAIL rename twice changed files"; fails=$((fails + 1)); fi
# --pins: only the pin URLs, after the GitHub rename; --allow keeps a history page as it is
printf 'alt fs-wiki, github.com/FreeSynergy/fs-wiki\n' >"$r/fs-wiki/HIST.md"; git -C "$r/fs-wiki" add -A; git -C "$r/fs-wiki" -c user.name=t -c user.email=t@t commit -q -m hist
expect 0 "green" "rename --allow skips the history page" -- rename --old fs-wiki --new fs-text --repos "$r/fs-wiki" --keep "$rk" --allow HIST.md
if grep -q '^alt fs-wiki, github.com/FreeSynergy/fs-wiki$' "$r/fs-wiki/HIST.md"; then echo "ok   rename --allow: file untouched"; else echo "FAIL --allow ignored"; fails=$((fails + 1)); fi
git -C "$r/fs-wiki" add -A; git -C "$r/fs-wiki" -c user.name=t -c user.email=t@t commit -q -m x >/dev/null 2>&1
expect 0 "green" "rename --pins" -- rename --old fs-wiki --new fs-text --repos "$r/fs-wiki" --pins
if grep -q 'github.com/FreeSynergy/fs-text/releases' "$r/fs-wiki/README.md" && grep -q 'github.com/FreeSynergy/fs-text$' "$r/fs-wiki/HIST.md" && grep -q 'alt fs-wiki, ' "$r/fs-wiki/HIST.md" && [ -d "$r/fs-wiki/crates/fs-text" ]; then echo "ok   rename --pins: only the URLs"; else echo "FAIL --pins"; cat "$r/fs-wiki/HIST.md"; fails=$((fails + 1)); fi
git -C "$r/fs-wiki" add -A; git -C "$r/fs-wiki" -c user.name=t -c user.email=t@t commit -q -m pins
git -C "$r/fs-wiki" checkout -q -b main 2>/dev/null
expect 1 "only in a worktree branch" "rename refuses on main" -- rename --old fs-wiki --new fs-other --repos "$r/fs-wiki"

if [ "$fails" -eq 0 ]; then echo "all ok"; else echo "$fails failed"; exit 1; fi
