#!/usr/bin/env bash
# Pattern: none (script of the pin cascade).
# shellcheck disable=SC2154,SC2034  # $tmp and $here come from the sourcing test; ifc_head, rev_a, rev_b are read by it
# Shared fixture of the self tests of the pin-kaskade scripts (source it, do not run it): local bare repos as the "remote" (FS_KASKADE_BASE=file://...),
# so no test touches the network. Expects: $here (scripts dir), $tmp (a temp dir), the helpers ok/bad/chk of the caller.
#   fs-types      tag v0.0.3 (the trigger)
#   fs-interfaces tag v0.0.2: Cargo.toml + Cargo.lock on fs-types v0.0.3, `.github/workflows/windows.yml` calls _windows-check.yml@v0.0.11
#   fs-objects    depends on fs-types; (no tag at first, no windows caller)
#   fs-people     tag v0.0.2 = commit A, a later commit B on main (so a `rev` of B is on no tag)
#   fs-tasks      depends on fs-interfaces (tag), fs-types THROUGH the repo fs-interfaces (wrong source, the lift moves it), fs-people-directory by `rev` A,
#                 and blinc_app from the foreign source FreeSynergy/blinc-app-fix by `rev`
#   fs-release    tag v0.0.11
export GIT_AUTHOR_NAME=t GIT_AUTHOR_EMAIL=t@t GIT_COMMITTER_NAME=t GIT_COMMITTER_EMAIL=t@t
origin="$tmp/origin"
export FS_KASKADE_BASE="file://$origin"
GH="https://github.com/FreeSynergy"

# newrepo <name>: a bare repo "origin/<name>" and a work clone "work/<name>" on main
newrepo() {
  git init -q --bare -b main "$origin/$1"
  git init -q -b main "$tmp/work/$1"
  git -C "$tmp/work/$1" remote add origin "$origin/$1"
}
# commit_push <name> <message>: commit everything and push main
commit_push() { git -C "$tmp/work/$1" add -A && git -C "$tmp/work/$1" commit -qm "$2" && git -C "$tmp/work/$1" push -q origin main 2>/dev/null; }
# tag_push <name> <tag> [rev]
tag_push() { git -C "$tmp/work/$1" tag -a "$2" -m "$2" "${3:-HEAD}" && git -C "$tmp/work/$1" push -q origin "$2" 2>/dev/null; }
# lock <fs-types source tag or repo> -> a Cargo.lock with fs-types from the given git source (+ the workspace member)
lock() { # <member> <source>
  printf 'version = 4\n\n[[package]]\nname = "%s"\nversion = "0.1.0"\ndependencies = [\n "fs-types",\n]\n\n[[package]]\nname = "fs-types"\nversion = "0.0.3"\nsource = "%s"\n' "$1" "$2"
}

mkdir -p "$origin" "$tmp/work"
newrepo fs-types
printf '[workspace]\nmembers = []\n' >"$tmp/work/fs-types/Cargo.toml"
commit_push fs-types one; tag_push fs-types v0.0.3

newrepo fs-interfaces
printf '[package]\nname = "fs-interfaces"\nversion = "0.0.2"\n[dependencies]\nfs-types = { git = "%s/fs-types", tag = "v0.0.3" }\n' "$GH" >"$tmp/work/fs-interfaces/Cargo.toml"
lock fs-interfaces "git+$GH/fs-types?tag=v0.0.3#aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa" >"$tmp/work/fs-interfaces/Cargo.lock"
printf "strict = true\nonly_from_enabled = true\n" >"$tmp/work/fs-interfaces/.fs-lock-sources.toml"
mkdir -p "$tmp/work/fs-interfaces/.github/workflows"
printf 'name: windows\njobs:\n  windows:\n    uses: FreeSynergy-Net/fs-release/.github/workflows/_windows-check.yml@v0.0.11\n' >"$tmp/work/fs-interfaces/.github/workflows/windows.yml"
commit_push fs-interfaces one; tag_push fs-interfaces v0.0.2
ifc_head="$(git -C "$tmp/work/fs-interfaces" rev-parse HEAD)"

