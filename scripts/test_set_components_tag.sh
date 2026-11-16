#!/usr/bin/env bash
# Self test of scripts/set_components_tag.py for the V1+ lift (ADR 0092/0093): fixture like fs-maps (workspace root + 3 crates, `fs-types` over the repo
# fs-interfaces in every form: workspace table, `.workspace = true`, `[dependencies.x]` table, `package =` alias with rev).
#  - lifting fs-interfaces NEVER touches fs-types (name or package), but moves the real fs-interfaces entries;
#  - `--move fs-types <tag>` puts every fs-types entry on github.com/FreeSynergy/fs-types at the tag (rev removed), leaves fs-interfaces alone,
#    and afterwards no fs-types points at fs-interfaces; idempotent; another URL (a fork, a rev pin elsewhere) is an error and nothing is written;
#  - mutants (protection removed, URL check removed, URL not replaced, alias ignored) must turn the suite red.
#   bash scripts/test_set_components_tag.sh
set -uo pipefail
here="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
tool="$here/set_components_tag.py"
mkdir -p "$here/../.test-tmp"; tmp="$(TMPDIR="${TEST_TMPDIR:-$here/../.test-tmp}" mktemp -d)"  # never /tmp (RAM, rule [PLATTE])
trap 'rm -rf "$tmp"; rmdir "$here/../.test-tmp" 2>/dev/null' EXIT
fails=0
ok() { echo "ok   $1"; }
bad() { echo "FAIL $1"; fails=$((fails + 1)); }
IFC='https://github.com/FreeSynergy/fs-interfaces'
TYP='https://github.com/FreeSynergy/fs-types'

# mkfx <dir>: the fs-maps-like workspace
mkfx() {
  mkdir -p "$1/crates/a" "$1/crates/b" "$1/crates/c"
  cat >"$1/Cargo.toml" <<TOML
[workspace]
members = ["crates/*"]
[workspace.dependencies]
fs-types = { git = "$IFC", tag = "v0.0.2" }
fs-interfaces = { git = "$IFC", tag = "v0.0.14" }
TOML
  printf '[package]\nname = "a"\nversion = "0.0.0"\n[dependencies]\nfs-types.workspace = true\nfs-interfaces.workspace = true\n' >"$1/crates/a/Cargo.toml"
  cat >"$1/crates/b/Cargo.toml" <<TOML
[package]
name = "b"
version = "0.0.0"
[dependencies.fs-types]
git = "$IFC"
tag = "v0.0.2"
features = ["x"]
[dependencies.fs-interfaces]
git = "$IFC"
tag = "v0.0.14"
TOML
  cat >"$1/crates/c/Cargo.toml" <<TOML
[package]
name = "c"
version = "0.0.0"
[dependencies]
types = { package = "fs-types", git = "$IFC", rev = "abc1234" }
ifc = { package = "fs-interfaces", git = "$IFC", tag = "v0.0.14" }
serde = "1"
TOML
}

# facts <dir>: every git dependency as "<file>:<name or package>=<url>@<tag|rev>" (all forms, parsed by tomllib)
facts() {
  python3 -I - "$1" <<'PY'
import sys, tomllib
from pathlib import Path
root = Path(sys.argv[1])
def tables(d):
    for k, v in d.items():
        if k in ("dependencies", "dev-dependencies", "build-dependencies"): yield v
    if "workspace" in d: yield from tables(d["workspace"])
for f in sorted(root.rglob("Cargo.toml")):
    d = tomllib.loads(f.read_text())
    for t in tables(d):
        for n, s in sorted(t.items()):
            if isinstance(s, dict) and "git" in s:
                print(f"{f.relative_to(root)}:{s.get('package', n)}={s['git']}@{s.get('tag') or 'rev:' + s.get('rev', '?')}")
PY
}

