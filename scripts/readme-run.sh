#!/usr/bin/env bash
# Runs the commands of the README WORD FOR WORD, so that the instructions never differ from the tool.
#   bash scripts/readme-run.sh [README.md]
# The commands are the lines of the code block that follows the marker line `<!-- readme-run -->`. The block is written to a FILE (CR of a Windows
# checkout removed) and run by ONE `bash -e -x` with a closed stdin (a line that reads stdin cannot swallow the next lines), in an empty work folder
# that holds a copy of `examples/` (the README starts in a checkout of this repo). Nothing is installed globally: CARGO_INSTALL_ROOT points
# `cargo install` into the work folder and its `bin/` comes first in PATH. rc 0 = every line ran, else the first failing rc; rc 2 = the README is
# refused: no marker, empty block, a `cargo install` line without the ARGUMENT --locked (a comment does not count), or a line that hides a failure (`||`, `set +e`, `; true`), or an install tag that is not
# `v` + the version of crates/fs-blueprint/Cargo.toml (the instructions name the tool of this version). Works in Git Bash on Windows.
# The work folder is removed at the end when it was green (READMERUN_KEEP=1 keeps it; a red run keeps it for the log).
set -euo pipefail

root=$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)
readme=${1:-$root/README.md}
refuse() { echo "readme-run: $1" >&2; exit 2; }
[ -f "$readme" ] || refuse "no README at $readme"

cmds=$(awk '
  /^<!-- readme-run -->[[:space:]]*$/ { want = 1; next }
  want == 1 && /^```/ { want = 2; next }
  want == 2 && /^```/ { exit }
  want == 2 { print }
' "$readme" | tr -d '\r')
[ -n "$(printf '%s' "$cmds" | tr -d '[:space:]')" ] || refuse "no command block after the marker <!-- readme-run --> in $readme"

if printf '%s\n' "$cmds" | grep -Eq '\|\||set \+e|; *true( |$)'; then refuse "the block hides a failure (|| / set +e / ; true): every line must be able to fail"; fi
installs=$(printf '%s\n' "$cmds" | grep -E '^[[:space:]]*cargo install ' || true)
[ -n "$installs" ] || refuse "the block has no 'cargo install' line"
while IFS= read -r line; do
  code=${line%% \#*} # a comment is not an argument
  case " $code " in
    *" --locked "*) ;;
    *) refuse "the install line lacks the argument --locked: $line" ;;
  esac
  tag=$(printf '%s\n' "$code" | sed -nE 's/.* --tag ([^ ]+).*/\1/p')
  [ -n "$tag" ] || refuse "the install line has no --tag: $line"
  if [ -f "$root/crates/fs-blueprint/Cargo.toml" ]; then
    ver=$(sed -nE 's/^version = "([^"]+)"$/\1/p' "$root/crates/fs-blueprint/Cargo.toml" | head -1)
    [ "$tag" = "v$ver" ] || refuse "the install tag $tag is not v$ver (crates/fs-blueprint/Cargo.toml)"
  fi
done <<<"$installs"

tmproot=${TMPDIR:-/tmp}
tmproot=${tmproot%/}
work=$(mktemp -d "$tmproot/readme-run.XXXXXX")
# Check function: only our own work folder (a directory named readme-run.* directly in TMPDIR, no link) is ever removed.
remove_own_work() {
  local d=$1
  case "$d" in */readme-run.??????) ;; *) echo "readme-run: refuse to remove $d" >&2; return 1 ;; esac
  [ -d "$d" ] && [ ! -L "$d" ] || return 1
  [ "$(dirname "$d")" = "$tmproot" ] || return 1
  rm -rf -- "$d"
}
mkdir -p "$work/examples"
cp -R "$root/examples/leer" "$work/examples/leer"
printf '%s\n' "$cmds" >"$work/.cmds.sh"
export CARGO_INSTALL_ROOT="$work/inst"
export PATH="$work/inst/bin:$PATH"
echo "readme-run: work folder $work"
sed 's/^/readme-run: + /' "$work/.cmds.sh"
rc=0
(cd "$work" && bash -e -x .cmds.sh </dev/null) || rc=$?
if [ "$rc" = 0 ] && [ -z "${READMERUN_KEEP:-}" ]; then
  remove_own_work "$work" && echo "readme-run: rc=0 (work folder removed)"
else
  echo "readme-run: rc=$rc (work folder $work kept)"
fi
exit "$rc"