newrepo fs-objects
printf '[package]\nname = "fs-objects"\nversion = "0.0.2"\n[dependencies]\nfs-types = { git = "%s/fs-types", tag = "v0.0.3" }\n' "$GH" >"$tmp/work/fs-objects/Cargo.toml"
commit_push fs-objects one

newrepo fs-people
mkdir -p "$tmp/work/fs-people/crates/fs-people-directory"
printf '[package]\nname = "fs-people"\nversion = "0.0.2"\n[dependencies]\nfs-interfaces = { git = "%s/fs-interfaces", tag = "v0.0.2" }\n' "$GH" >"$tmp/work/fs-people/Cargo.toml"
printf '[package]\nname = "fs-people-directory"\nversion = "0.0.2"\n' >"$tmp/work/fs-people/crates/fs-people-directory/Cargo.toml"
commit_push fs-people A; tag_push fs-people v0.0.2
rev_a="$(git -C "$tmp/work/fs-people" rev-parse --short=8 HEAD)"
echo more >"$tmp/work/fs-people/more.txt"; commit_push fs-people B
rev_b="$(git -C "$tmp/work/fs-people" rev-parse --short=8 HEAD)"

# tasks_toml <rev of fs-people-directory>
tasks_toml() {
  printf '[package]\nname = "fs-tasks"\nversion = "0.0.2"\n[dependencies]\nfs-interfaces = { git = "%s/fs-interfaces", tag = "v0.0.2" }\nfs-types = { git = "%s/fs-interfaces", tag = "v0.0.22" }\nfs-people-directory = { git = "%s/fs-people", rev = "%s" }\n[dev-dependencies]\nblinc_app = { git = "%s/blinc-app-fix", rev = "5180d4eae80034d851d98464861f098d69eceaf0" }\n' "$GH" "$GH" "$GH" "$1" "$GH"
}
newrepo fs-tasks
tasks_toml "$rev_a" >"$tmp/work/fs-tasks/Cargo.toml"
lock fs-tasks "git+$GH/fs-interfaces?tag=v0.0.22#bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb" >"$tmp/work/fs-tasks/Cargo.lock"
commit_push fs-tasks one

newrepo fs-release
printf 'x\n' >"$tmp/work/fs-release/README"
commit_push fs-release one; tag_push fs-release v0.0.11

# the Glieder input in a deliberately WRONG order (fs-tasks first, fs-people before its dependency fs-interfaces): the plan must derive the order from the pins
glieder_json() {
  printf '[{"repo":"fs-tasks","art":"programm","tag":"v0.0.2","allow":"x"},{"repo":"fs-people","art":"programm","tag":"v0.0.2","allow":"x"},{"repo":"fs-interfaces","art":"bibliothek","tag":"v0.0.2","allow":"x","besonderes":"capnp"},{"repo":"fs-objects","art":"bibliothek","tag":"v0.0.2","allow":"x"}]'
}
# q <json file> <python expression over d>
q() { python3 -I -c 'import json,sys; d=json.load(open(sys.argv[1])); print(eval(sys.argv[2]))' "$1" "$2"; }
# mutate <script> <old> <new> <out>: a copy with ONE exact text replaced; fails when the text is not found (a mutation that changes nothing is a test error)
mutate() {
  python3 -I - "$@" <<'PY'
import sys
src, old, new, out = sys.argv[1:5]
t = open(src, encoding="utf-8").read()
if old not in t:
    sys.exit(f"mutation text not found in {src}: {old!r}")
open(out, "w", encoding="utf-8").write(t.replace(old, new, 1))
PY
}
# the mutants run from a copy of the scripts dir (the library stays next to them)
mkdir -p "$tmp/mut"; cp -r "$here/kaskade_lib" "$here/check_lock_sources.py" "$here/gates-lock.sh" "$here/lock-sources.toml" "$tmp/mut/"
mkdir -p "$tmp/workflows"; cp "$here/../workflows/kaskade-fremdquellen.txt" "$tmp/workflows/"
