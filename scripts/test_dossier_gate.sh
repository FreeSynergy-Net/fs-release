#!/usr/bin/env bash
# Self test of scripts/dossier-gate: green cases, and counter-checks (each defect must really turn it red).
#   bash scripts/test_dossier_gate.sh
set -uo pipefail
here="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
tool=(python3 -I "$here/dossier-gate")
tmp="$(mktemp -d)"
trap 'rm -rf "$tmp"' EXIT
fails=0
head='# Dossier\n\n| Zustand | belegt | Quelle |\n|---|---|---|\n'
n=0
expect() { # expect <rc> <needle> <name> <rows>; every case in a fresh folder
  local out rc
  n=$((n+1)); mkdir -p "$tmp/c$n"
  printf "$head$4" >"$tmp/c$n/states.md"
  out="$("${tool[@]}" "$tmp/c$n" 2>&1)"; rc=$?
  if [ "$rc" -ne "$1" ] || ! grep -qF -- "$2" <<<"$out"; then
    echo "FAIL $3: rc=$rc (want $1), needle '$2'"; printf '    %s\n' "$out"; fails=$((fails+1))
  else echo "ok   $3"; fi
}
expect 0 "rot 0" green "| Liste leer | ja | Vorbild-Seite S1 |\n| Fehler | [offen] | ohne Vorbild |\n"
expect 0 "ohne Vorbild 1" "[offen] marked as own version counts as ohne Vorbild" "| Laden | [offen] | ohne Vorbild (eigene Fassung) |\n"
expect 1 "[offen] ohne Markierung" "counter: [offen] without marker is red" "| Laden | [offen] | - |\n"
expect 1 "belegt ohne Quelle" "counter: belegt without source is red" "| Suche | ja | - |\n"
expect 1 "belegt ohne Quelle" "counter: belegt with empty source is red" "| Suche | ja |  |\n"
expect 1 "zugleich 'ohne Vorbild'" "counter: belegt and ohne Vorbild at once is red" "| Suche | ja | ohne Vorbild |\n"
expect 1 "erlaubt: ja oder [offen]" "counter: unclear belegt value is red" "| Suche | vielleicht | S2 |\n"
expect 1 "belegt ist ''" "counter: empty belegt cell is red" "| Suche |  | S2 |\n"
expect 1 "Zustand ohne Namen" "counter: row without state name is red" "|  | ja | S2 |\n"
expect 1 "Tabelle ohne Zeilen" "counter: table without rows is red" ""
mkdir -p "$tmp/zweit"
printf "$head| Laden | [offen] | - |\n\nRolle: Zweitvorbild\n" >"$tmp/zweit/store.md"
out="$("${tool[@]}" "$tmp/zweit" 2>&1)"; rc=$?
if [ "$rc" -ne 0 ] || ! grep -q 'Zweitvorbild, nicht massgeblich' <<<"$out" || ! grep -q '^green$' <<<"$out"; then echo "FAIL second model: rc=$rc"; printf '    %s\n' "$out"; fails=$((fails+1)); else echo "ok   second model: gap is hinweis, stays green"; fi
# column order is free and a row may be shorter than the header
mkdir -p "$tmp/order"
printf '| Quelle | Zustand | belegt | Notiz |\n|---|---|---|---|\n| S1 | Liste | ja |\n| ohne Vorbild | Fehler | [offen] | x |\n' >"$tmp/order/states.md"
out="$("${tool[@]}" "$tmp/order" 2>&1)"; rc=$?
if [ "$rc" -ne 0 ] || ! grep -q 'ohne Vorbild 1' <<<"$out"; then echo "FAIL column order: rc=$rc"; fails=$((fails+1)); else echo "ok   column order is free"; fi
# folder: a file without the table is listed, not counted as red
mkdir -p "$tmp/folder"
printf "$head| Liste | ja | S1 |\n" >"$tmp/folder/states.md"
printf '# Menues\n\nText ohne Tabelle.\n' >"$tmp/folder/menus.md"
out="$("${tool[@]}" "$tmp/folder" 2>&1)"; rc=$?
if [ "$rc" -ne 0 ] || ! grep -q 'menus.md: ohne Tabelle' <<<"$out"; then echo "FAIL folder with file without table: rc=$rc"; fails=$((fails+1)); else echo "ok   folder: file without table listed, green"; fi
# bad input is never green
out="$("${tool[@]}" "$tmp/gibt-es-nicht" 2>&1)"; rc=$?
if [ "$rc" -ne 2 ]; then echo "FAIL missing path rc=$rc (want 2)"; fails=$((fails+1)); else echo "ok   missing path: rc 2"; fi
mkdir -p "$tmp/leer"
printf '# nur Text\n' >"$tmp/leer/x.md"
out="$("${tool[@]}" "$tmp/leer" 2>&1)"; rc=$?
if [ "$rc" -ne 2 ]; then echo "FAIL no table at all rc=$rc (want 2)"; fails=$((fails+1)); else echo "ok   no table at all: rc 2, never green"; fi
if [ "$fails" -eq 0 ]; then echo "test_dossier_gate: alle Faelle ok"; exit 0; fi
echo "test_dossier_gate: $fails Fehlschlag/Fehlschlaege"; exit 1
