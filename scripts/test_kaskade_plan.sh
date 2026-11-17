#!/usr/bin/env bash
# Self test of scripts/kaskade-lage, kaskade-plan, kaskade-glied and kaskade-glied-fertig (workflow pin-kaskade) on local bare repos (scripts/test_kaskade_fixture.sh), no network:
# lage proves trigger and fs-release tag and the Windows caller per Glied, plan derives the order from the REAL pins (input order wrong on purpose), finds rev pins, wrong
# sources, known foreign sources and a cycle, glied takes the libs, glied-fertig needs the tag on the remote. One mutant per script must turn a case red.
#   bash scripts/test_kaskade_plan.sh
set -uo pipefail
here="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
mkdir -p "$here/../.test-tmp"; tmp="$(TMPDIR="${TEST_TMPDIR:-$here/../.test-tmp}" mktemp -d)"  # never /tmp (RAM, rule [PLATTE])
srv=""
trap '[ -z "$srv" ] || kill "$srv" 2>/dev/null; rm -rf "$tmp"; rmdir "$here/../.test-tmp" 2>/dev/null' EXIT
fails=0
ok() { echo "ok   $1"; }
bad() { echo "FAIL $1"; fails=$((fails + 1)); }
chk() { local name="$1"; shift; if "$@"; then ok "$name"; else bad "$name"; fi; }
# shellcheck source=scripts/test_kaskade_fixture.sh
source "$here/test_kaskade_fixture.sh"
py() { python3 -I "$@"; }
glieder_json >"$tmp/glieder.json"

