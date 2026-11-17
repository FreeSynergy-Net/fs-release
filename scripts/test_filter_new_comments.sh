#!/usr/bin/env bash
# Self test of scripts/filter-new-comments (pure filter, no network): NEW = later than the last check OR id not remembered; own accounts (default FreeSynergy,
# any case) and [bot] accounts never count; the first run is a BASELINE (remember all, output nothing); the order is deterministic. Mutants for the three conditions.
#   bash scripts/test_filter_new_comments.sh
set -uo pipefail
here="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
tool="$here/filter-new-comments"
tmp="$(mktemp -d)"
trap 'rm -rf "$tmp"' EXIT
fails=0
chk() { local name="$1"; shift; if "$@"; then echo "ok   $name"; else echo "FAIL $name"; fails=$((fails + 1)); fi; }
differs() { ! cmp -s "$1" "$2"; }
q() { python3 -I -c 'import json,sys; d=json.load(sys.stdin); print(eval(sys.argv[1]))' "$1" <<<"$out"; }
cat >"$tmp/abruf.json" <<'J'
{"status":"ok","fehler":[],"eintraege":[
 {"repo":"project-blinc/Blinc","nummer":77,"art":"pull","pull":null,"kommentare":[
  {"kind":"issue","id":9,"user":"dave","created_at":"2026-11-05T09:00:00Z","body":"late","state":null},
  {"kind":"review","id":3,"user":"Carol","created_at":"2026-11-04T10:00:00Z","body":"review","state":"COMMENTED"}]},
 {"repo":"project-blinc/Blinc","nummer":73,"art":"pull","pull":null,"kommentare":[
  {"kind":"issue","id":1,"user":"alice","created_at":"2026-11-01T10:00:00Z","body":"old","state":null},
  {"kind":"issue","id":2,"user":"FREESYNERGY","created_at":"2026-11-02T10:00:00Z","body":"ours","state":null},
  {"kind":"pull","id":1,"user":"dependabot[bot]","created_at":"2026-11-03T10:00:00Z","body":"bot","state":null},
  {"kind":"pull","id":4,"user":"bob","created_at":"2026-11-03T11:00:00Z","body":"line","state":null}]}]}
J
run() { out="$(python3 -I "${TOOL:-$tool}" --abruf "$tmp/abruf.json" "$@")"; rc=$?; }
keys() { q '[k["key"] for k in d["kommentare"]]'; }
# remembered everything but the time rule: the since time alone makes the later ones new
run --letzte-pruefung 2026-11-03T10:30:00Z --gemerkt '["issue:1","issue:2","pull:1","pull:4","issue:9","review:3"]'
chk "rc 0" [ "$rc" -eq 0 ]
chk "time rule: later than the last check (even if remembered), sorted by time" [ "$(keys)" = "['pull:4', 'review:3', 'issue:9']" ]
# remembered nothing new in time, but ids unknown: the id rule makes them new
run --letzte-pruefung 2026-12-01T00:00:00Z --gemerkt '["issue:2"]'
chk "id rule: not remembered = new although older than the last check" [ "$(keys)" = "['issue:1', 'pull:4', 'review:3', 'issue:9']" ]
chk "own (any case) and [bot] are never new" [ "$(q '[k["autor"] for k in d["kommentare"]]')" = "['alice', 'bob', 'Carol', 'dave']" ]
run --letzte-pruefung 2026-12-01T00:00:00Z --gemerkt '["issue:1","issue:2","pull:1","pull:4","issue:9","review:3"]'
chk "nothing new = empty list (rc 0)" [ "$(q 'd["kommentare"], d["grundlinie"]')" = "([], False)" ]
# condition 1: eigene default
run --letzte-pruefung 2026-12-01T00:00:00Z --gemerkt '["issue:1"]' --eigene '[]'
chk "empty eigene falls back to FreeSynergy (own comment is not foreign)" [ "$(q '"FREESYNERGY" in [k["autor"] for k in d["kommentare"]]')" = False ]
run --letzte-pruefung 2026-12-01T00:00:00Z --gemerkt '["issue:1"]'
chk "absent eigene falls back to FreeSynergy" [ "$(q '"FREESYNERGY" in [k["autor"] for k in d["kommentare"]]')" = False ]
run --letzte-pruefung 2026-12-01T00:00:00Z --gemerkt '["issue:1"]' --eigene 'ALICE, other'
chk "given eigene (comma list, any case) are skipped, FreeSynergy is then not special" [ "$(q '("alice" in [k["autor"] for k in d["kommentare"]], "FREESYNERGY" in [k["autor"] for k in d["kommentare"]])')" = "(False, True)" ]
# condition 3: baseline
run
chk "baseline (empty gemerkt, no time): no comment is output" [ "$(q 'd["kommentare"], d["grundlinie"]')" = "([], True)" ]
chk "baseline remembers ALL keys, own and bots too" [ "$(q 'len(d["gemerkt"])')" = 6 ]
run --letzte-pruefung ""  --gemerkt '[]'
chk "baseline also for explicit empty values" [ "$(q 'd["grundlinie"]')" = True ]
run --letzte-pruefung 2026-11-04T00:00:00Z
chk "empty gemerkt WITH a time is no baseline (the time is the start value)" [ "$(q 'd["grundlinie"]')" = False ]
run --letzte-pruefung "not a time" 
chk "an unreadable time with empty gemerkt is a baseline, not a flood of drafts" [ "$(q 'd["grundlinie"], d["kommentare"]')" = "(True, [])" ]
run --gemerkt '["issue:1"]'
chk "remembered ids without a time is no baseline (id rule)" [ "$(q 'd["grundlinie"], len(d["kommentare"])')" = "(False, 3)" ]
chk "gemerkt of the result = old + all present, sorted, no doubles" [ "$(q 'd["gemerkt"] == sorted(set(d["gemerkt"])) and "issue:1" in d["gemerkt"]')" = True ]
# chain: run 1 (baseline) -> its gemerkt and a time into run 2 -> still no draft for the old comments
run; state="$(q 'json.dumps(d["gemerkt"])')"
run --letzte-pruefung 2026-11-06T00:00:00Z --gemerkt "$state"
chk "run 2 after the baseline gives no draft for the old comments" [ "$(q 'd["kommentare"], d["grundlinie"]')" = "([], False)" ]
# deterministic: the same input in another order gives the same output
python3 -I - "$tmp/abruf.json" "$tmp/abruf2.json" <<'PY'
import json, sys
d = json.load(open(sys.argv[1]))
d["eintraege"].reverse()
for e in d["eintraege"]:
    e["kommentare"].reverse()
