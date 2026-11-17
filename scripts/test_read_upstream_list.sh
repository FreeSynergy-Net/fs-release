#!/usr/bin/env bash
# Self test of scripts/read-upstream-list: the list file is validated (repo name, digits only), bad input is skipped (rc 2), never guessed or an empty green.
#   bash scripts/test_read_upstream_list.sh
set -uo pipefail
here="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
tool="$here/read-upstream-list"
tmp="$(mktemp -d)"
trap 'rm -rf "$tmp"' EXIT
fails=0
chk() { local name="$1"; shift; if "$@"; then echo "ok   $name"; else echo "FAIL $name"; fails=$((fails + 1)); fi; }
q() { python3 -I -c 'import json,sys; d=json.load(sys.stdin); print(eval(sys.argv[1]))' "$1" <<<"$out"; }
printf '# our open items\n\nproject-blinc/Blinc#77 pull\nproject-blinc/Blinc#73 pull\nproject-blinc/Blinc#73 pull\nproject-blinc/Blinc#80 issue\nother/Repo.x#5 issue\n' >"$tmp/ok"
out="$(python3 -I "$tool" --datei "$tmp/ok")"; rc=$?
chk "rc 0" [ "$rc" -eq 0 ]
chk "sorted, no double, comments skipped" [ "$(q '[(i["repo"], i["nummer"], i["art"]) for i in d["nummern"]]')" = "[('other/Repo.x', 5, 'issue'), ('project-blinc/Blinc', 73, 'pull'), ('project-blinc/Blinc', 77, 'pull'), ('project-blinc/Blinc', 80, 'issue')]" ]
chk "not empty" [ "$(q 'd["leer"]')" = False ]
out="$(python3 -I "$tool" --datei "$tmp/ok" --repo project-blinc/blinc)"
chk "--repo keeps the items of that repo (case free)" [ "$(q 'len(d["nummern"])')" = 3 ]
printf '# only comments\n' >"$tmp/empty"
out="$(python3 -I "$tool" --datei "$tmp/empty")"; rc=$?
chk "empty list: rc 0 and leer true (the workflow ends yellow)" [ "$rc" -eq 0 ] 
chk "leer flag" [ "$(q 'd["leer"]')" = True ]
out="$(python3 -I "$tool" --datei "$tmp/missing")"; rc=$?
chk "missing file: rc 2, skipped" [ "$rc" -eq 2 ]
chk "skipped status and leer" [ "$(q 'd["status"], d["leer"]')" = "('skipped', True)" ]
bad() { printf '%s\n' "$1" >"$tmp/bad"; out="$(python3 -I "$tool" --datei "$tmp/bad")"; rc=$?; [ "$rc" -eq 2 ] && [ "$(q 'len(d["nummern"])')" = 0 ]; }
chk "number with letters is refused" bad 'project-blinc/Blinc#7a pull'
chk "repo with a space is refused" bad 'project blinc/Blinc#7 pull'
chk "repo with a shell character is refused" bad 'a/b;rm#7 pull'
chk "repo without owner is refused" bad 'Blinc#7 pull'
chk "unknown kind is refused" bad 'project-blinc/Blinc#7 merge'
chk "missing kind is refused" bad 'project-blinc/Blinc#7'
chk "negative number is refused" bad 'project-blinc/Blinc#-7 pull'
chk "parent segment ../x is refused" bad '../x#7 pull'
chk "parent segment a/.. is refused" bad 'a/..#7 pull'
chk "dot segment ./b is refused" bad './b#7 pull'
chk "dot segment a/. is refused" bad 'a/.#7 pull'
printf '.a/b..#7 pull\n' >"$tmp/dots"
out="$(python3 -I "$tool" --datei "$tmp/dots")"; rc=$?
chk "dots inside a name stay allowed (.a/b..)" [ "$rc" -eq 0 ]
printf 'project-blinc/Blinc#1 pull\nbroken\n' >"$tmp/half"
out="$(python3 -I "$tool" --datei "$tmp/half")"; rc=$?
chk "one bad line makes the whole list skipped (no half list)" [ "$rc" -eq 2 ]
# mutation: the number check accepts any text -> the planted bad number is no longer refused
sed 's/(\[0-9\]+)/([0-9a-z]+)/' "$tool" >"$tmp/mutant"
differs() { ! cmp -s "$1" "$2"; }
chk "mutation changed something" differs "$tmp/mutant" "$tool"
printf '%s\n' 'project-blinc/Blinc#7a pull' >"$tmp/bad"
out="$(python3 -I "$tmp/mutant" --datei "$tmp/bad" 2>/dev/null)"; rc=$?
chk "mutation (digits check loosened) is caught by the bad-number case" [ "$rc" -ne 2 ]
# mutation: the segment rule is dropped -> ../x is accepted again, caught by the parent-segment case
sed 's/(?!\\.{1,2}(?:\/|#|\$))//' "$tool" >"$tmp/mutant2"
chk "mutation 2 changed something" differs "$tmp/mutant2" "$tool"
printf '%s\n' '../x#7 pull' >"$tmp/bad"
out="$(python3 -I "$tmp/mutant2" --datei "$tmp/bad" 2>/dev/null)"; rc=$?
chk "mutation 2 (segment rule removed) is caught by ../x" [ "$rc" -ne 2 ]
if [ "$fails" -eq 0 ]; then echo "all ok"; else echo "$fails failed"; exit 1; fi