# ---- kaskade-lage ----
py "$here/kaskade-lage" --ausloeser v0.0.3 --glieder "$tmp/glieder.json" --ohne-ci >"$tmp/lage.json"; rc=$?
chk "lage: rc 0" [ "$rc" -eq 0 ]
chk "lage: trigger tag proven by ls-remote" [ "$(q "$tmp/lage.json" 'len(d["ausloeser"]["beleg"])')" = 40 ]
chk "lage: fs-release v0.0.11 proven" [ "$(q "$tmp/lage.json" 'len(d["fs_release"]["beleg"])')" = 40 ]
chk "lage: the Glied input is kept (art, allow, besonderes) for the plan" [ "$(q "$tmp/lage.json" '[g for g in d["glieder"] if g["repo"]=="fs-interfaces"][0]["besonderes"]')" = capnp ]
chk "lage: the head of main is the ls-remote head" [ "$(q "$tmp/lage.json" '[g for g in d["glieder"] if g["repo"]=="fs-interfaces"][0]["kopf"]')" = "$ifc_head" ]
chk "lage: Windows caller found at a tag >= v0.0.11" [ "$(q "$tmp/lage.json" '[w for w in d["windows"] if w["repo"]=="fs-interfaces"][0]["pin_ok"]')" = True ]
chk "lage: a Glied without caller is listed, not red" [ "$(q "$tmp/lage.json" 'sorted(d["ohne_aufrufer"])')" = "['fs-objects', 'fs-people', 'fs-tasks']" ]
py "$here/kaskade-lage" --ausloeser v9.9.9 --glieder "$tmp/glieder.json" --ohne-ci >"$tmp/o.json"; rc=$?
chk "lage: a missing trigger tag is red (rc 1)" [ "$rc" -eq 1 ]
py "$here/kaskade-lage" --ausloeser v0.0.3 --glieder "$tmp/glieder.json" --ohne-ci --fs-release-tag v0.0.12 >"$tmp/o.json"; rc=$?
chk "lage: a missing fs-release tag is red (the cascade starts only after it)" [ "$rc" -eq 1 ]
py "$here/kaskade-lage" --ausloeser v0.0.3 --glieder "$tmp/glieder.json" --ab nope --ohne-ci >"$tmp/o.json"; rc=$?
chk "lage: --ab that is no Glied is bad input (rc 2)" [ "$rc" -eq 2 ]
py "$here/kaskade-lage" --ausloeser v0.0.3 --glieder '[{"repo":"fs-x"}]' --ohne-ci >"$tmp/o.json"; rc=$?
chk "lage: a Glied without tag/art/allow is bad input (rc 2)" [ "$rc" -eq 2 ]
# resume: the Glieder before --ab are done ONLY with their tag on the remote
printf '[{"repo":"fs-interfaces","art":"bibliothek","tag":"v0.0.2","allow":"x"},{"repo":"fs-objects","art":"bibliothek","tag":"v0.0.2","allow":"x"},{"repo":"fs-people","art":"programm","tag":"v0.0.2","allow":"x"}]' >"$tmp/ab.json"
py "$here/kaskade-lage" --ausloeser v0.0.3 --glieder "$tmp/ab.json" --ab fs-people --ohne-ci >"$tmp/o.json"; rc=$?
chk "lage: with --ab a done-looking Glied without remote tag is red (fs-objects has no v0.0.2)" [ "$rc" -eq 1 ]
tag_push fs-objects v0.0.2
py "$here/kaskade-lage" --ausloeser v0.0.3 --glieder "$tmp/ab.json" --ab fs-people --ohne-ci >"$tmp/lage-ab.json"; rc=$?
chk "lage: with --ab and both tags on the remote: rc 0, two done with proof" [ "$rc" -eq 0 ] && [ "$(q "$tmp/lage-ab.json" '[e["repo"] for e in d["erledigt"]]')" = "['fs-interfaces', 'fs-objects']" ]
# the CI state of exactly this head, from a local stand-in of the public REST API (GET only)
mkdir -p "$tmp/api/repos/FreeSynergy/fs-interfaces/actions"
printf '{"workflow_runs":[{"name":"windows","head_sha":"%s","status":"completed","conclusion":"success"},{"name":"windows","head_sha":"0000","status":"completed","conclusion":"failure"}]}' "$ifc_head" >"$tmp/api/repos/FreeSynergy/fs-interfaces/actions/runs"
port="$(python3 -I -c 'import socket;s=socket.socket();s.bind(("127.0.0.1",0));print(s.getsockname()[1])')"
python3 -I -m http.server "$port" --bind 127.0.0.1 -d "$tmp/api" >/dev/null 2>&1 & srv=$!
for _ in 1 2 3 4 5 6 7 8 9 10; do python3 -I -c "import urllib.request,sys; urllib.request.urlopen('http://127.0.0.1:$port/')" 2>/dev/null && break; sleep 0.3; done
FS_KASKADE_API="http://127.0.0.1:$port" py "$here/kaskade-lage" --ausloeser v0.0.3 --glieder "$tmp/glieder.json" >"$tmp/o.json"
chk "lage: Windows CI green for exactly this head (an older red run does not count)" [ "$(q "$tmp/o.json" '[w for w in d["windows"] if w["repo"]=="fs-interfaces"][0]["windows_ci"]')" = gruen ]
chk "lage: no run known is reported, never green" [ "$(q "$tmp/o.json" '[w for w in d["windows"] if w["repo"]=="fs-objects"][0]["windows_ci"].startswith("unbekannt")')" = True ]

