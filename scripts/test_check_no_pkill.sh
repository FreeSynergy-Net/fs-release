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
scripts/x.sh|kill \$(pgrep -f cargo)
scripts/x.sh|kill -9 \`pgrep -f cargo\`
scripts/x.sh|if true; then $a cargo; fi
scripts/x.sh|for i in 1 2; do $a cargo; done
scripts/x.sh|  /usr/bin/$a cargo
scripts/x.sh|  command $a cargo
scripts/x.sh|  \\$a cargo
scripts/x.sh|kill \$(pidof cargo)
scripts/x.sh|pidof cargo | xargs kill
scripts/x.sh|pidof cargo | xargs -r kill -9
scripts/x.sh|ps aux | grep cargo | awk '{print \$2}' | xargs kill
scripts/x.sh|fuser -k 8080/tcp
scripts/x.sh|sudo fuser -km /srv
scripts/x.sh|$b -9 cargo
scripts/x.sh|for p in \$(pgrep cargo); do kill \$p; done
scripts/x.sh|pgrep cargo; kill 1
workflows/x.yaml|        run: $a -f build-slot
workflows/x.yaml|      - run: cd x && $b cargo
EOF
}
cases="$(check_cases "$tool" real)"
echo "$cases" | grep -v '^FAIL' | sed 's/^/ /;s/^ ok/ok/'
if grep -q '^FAIL' <<<"$cases"; then grep '^FAIL' <<<"$cases"; fails=$((fails + 1)); fi

# not red: prose naming the rule, a comment, kill of an own PID
mkdir -p "$tmp/c/scripts"
printf '%s\n' "# never $a here" "echo \"no $a, no $b with patterns\"" "summary: \"no \`$a\` / \`$b\`\"" 'kill "$pid"' >"$tmp/c/scripts/ok.sh"
python3 -I "$tool" "$tmp/c" >/dev/null 2>&1 && ok "prose, comment and kill of an own PID are not red" || bad "a clean file was red"

# false alarms that must stay green: own PID (also from $!), a lookup that only asks (no kill), a kill of a variable that is NOT a lookup, markdown quotes
mkdir -p "$tmp/f/scripts"
cat >"$tmp/f/scripts/ok.sh" <<EOF
scripts/build-slot.sh cargo test & PID=\$!
kill \$PID
kill -TERM "\$PID"
if pgrep -f cargo >/dev/null; then echo busy; fi
n=\$(pgrep -c cargo)
echo "see \`$a\` and \`kill \\\`pgrep\\\`\` in the rule"
pid=\$(cat slot.pid); kill "\$pid"
fuser /srv/x
EOF
python3 -I "$tool" "$tmp/f" >"$tmp/o" 2>&1 && ok "own PID, a pure lookup, fuser without -k are not red" || { cat "$tmp/o"; bad "a false alarm"; }

# a variable that holds a lookup and is killed on a LATER line (x=\$(pgrep ..); kill \$x), also a for loop over the lookup
mkdir -p "$tmp/v/scripts"
for body in 'x=$(pgrep -f cargo)
echo "$x"
kill $x' 'p=`pidof cargo`
kill -9 "$p"' 'for q in $(pgrep cargo)
do
  kill ${q}
done'; do
  printf '%s\n' "$body" >"$tmp/v/scripts/x.sh"
  python3 -I "$tool" "$tmp/v" >"$tmp/o" 2>&1; r=$?
  if [ "$r" -eq 1 ]; then ok "lookup variable killed on a later line is red"; else bad "variable kill not red (rc=$r): $body"; fi
done

python3 -I "$tool" "$tmp/nothing" >/dev/null 2>&1; [ $? -eq 2 ] && ok "a missing path is rc 2" || bad "missing path not rc 2"

# mutation: no command pattern matches any more
cp "$tool" "$tmp/mutant"
sed -i 's/^CMD = re.compile(.*/CMD = re.compile(r"(?!x)x")/; s/^PGREP_KILL = re.compile(.*/PGREP_KILL = re.compile(r"(?!x)x")/' "$tmp/mutant"
if cmp -s "$tmp/mutant" "$tool"; then bad "mutation changed nothing"; fi
mcases="$(check_cases "$tmp/mutant" mut)"
if grep -q '^FAIL' <<<"$mcases"; then ok "mutation (patterns removed) turns the planted cases red"; else bad "mutation stayed green"; fi

if [ "$fails" -eq 0 ]; then echo "all ok"; else echo "$fails failed"; exit 1; fi
