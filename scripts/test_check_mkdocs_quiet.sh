#!/usr/bin/env bash
# Self test of scripts/check-mkdocs-quiet: the real tree is clean; planted quiet calls (shell, YAML run:, Python list, --quiet) are red; harmless lines
# (grep -q after mkdocs, -q before mkdocs, a call without -q) are not; a mutation (pattern emptied) turns the planted cases red.
#   bash scripts/test_check_mkdocs_quiet.sh
set -uo pipefail
here="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
tool="$here/check-mkdocs-quiet"
tmp="$(mktemp -d)"
trap 'rm -rf "$tmp"' EXIT
fails=0
ok() { echo "ok   $1"; }
bad() { echo "FAIL $1"; fails=$((fails + 1)); }
q="-""q"

python3 -I "$tool" >/dev/null 2>&1 && ok "the real tree has no quiet mkdocs call" || bad "the real tree has a quiet mkdocs call"

cases() { # <tool> <label>
  local t="$1" l="$2" line r n=0
  while IFS= read -r line; do
    n=$((n + 1)); rm -rf "$tmp/c"; mkdir -p "$tmp/c"; printf '%s\n' "$line" >"$tmp/c/x"
    python3 -I "$t" "$tmp/c" >"$tmp/o" 2>&1; r=$?
    if [ "$r" -eq 1 ] && grep -q 'x:1:' "$tmp/o"; then echo "ok   $l: red '$line'"; else echo "FAIL $l: rc=$r for '$line'"; fi
  done <<EOF
mkdocs build --strict $q -f mkdocs.yml
        run: mkdocs build $q --strict
["mkdocs", "build", "--strict", "$q", "-f", x]
mkdocs build --quiet --strict
/x/.venv/bin/mkdocs build --strict $q
EOF
}
out="$(cases "$tool" real)"; grep '^ok' <<<"$out"; grep -q '^FAIL' <<<"$out" && { grep '^FAIL' <<<"$out"; fails=$((fails + 1)); }

mkdir -p "$tmp/h"
printf '%s\n' 'mkdocs build --strict -f mkdocs.yml' "mkdocs --version | grep $q x" "grep $q WARNING build.log; mkdocs build" "echo $q mkdocs" >"$tmp/h/ok.sh"
python3 -I "$tool" "$tmp/h" >/dev/null 2>&1 && ok "harmless lines are not red" || bad "a harmless line was red"
python3 -I "$tool" "$tmp/none" >/dev/null 2>&1; [ $? -eq 2 ] && ok "a missing path is rc 2" || bad "missing path not rc 2"

cp "$tool" "$tmp/mutant"
sed -i 's/^QUIET = re.compile(.*/QUIET = re.compile(r"(?!x)x")/' "$tmp/mutant"
cmp -s "$tmp/mutant" "$tool" && bad "mutation changed nothing"
mout="$(cases "$tmp/mutant" mut)"
if grep -q '^FAIL' <<<"$mout"; then ok "mutation (pattern removed) turns the planted cases red"; else bad "mutation stayed green"; fi

if [ "$fails" -eq 0 ]; then echo "all ok"; else echo "$fails failed"; exit 1; fi
