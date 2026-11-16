#!/usr/bin/env bash
# Self test of scripts/check-no-pkill: the real tree is clean; planted pattern kills (script, workflow `run:`, pipe, sudo, kill $(pgrep)) are red
# (rc 1, file:line named); prose that names the rule and a comment are not; a mutation (the pattern emptied) must turn the planted cases red.
#   bash scripts/test_check_no_pkill.sh
set -uo pipefail
here="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
tool="$here/check-no-pkill"
tmp="$(mktemp -d)"
trap 'rm -rf "$tmp"' EXIT
fails=0
ok() { echo "ok   $1"; }
bad() { echo "FAIL $1"; fails=$((fails + 1)); }
a="pk""ill"; b="kill""all"

python3 -I "$tool" >/dev/null 2>&1 && ok "the real tree has no pattern kill" || bad "the real tree has a pattern kill"

mkdir -p "$tmp/t/scripts" "$tmp/t/workflows"
check_cases() { # <tool> <label>
  local t="$1" l="$2" r
  while IFS='|' read -r file line; do
    rm -f "$tmp/t/scripts/"* "$tmp/t/workflows/"*
    printf '%s\n' "$line" >"$tmp/t/$file"
    python3 -I "$t" "$tmp/t" >"$tmp/o" 2>&1; r=$?
    if [ "$r" -eq 1 ] && grep -q "$file:1:" "$tmp/o"; then echo "ok   $l: red '$line'"; else echo "FAIL $l: rc=$r for '$line'"; fi
  done <<EOF
scripts/x.sh|$a -f cargo
scripts/x.sh|  sudo $b cargo
scripts/x.sh|ps aux | grep cargo; $a -9 rustc
scripts/x.sh|echo hi && $b -9 rustc
scripts/x.sh|kill \$(pgrep -f cargo)
scripts/x.sh|pgrep -f cargo | xargs kill
workflows/x.yaml|        run: $a -f build-slot
workflows/x.yaml|      - run: cd x && $b cargo
EOF
}
cases="$(check_cases "$tool" real)"
echo "$cases" | grep -v '^FAIL' | sed 's/^/ /;s/^ ok/ok/'
if echo "$cases" | grep -q '^FAIL'; then echo "$cases" | grep '^FAIL'; fails=$((fails + 1)); fi

# not red: prose naming the rule, a comment, kill of an own PID
mkdir -p "$tmp/c/scripts"
printf '%s\n' "# never $a here" "echo \"no $a, no $b with patterns\"" "summary: \"no \`$a\` / \`$b\`\"" 'kill "$pid"' >"$tmp/c/scripts/ok.sh"
python3 -I "$tool" "$tmp/c" >/dev/null 2>&1 && ok "prose, comment and kill of an own PID are not red" || bad "a clean file was red"

python3 -I "$tool" "$tmp/nothing" >/dev/null 2>&1; [ $? -eq 2 ] && ok "a missing path is rc 2" || bad "missing path not rc 2"

# mutation: no command pattern matches any more
cp "$tool" "$tmp/mutant"
sed -i 's/^CMD = re.compile(.*/CMD = re.compile(r"(?!x)x")/; s/^PGREP_KILL = re.compile(.*/PGREP_KILL = re.compile(r"(?!x)x")/' "$tmp/mutant"
if cmp -s "$tmp/mutant" "$tool"; then bad "mutation changed nothing"; fi
mcases="$(check_cases "$tmp/mutant" mut)"
if echo "$mcases" | grep -q '^FAIL'; then ok "mutation (patterns removed) turns the planted cases red"; else bad "mutation stayed green"; fi

if [ "$fails" -eq 0 ]; then echo "all ok"; else echo "$fails failed"; exit 1; fi
