#!/usr/bin/env bash
# Runs the commands of the README WORD FOR WORD, so that the instructions never differ from the tool.
#   bash scripts/readme-run.sh [README.md]
# The commands are the lines of the code block that follows the marker line `<!-- readme-run -->`. They run in ONE bash (so `cd` carries on),
# in an empty work folder that holds a copy of `examples/` (the README says `cp -r examples/leer ...`); nothing is installed globally:
# CARGO_INSTALL_ROOT points `cargo install` into the work folder and its `bin/` comes first in PATH. rc 0 = every line ran, else the first failing rc.
# Works in Git Bash on Windows. Refuses (rc 2) a README without the marker, with an empty block, or whose install line lacks `--locked`.
set -euo pipefail

root=$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)
readme=${1:-$root/README.md}
[ -f "$readme" ] || { echo "readme-run: no README at $readme" >&2; exit 2; }

cmds=$(awk '
  /^<!-- readme-run -->[[:space:]]*$/ { want = 1; next }
  want == 1 && /^```/ { want = 2; next }
  want == 2 && /^```/ { exit }
  want == 2 { print }
' "$readme" | tr -d '\r')
if [ -z "$(printf '%s' "$cmds" | tr -d '[:space:]')" ]; then
  echo "readme-run: no command block after the marker <!-- readme-run --> in $readme" >&2
  exit 2
fi
if ! printf '%s\n' "$cmds" | grep -Eq '^cargo install .*--locked'; then
  echo "readme-run: the install line must be a 'cargo install' with --locked" >&2
  exit 2
fi

work=$(mktemp -d "${TMPDIR:-/tmp}/readme-run.XXXXXX")
mkdir -p "$work/examples"
cp -R "$root/examples/leer" "$work/examples/leer"
export CARGO_INSTALL_ROOT="$work/inst"
export PATH="$work/inst/bin:$PATH"
echo "readme-run: work folder $work"
printf '%s\n' "$cmds" | sed 's/^/readme-run: + /'
rc=0
(cd "$work" && printf '%s\n' "$cmds" | bash -e -x) || rc=$?
echo "readme-run: rc=$rc (work folder $work stays, remove it yourself)"
exit "$rc"