suite() { # <script> -> runs every case; uses $fails
  local t="$1" d="$tmp/w" before after rc
  rm -rf "$d"; mkfx "$d"; before="$(facts "$d")"
  python3 -I "$t" v0.0.27 "$d" fs-interfaces >"$tmp/o" 2>&1; rc=$?
  after="$(facts "$d")"
  [ "$rc" = 0 ] && ok "fs-interfaces lift rc 0" || bad "fs-interfaces lift rc=$rc: $(cat "$tmp/o")"
  [ "$(grep -c 'fs-types=.*fs-interfaces@v0.0.2$' <<<"$after")" = 2 ] && grep -q "crates/c/Cargo.toml:fs-types=$IFC@rev:abc1234" <<<"$after" \
    && ok "fs-interfaces lift leaves every fs-types entry (inline, table, alias with rev) as it was" || bad "fs-types touched by the fs-interfaces lift: $after"
  [ "$(grep -c "fs-interfaces=$IFC@v0.0.27" <<<"$after")" = 3 ] && ok "the 3 fs-interfaces entries are on v0.0.27" || bad "fs-interfaces entries: $after"
  # only fs-types over fs-interfaces: the fs-interfaces lift finds no dependency (rc 3) and writes nothing
  rm -rf "$d"; mkdir -p "$d"; printf '[package]\nname = "o"\nversion = "0"\n[dependencies]\nfs-types = { git = "%s", tag = "v0.0.2" }\n' "$IFC" >"$d/Cargo.toml"
  python3 -I "$t" v0.0.27 "$d" fs-interfaces >"$tmp/o" 2>&1; rc=$?
  [ "$rc" = 3 ] && grep -q 'v0.0.2' "$d/Cargo.toml" && ok "only fs-types over fs-interfaces: rc 3, untouched" || bad "only fs-types: rc=$rc"

  rm -rf "$d"; mkfx "$d"
  python3 -I "$t" --move fs-types v0.0.1 "$d" >"$tmp/o" 2>&1; rc=$?
  after="$(facts "$d")"
  [ "$rc" = 0 ] && ok "--move fs-types rc 0" || bad "--move rc=$rc: $(cat "$tmp/o")"
  [ "$(grep -c "fs-types=$TYP@v0.0.1$" <<<"$after")" = 3 ] && ok "all 3 fs-types entries (workspace, table, alias) on the fs-types repo at v0.0.1, rev gone" || bad "after --move: $after"
  grep -q 'fs-types=.*fs-interfaces' <<<"$after" && bad "a fs-types still points at fs-interfaces" || ok "no fs-types points at fs-interfaces any more"
  [ "$(grep -c "fs-interfaces=$IFC@v0.0.14" <<<"$after")" = 3 ] && ok "--move leaves the 3 fs-interfaces entries alone" || bad "fs-interfaces touched: $after"
  grep -q 'features = \["x"\]' "$d/crates/b/Cargo.toml" && grep -q 'fs-types.workspace = true' "$d/crates/a/Cargo.toml" && ok "other fields and \`.workspace = true\` stay" || bad "layout lost"
  python3 -I "$t" --move fs-types v0.0.1 "$d" >"$tmp/o" 2>&1; rc=$?
  [ "$rc" = 0 ] && grep -q 'already on v0.0.1' "$tmp/o" && [ "$(facts "$d")" = "$after" ] && ok "second --move run: already on the tag, nothing changed" || bad "idempotence: rc=$rc $(cat "$tmp/o")"
  python3 -I "$t" --move fs-types v0.0.2 "$d" >/dev/null 2>&1
  [ "$(facts "$d" | grep -c "fs-types=$TYP@v0.0.2$")" = 3 ] && ok "--move on an already moved repo raises the tag" || bad "tag raise after move"
  python3 -I "$t" v0.0.28 "$d" fs-interfaces >/dev/null 2>&1
  [ "$(facts "$d" | grep -c "fs-types=$TYP@v0.0.2$")" = 3 ] && ok "a later fs-interfaces lift keeps fs-types on its own repo" || bad "fs-interfaces lift moved fs-types"

  # a foreign URL (fork with rev pin): error, nothing written, in ANY crate
  rm -rf "$d"; mkfx "$d"; printf '[package]\nname = "z"\nversion = "0"\n[dependencies]\nfs-types = { git = "https://example.org/x/fs-types", rev = "deadbee" }\n' >"$d/crates/c/Cargo.toml"
  before="$(facts "$d")"
  python3 -I "$t" --move fs-types v0.0.1 "$d" >"$tmp/o" 2>&1; rc=$?
  [ "$rc" = 1 ] && grep -q 'not guessing' "$tmp/o" && [ "$(facts "$d")" = "$before" ] && ok "foreign fs-types URL: rc 1, nothing written" || bad "foreign URL: rc=$rc $(cat "$tmp/o")"
  # a [patch] on fs-interfaces hides fs-types too
  rm -rf "$d"; mkfx "$d"; printf '[patch."%s"]\nfs-types = { path = "../x" }\n' "$IFC" >>"$d/Cargo.toml"
  python3 -I "$t" --move fs-types v0.0.1 "$d" >/dev/null 2>&1; rc=$?
  [ "$rc" = 1 ] && ok "[patch] on fs-interfaces: rc 1" || bad "patch: rc=$rc"
  python3 -I "$t" --move fs-types >/dev/null 2>&1; rc=$?
  [ "$rc" = 2 ] && ok "--move without tag and root: rc 2" || bad "usage rc=$rc"
}

suite "$tool"
echo "-- mutants (each must turn the suite red)"
mutant() { # <label> <sed expression>
  local m="$tmp/mut.py" before="$fails"
  sed -E "$2" "$tool" >"$m"
  if cmp -s "$m" "$tool"; then bad "mutant '$1' did not change the script"; return; fi
  suite "$m" >"$tmp/mut.out" 2>&1
  if [ "$fails" -gt "$before" ]; then ok "mutant '$1' turns the suite red"; fails="$before"; else bad "mutant '$1' stayed green"; fi
}
mutant "fs-types protection removed" 's/^(PROTECT = None.*)$/\1/; s/PROTECT = "fs-types" if REPO == "fs-interfaces" else None/PROTECT = None/'
mutant "foreign URL accepted under --move" 's/and old_or_new else None/else None/'
mutant "URL not replaced under --move" 's|return f"\{HOME\}/\{MOVE\}" if named|return url if named|'
mutant "package alias ignored" 's/return lib is not None and \(pkg or name\) == lib/return lib is not None and name == lib/'
mutant "rev/branch not removed (inline form)" '/^    line = re\.sub\(rf"/d'
[ "$fails" -eq 0 ] && echo "all ok" || { echo "$fails failure(s)"; exit 1; }