json.dump(d, open(sys.argv[2], "w"))
PY
run --letzte-pruefung 2026-12-01T00:00:00Z --gemerkt '["issue:2"]'; first="$out"
out="$(python3 -I "$tool" --abruf "$tmp/abruf2.json" --letzte-pruefung 2026-12-01T00:00:00Z --gemerkt '["issue:2"]')"
chk "reordered input gives the identical output" [ "$out" = "$first" ]
# bad input
printf '{"status":"error","eintraege":[],"fehler":["x"]}' >"$tmp/err.json"
python3 -I "$tool" --abruf "$tmp/err.json" >/dev/null 2>&1; rc=$?
chk "a fetch result that is not ok is refused (rc 2)" [ "$rc" -eq 2 ]
python3 -I "$tool" --abruf "$tmp/missing.json" >/dev/null 2>&1; rc=$?
chk "a missing file is refused (rc 2)" [ "$rc" -eq 2 ]
# mutants
mut() { sed "$2" "$tool" >"$tmp/$1"; differs "$tmp/$1" "$tool"; }
chk "mutant A (eigene default empty) changed something" mut mA 's/^DEFAULT_EIGENE = .*/DEFAULT_EIGENE = []/'
TOOL="$tmp/mA" run --letzte-pruefung 2026-12-01T00:00:00Z --gemerkt '["issue:1"]'
chk "mutant A (default []) is caught: our own comment turns foreign" [ "$(q '"FREESYNERGY" in [k["autor"] for k in d["kommentare"]]')" = True ]
chk "mutant B (baseline off) changed something" mut mB 's/baseline = not known and since is None/baseline = False/'
TOOL="$tmp/mB" run
chk "mutant B is caught: the first run floods drafts" [ "$(q 'len(d["kommentare"]) > 0')" = True ]
chk "mutant C ([bot] not skipped) changed something" mut mC 's/u.endswith("\[bot\]") or //'
TOOL="$tmp/mC" run --letzte-pruefung 2026-12-01T00:00:00Z --gemerkt '["issue:1"]'
chk "mutant C is caught: the bot is foreign" [ "$(q '"dependabot[bot]" in [k["autor"] for k in d["kommentare"]]')" = True ]
chk "mutant D (case sensitive) changed something" mut mD 's/u in {e.lower() for e in eigene}/(user or "") in set(eigene)/'
TOOL="$tmp/mD" run --letzte-pruefung 2026-12-01T00:00:00Z --gemerkt '["issue:1"]'
chk "mutant D is caught: FREESYNERGY is foreign" [ "$(q '"FREESYNERGY" in [k["autor"] for k in d["kommentare"]]')" = True ]
chk "mutant E (id rule dropped) changed something" mut mE 's/new = key not in known or /new = /'
TOOL="$tmp/mE" run --letzte-pruefung 2026-12-01T00:00:00Z --gemerkt '["issue:2"]'
chk "mutant E is caught: the unknown old comment is lost" [ "$(q 'len(d["kommentare"])')" = 0 ]
chk "mutant F (time rule dropped) changed something" mut mF 's/ or (since is not None and when is not None and when > since)//'
TOOL="$tmp/mF" run --letzte-pruefung 2026-11-03T10:30:00Z --gemerkt '["issue:1","issue:2","pull:1","pull:4","issue:9","review:3"]'
chk "mutant F is caught: the time rule no longer fires" [ "$(q 'len(d["kommentare"])')" = 0 ]
if [ "$fails" -eq 0 ]; then echo "all ok"; else echo "$fails failed"; exit 1; fi
