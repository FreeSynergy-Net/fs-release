#!/usr/bin/env bash
# Self test of the warning level of scripts/check_workflows.py (ADR 0105 E1): an ai step without bot-seat warns and stays rc 0,
# an ai step with actor-type Application warns (Application is still accepted, decision 8 only warns), ai + Service + bot-seat is clean, a real error is still rc 1.
#   bash scripts/test_check_workflows_warn.sh
set -uo pipefail
here="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
tool="$here/check_workflows.py"
tmp="$(mktemp -d)"
trap 'rm -rf "$tmp"' EXIT
fails=0
ok() { echo "ok   $1"; }
bad() { echo "FAIL $1"; fails=$((fails + 1)); }

mk() { # <dir> <actor-type> <extra metadata line>
  mkdir -p "$1/run"; cp "$here/../workflows/run/rules.yaml" "$1/run/"
  cat >"$1/w.yaml" <<YAML
document: {dsl: 1.0.0, namespace: fs, name: w, version: 0.0.1}
do:
  - s1:
      set: {a: 1}
      metadata:
        freesynergy.net:actor: ai
        freesynergy.net:actor-type: $2
        freesynergy.net:gate: none
        freesynergy.net:kind: objective
$3
YAML
}

mk "$tmp/a" Application ""
python3 -I "$tool" "$tmp/a" >"$tmp/o" 2>&1; r=$?
[ "$r" -eq 0 ] && grep -q "warn  w.yaml:.*bot-seat fehlt" "$tmp/o" && ok "ai step without bot-seat: warning, rc 0" || { cat "$tmp/o"; bad "missing bot-seat (rc=$r)"; }

mk "$tmp/b" Application "        freesynergy.net:bot-seat: x
        freesynergy.net:role: r
        freesynergy.net:effort: low"
python3 -I "$tool" "$tmp/b" >"$tmp/o" 2>&1; r=$?
[ "$r" -eq 0 ] && ! grep -q "bot-seat fehlt" "$tmp/o" && ok "bot-seat and new optional keys: no bot-seat warning, rc 0" || { cat "$tmp/o"; bad "clean case (rc=$r)"; }

mk "$tmp/c" Application "        freesynergy.net:bot-seat: x"
python3 -I "$tool" "$tmp/c" >"$tmp/o" 2>&1; r=$?
[ "$r" -eq 0 ] && grep -q "warn  w.yaml:.*decision 8" "$tmp/o" && ok "ai + Application: warning, rc 0" || { cat "$tmp/o"; bad "Application (rc=$r)"; }

mk "$tmp/e" Service "        freesynergy.net:bot-seat: x"
python3 -I "$tool" "$tmp/e" >"$tmp/o" 2>&1; r=$?
[ "$r" -eq 0 ] && ! grep -q "warn " "$tmp/o" && ok "ai + Service + bot-seat: no warning, rc 0" || { cat "$tmp/o"; bad "ai+Service (rc=$r)"; }

mk "$tmp/f" Person "        freesynergy.net:bot-seat: x"
python3 -I "$tool" "$tmp/f" >"$tmp/o" 2>&1; r=$?
[ "$r" -eq 1 ] && ok "ai + Person is an error (rc 1)" || { cat "$tmp/o"; bad "ai+Person (rc=$r)"; }

mk "$tmp/d" Application ""
sed -i 's/kind: objective/kind: bogus/' "$tmp/d/w.yaml"
python3 -I "$tool" "$tmp/d" >"$tmp/o" 2>&1; r=$?
[ "$r" -eq 1 ] && ok "a real error is still rc 1" || { cat "$tmp/o"; bad "error not rc 1 (rc=$r)"; }

# decision 2=B switch: off by default (warning above); a past date makes the missing bot-seat an error, a future date keeps the warning
mk "$tmp/g" Service ""
FS_BOTSEAT_ERROR_FROM=2000-01-01 python3 -I "$tool" "$tmp/g" >"$tmp/o" 2>&1; r=$?
[ "$r" -eq 1 ] && grep -q "bot-seat.*is missing" "$tmp/o" && ok "date reached: missing bot-seat is an error (rc 1)" || { cat "$tmp/o"; bad "switch on (rc=$r)"; }
FS_BOTSEAT_ERROR_FROM=2999-01-01 python3 -I "$tool" "$tmp/g" >"$tmp/o" 2>&1; r=$?
[ "$r" -eq 0 ] && grep -q "warn  w.yaml:.*bot-seat fehlt" "$tmp/o" && ok "date in the future: still a warning, rc 0" || { cat "$tmp/o"; bad "switch future (rc=$r)"; }
mk "$tmp/h" Service "        freesynergy.net:bot-seat: x"
FS_BOTSEAT_ERROR_FROM=2000-01-01 python3 -I "$tool" "$tmp/h" >"$tmp/o" 2>&1; r=$?
[ "$r" -eq 0 ] && ok "date reached, bot-seat present: rc 0" || { cat "$tmp/o"; bad "switch on, seat present (rc=$r)"; }

if [ "$fails" -eq 0 ]; then echo "all ok"; else echo "$fails failed"; exit 1; fi