# ---- kaskade-plan ----
py "$here/kaskade-plan" --lage "$tmp/lage.json" >"$tmp/plan.json"; rc=$?
chk "plan: rc 0" [ "$rc" -eq 0 ]
chk "plan: order from the real pins (input: fs-people stands before its dependency fs-interfaces), ties in input order" [ "$(q "$tmp/plan.json" '[g["repo"] for g in d["offen"]]')" = "['fs-interfaces', 'fs-people', 'fs-tasks', 'fs-objects']" ]
chk "plan: fs-tasks depends on fs-people (rev pin edge) and fs-interfaces (fs-types comes through fs-interfaces: no edge to fs-types)" [ "$(q "$tmp/plan.json" 'd["kanten"]["fs-tasks"]')" = "['fs-people', 'fs-interfaces']" ]
chk "plan: rev pin of fs-people-directory listed" [ "$(q "$tmp/plan.json" '[(p["crate"], p["repo"]) for p in [g for g in d["offen"] if g["repo"]=="fs-tasks"][0]["rev_pins"]]')" = "[('fs-people-directory', 'fs-people')]" ]
chk "plan: fs-types from the repo fs-interfaces is a movable wrong source, not a finding" [ "$(q "$tmp/plan.json" '[(w["crate"], w["verschiebbar"]) for w in [g for g in d["offen"] if g["repo"]=="fs-tasks"][0]["falsche_quellen"]]')" = "[('fs-types', True)]" ]
chk "plan: the foreign source blinc-app-fix is an exception, not a finding" [ "$(q "$tmp/plan.json" '[g for g in d["offen"] if g["repo"]=="fs-tasks"][0]["fremdquellen"]')" = "['FreeSynergy/blinc-app-fix']" ]
chk "plan: art, tag and allow are carried" [ "$(q "$tmp/plan.json" '[g["art"] for g in d["offen"]][:2]')" = "['bibliothek', 'programm']" ]
chk "plan: with --ab the done Glieder are not planned again" [ "$(py "$here/kaskade-plan" --lage "$tmp/lage-ab.json" | python3 -I -c 'import json,sys; print([g["repo"] for g in json.load(sys.stdin)["offen"]])')" = "['fs-people']" ]
# a dependency on a repo that is no Glied
printf '[{"repo":"fs-tasks","art":"programm","tag":"v0.0.2","allow":"x"},{"repo":"fs-interfaces","art":"bibliothek","tag":"v0.0.2","allow":"x"}]' >"$tmp/g2.json"
py "$here/kaskade-lage" --ausloeser v0.0.3 --glieder "$tmp/g2.json" --ohne-ci >"$tmp/lage2.json"
py "$here/kaskade-plan" --lage "$tmp/lage2.json" >"$tmp/o.json"; rc=$?
chk "plan: a dependency on a repo that is not a Glied (fs-people) is a finding (rc 1)" [ "$rc" -eq 1 ]
# a cycle
for r in cyc-a cyc-b; do
  o=cyc-b; [ "$r" = cyc-b ] && o=cyc-a
  newrepo "$r"; printf '[dependencies]\nx = { git = "%s/%s", tag = "v0.0.1" }\n' "$GH" "$o" >"$tmp/work/$r/Cargo.toml"; commit_push "$r" one; tag_push "$r" v0.0.1
done
printf '[{"repo":"cyc-a","art":"bibliothek","tag":"v0.0.1","allow":"x"},{"repo":"cyc-b","art":"bibliothek","tag":"v0.0.1","allow":"x"}]' >"$tmp/g3.json"
py "$here/kaskade-lage" --ausloeser v0.0.3 --glieder "$tmp/g3.json" --ohne-ci >"$tmp/lage3.json"
py "$here/kaskade-plan" --lage "$tmp/lage3.json" >"$tmp/o.json"; rc=$?
chk "plan: a cycle is a finding (rc 1)" [ "$rc" -eq 1 ] && [ "$(q "$tmp/o.json" '"cycle" in d["befunde"][0]')" = True ]

