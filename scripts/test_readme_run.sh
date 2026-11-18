#!/usr/bin/env bash
# Self test of scripts/readme-run.sh with a STUB cargo (no network, no build): a good README is green, every mutant of the README or of the
# script is red (rc 2 = refused, rc 1 = a command failed). A mutant that is not applied counts as a failure.
#   bash scripts/test_readme_run.sh
set -uo pipefail
here="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
repo="$(cd "$here/.." && pwd)"
tmp="$(mktemp -d)"
trap 'rm -rf "$tmp"' EXIT
fails=0
ok() { echo "ok   $1"; }
bad() { echo "FAIL $1"; fails=$((fails + 1)); }

# the stub: `cargo install` makes a stub fs-blueprint; both refuse an argument that ends in CR; the stub fs-blueprint logs its words
mkdir -p "$tmp/stub"
cat >"$tmp/stub/cargo" <<'STUB'
#!/usr/bin/env bash
for a in "$@"; do case "$a" in *$'\r') echo "stub cargo: CR in argument" >&2; exit 9 ;; esac; done
[ "$1" = install ] || exit 3
mkdir -p "$CARGO_INSTALL_ROOT/bin"
cat >"$CARGO_INSTALL_ROOT/bin/fs-blueprint" <<'FB'
#!/usr/bin/env bash
for a in "$@"; do case "$a" in *$'\r') echo "stub fs-blueprint: CR in argument" >&2; exit 9 ;; esac; done
case "$1" in
  new) mkdir -p "$4" ;;
  check) [ -d "$3" ] || exit 1 ;;
  *) exit 3 ;;
esac
FB
chmod +x "$CARGO_INSTALL_ROOT/bin/fs-blueprint"
STUB
chmod +x "$tmp/stub/cargo"

ver="$(sed -nE 's/^version = "([^"]+)"$/\1/p' "$repo/crates/fs-blueprint/Cargo.toml" | head -1)"
[ -n "$ver" ] || { echo "FAIL no crate version"; exit 1; }

# tree <name> <script-sed> <readme-sed>: a copy tree with the script and the README (each changed by a sed expression, "" = unchanged)
tree() {
  local d="$tmp/$1"
  mkdir -p "$d/scripts" "$d/examples" "$d/crates/fs-blueprint" "$d/tmp"
  cp -R "$repo/examples/leer" "$d/examples/leer"
  cp "$repo/crates/fs-blueprint/Cargo.toml" "$d/crates/fs-blueprint/Cargo.toml"
  if [ -n "$2" ]; then sed -E "$2" "$repo/scripts/readme-run.sh" >"$d/scripts/readme-run.sh"; else cp "$repo/scripts/readme-run.sh" "$d/scripts/readme-run.sh"; fi
  if [ -n "$3" ]; then sed -E "$3" "$repo/README.md" >"$d/README.md"; else cp "$repo/README.md" "$d/README.md"; fi
  if [ -n "$2" ] && cmp -s "$d/scripts/readme-run.sh" "$repo/scripts/readme-run.sh"; then return 1; fi
  if [ -n "$3" ] && cmp -s "$d/README.md" "$repo/README.md"; then return 1; fi
  return 0
}
runit() { # <name>: prints rc
  (cd "$tmp/$1" && PATH="$tmp/stub:$PATH" TMPDIR="$tmp/$1/tmp" bash scripts/readme-run.sh README.md >"$tmp/$1/out.log" 2>&1 </dev/null; echo $?)
}
want() { # <name> <rc wanted> <script-sed> <readme-sed> [pre-step: a function name applied to the README]
  local name=$1 w=$2
  if ! tree "$name" "$3" "$4"; then bad "mutant '$name' changed nothing (NOT APPLIED)"; return; fi
  [ -z "${5:-}" ] || "$5" "$tmp/$name/README.md"
  local rc
  rc=$(runit "$name")
  if [ "$rc" = "$w" ]; then ok "$name: rc $rc as wanted"; else bad "$name: rc $rc, wanted $w (see $tmp/$name/out.log)"; sed 's/^/     /' "$tmp/$name/out.log" | tail -5; fi
}
crlf() { sed -i 's/$/\r/' "$1"; }
stdin_reader() { sed -i 's|^cd mein-plan$|read -r _\ncd mein-plan|' "$1"; }

want good 0 "" ""
[ -z "$(ls -A "$tmp/good/tmp" 2>/dev/null)" ] && ok "good: the work folder is removed" || bad "good: the work folder stays: $(ls "$tmp/good/tmp")"
want crlf-readme 0 "" "" crlf
want no-marker 2 "" 's/^<!-- readme-run -->$/<!-- x -->/'
want no-locked 2 "" 's/ --locked fs-blueprint/ fs-blueprint/'
want locked-in-comment 2 "" 's/ --locked fs-blueprint/ fs-blueprint # --locked/'
want locked-part-of-word 2 "" 's/ --locked fs-blueprint/ --lockedx fs-blueprint/'
want second-install-no-locked 2 "" 's/^cd mein-plan$/cargo install --git https:\/\/example.org\/x --tag v1.0.0 fs-blueprint\ncd mein-plan/'
want wrong-tag 2 "" 's/--tag v[0-9.]+ /--tag v9.9.9 /'
want no-tag 2 "" 's/ --tag v[0-9.]+//'
want false-in-the-middle 1 "" 's/^cd mein-plan$/cd mein-plan\nfalse/'
want stdin-reader 1 "" 's/^cd mein-plan$/cd mein-plan\nread -r _\nfalse/'
want or-true-on-check 2 "" 's/^(fs-blueprint check --repo fs-leer)$/\1 || true/'
want set-plus-e 2 "" 's/^cd mein-plan$/set +e\ncd mein-plan/'
want check-wrong-folder 1 "" 's/check --repo fs-leer/check --repo fs-leeer/'
want new-without-out-value 1 "" 's/--out fs-leer/--out/'

# the kept folder on request and on red
tree keep "" "" && rc=$(cd "$tmp/keep" && PATH="$tmp/stub:$PATH" TMPDIR="$tmp/keep/tmp" READMERUN_KEEP=1 bash scripts/readme-run.sh README.md >/dev/null 2>&1 </dev/null; echo $?)
[ "${rc:-x}" = 0 ] && [ -n "$(ls -A "$tmp/keep/tmp")" ] && ok "READMERUN_KEEP=1 keeps the work folder" || bad "READMERUN_KEEP=1 did not keep it"

if [ "$fails" -eq 0 ]; then echo "all ok"; else echo "$fails failed"; exit 1; fi