# ---- kaskade-glied ----
py "$here/kaskade-glied" --plan "$tmp/plan.json" --ausloeser v0.0.3 >"$tmp/o.json"; rc=$?
chk "glied: takes offen[0] (fs-interfaces) with the trigger as lib" [ "$rc" -eq 0 ] && [ "$(q "$tmp/o.json" '(d["glied"]["repo"], d["glied"]["libs"], d["glied"]["prioritaet"])')" = "('fs-interfaces', ['fs-types=v0.0.3'], 0)" ]
printf '{"offen":[{"repo":"fs-tasks","art":"programm","tag":"v0.0.2","allow":"x","abhaengig":["fs-types","fs-interfaces","fs-people"]}],"erledigt":[{"repo":"fs-interfaces","tag":"v0.0.2","beleg":"b"},{"repo":"fs-people","tag":"v0.0.5","beleg":"b"}]}' >"$tmp/p3.json"
py "$here/kaskade-glied" --plan "$tmp/p3.json" --ausloeser v0.0.3 >"$tmp/o.json"
chk "glied: libs = trigger + tags of the done Glieder the lift knows; fs-people only in tags" [ "$(q "$tmp/o.json" '(d["glied"]["libs"], d["glied"]["tags"]["fs-people"])')" = "(['fs-interfaces=v0.0.2', 'fs-types=v0.0.3'], 'v0.0.5')" ]
printf '{"offen":[{"repo":"fs-tasks","art":"programm","tag":"v0.0.2","allow":"x","abhaengig":["fs-types","fs-people"]}],"erledigt":[]}' >"$tmp/p4.json"
py "$here/kaskade-glied" --plan "$tmp/p4.json" --ausloeser v0.0.3 >"$tmp/o.json"; rc=$?
chk "glied: a dependency that is not done is red (rc 1)" [ "$rc" -eq 1 ]
# the tool file .fs-lock-sources.toml of the Glied worktree: created, raised from strict = false (reported), kept
mkdir -p "$tmp/wt1" "$tmp/wt2" "$tmp/wt3"
py "$here/kaskade-glied" --plan "$tmp/plan.json" --ausloeser v0.0.3 --dir "$tmp/wt1" >"$tmp/o.json"
chk "glied --dir: a missing .fs-lock-sources.toml is created strict" [ "$(q "$tmp/o.json" 'd["lock_datei"]')" = angelegt ] && grep -q '^strict = true' "$tmp/wt1/.fs-lock-sources.toml" && grep -q '^only_from_enabled = true' "$tmp/wt1/.fs-lock-sources.toml"
printf 'strict = false\nonly_from_enabled = true\n' >"$tmp/wt2/.fs-lock-sources.toml"
py "$here/kaskade-glied" --plan "$tmp/plan.json" --ausloeser v0.0.3 --dir "$tmp/wt2" >"$tmp/o.json"
chk "glied --dir: strict = false is raised to true and reported" [ "$(q "$tmp/o.json" '(d["lock_datei"], len(d["hinweise"]))')" = "('gehoben', 1)" ] && grep -q '^strict = true' "$tmp/wt2/.fs-lock-sources.toml"
printf 'strict = true\nonly_from_enabled = true\n' >"$tmp/wt3/.fs-lock-sources.toml"
py "$here/kaskade-glied" --plan "$tmp/plan.json" --ausloeser v0.0.3 --dir "$tmp/wt3" >"$tmp/o.json"
chk "glied --dir: a strict file stays (vorhanden)" [ "$(q "$tmp/o.json" 'd["lock_datei"]')" = vorhanden ]
py "$here/kaskade-glied" --plan '{"offen":[],"erledigt":[]}' --ausloeser v0.0.3 >"$tmp/o.json"; rc=$?
chk "glied: an empty offen is bad input (rc 2)" [ "$rc" -eq 2 ]

# ---- kaskade-glied-fertig ----
py "$here/kaskade-glied-fertig" --plan "$tmp/plan.json" --glied '{"repo":"fs-interfaces","tag":"v0.0.2"}' --beleg "ff proof" >"$tmp/o.json"; rc=$?
chk "glied-fertig: rc 0, offen shorter by one" [ "$rc" -eq 0 ] && [ "$(q "$tmp/o.json" 'len(d["offen"])')" = 3 ]
chk "glied-fertig: offen_vorher is the OLD length (loop guard)" [ "$(q "$tmp/o.json" 'd["offen_vorher"]')" = 4 ]
chk "glied-fertig: erledigt carries tag and the ls-remote proof" [ "$(q "$tmp/o.json" '(d["erledigt"][-1]["tag"], "refs/tags/v0.0.2 = " + "%s" in d["erledigt"][-1]["beleg"] or "refs/tags/v0.0.2" in d["erledigt"][-1]["beleg"])')" = "('v0.0.2', True)" ]
chk "glied-fertig: a board note text is returned" [ "$(q "$tmp/o.json" '"fs-interfaces v0.0.2 done" in d["karte_notiz"]')" = True ]
python3 -I -c 'import json,sys; d=json.load(open(sys.argv[1])); d["offen"]=d["offen"][:1]; json.dump(d,open(sys.argv[2],"w"))' "$tmp/plan.json" "$tmp/p5.json"
py "$here/kaskade-glied-fertig" --plan "$tmp/p5.json" --glied '{"repo":"fs-interfaces","tag":"v0.0.2"}' >"$tmp/o.json"
chk "glied-fertig: the plan with offen_vorher is fed back and the next call shortens again" [ "$(q "$tmp/o.json" '(len(d["offen"]), d["offen_vorher"])')" = "(0, 1)" ]
py "$here/kaskade-glied-fertig" --plan "$tmp/plan.json" --glied '{"repo":"fs-tasks","tag":"v0.0.2"}' >"$tmp/o.json"; rc=$?
chk "glied-fertig: only offen[0] can be closed (rc 2)" [ "$rc" -eq 2 ]
python3 -I -c 'import json,sys; d=json.load(open(sys.argv[1])); d["offen"]=d["offen"][1:]; json.dump(d,open(sys.argv[2],"w"))' "$tmp/plan.json" "$tmp/p6.json"
py "$here/kaskade-glied-fertig" --plan "$tmp/p6.json" --glied '{"repo":"fs-people","tag":"v0.0.77"}' >"$tmp/o.json"; rc=$?
chk "glied-fertig: a tag that is not on the remote is red, the plan stays" [ "$rc" -eq 1 ] && [ "$(q "$tmp/o.json" 'len(d["offen"])')" = 3 ]

# ---- mutants: one per script, each must turn one of the cases above red ----
cp "$here"/kaskade-lage "$here"/kaskade-plan "$here"/kaskade-glied "$here"/kaskade-glied-fertig "$tmp/mut/"
mutate "$here/kaskade-lage" 'if not trigger:' 'if False:' "$tmp/mut/kaskade-lage" || bad "mutant lage not applied"
py "$tmp/mut/kaskade-lage" --ausloeser v9.9.9 --glieder "$tmp/glieder.json" --ohne-ci >/dev/null; rc=$?
chk "mutant lage (trigger check removed) is caught by the missing-trigger case" [ "$rc" -ne 1 ]
mutate "$here/kaskade-plan" 'all(d in have or d not in offen for d in kanten[g])' 'True' "$tmp/mut/kaskade-plan" || bad "mutant plan not applied"
py "$tmp/mut/kaskade-plan" --lage "$tmp/lage.json" >"$tmp/o.json"
chk "mutant plan (dependencies ignored) is caught by the order case" [ "$(q "$tmp/o.json" '[g["repo"] for g in d["offen"]]')" != "['fs-interfaces', 'fs-people', 'fs-tasks', 'fs-objects']" ]
mutate "$here/kaskade-glied" 'befunde.append(f"{g['"'"'repo'"'"']} depends on' 'pass  # ' "$tmp/mut/kaskade-glied" || bad "mutant glied not applied"
py "$tmp/mut/kaskade-glied" --plan "$tmp/p4.json" --ausloeser v0.0.3 >/dev/null; rc=$?
chk "mutant glied (broken order accepted) is caught by the not-done case" [ "$rc" -ne 1 ]
mutate "$here/kaskade-glied" 'if have.get(k) is True:' 'if True:' "$tmp/mut/kaskade-glied" || bad "mutant glied 2 not applied"
mkdir -p "$tmp/wt4"; printf 'strict = false\n' >"$tmp/wt4/.fs-lock-sources.toml"
py "$tmp/mut/kaskade-glied" --plan "$tmp/plan.json" --ausloeser v0.0.3 --dir "$tmp/wt4" >/dev/null
chk "mutant glied (a false switch is kept) is caught by the strict = false case" grep -q '^strict = false' "$tmp/wt4/.fs-lock-sources.toml"
mutate "$here/kaskade-glied-fertig" 'new["offen_vorher"] = len(offen)' 'new["offen_vorher"] = 0' "$tmp/mut/kaskade-glied-fertig" || bad "mutant fertig not applied"
py "$tmp/mut/kaskade-glied-fertig" --plan "$tmp/plan.json" --glied '{"repo":"fs-interfaces","tag":"v0.0.2"}' >"$tmp/o.json"
chk "mutant glied-fertig (loop guard value lost) is caught by the offen_vorher case" [ "$(q "$tmp/o.json" 'd["offen_vorher"]')" != 4 ]
if [ "$fails" -eq 0 ]; then echo "all ok"; else echo "$fails failed"; exit 1; fi
