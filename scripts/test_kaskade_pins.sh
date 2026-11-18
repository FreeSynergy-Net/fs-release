#!/usr/bin/env bash
# Pattern: none (test).
# Self test of scripts/kaskade-fremde-pins and scripts/lock-gleich (workflow pin-kaskade) on local bare repos (scripts/kaskade_fixture.sh), no network:
# a `rev` pin is put on the owner tag only when its commit is on that tag, only Cargo.toml changes (never Cargo.lock), a known foreign source is left alone and listed;
# the end check is red for a second fs-types version, a left rev pin and a source from the wrong repo, and refuses a weakened --env. One or two mutants per script.
#   bash scripts/test_kaskade_pins.sh
set -uo pipefail
here="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
mkdir -p "$here/../.test-tmp"; tmp="$(TMPDIR="${TEST_TMPDIR:-$here/../.test-tmp}" mktemp -d)"  # never /tmp (RAM, rule [PLATTE])
trap 'rm -rf "$tmp"; rmdir "$here/../.test-tmp" 2>/dev/null' EXIT
fails=0
: >"$tmp/.fails"  # every failure is ALSO written to this file: a failure in a subshell, pipe or background job cannot be lost (Opus N4)
ok() { echo "ok   $1"; }
bad() { echo "FAIL $1"; fails=$((fails + 1)); echo "$1" >>"$tmp/.fails"; }
chk() { local name="$1"; shift; if "$@"; then ok "$name"; else bad "$name"; fi; }
# shellcheck source=scripts/kaskade_fixture.sh
source "$here/kaskade_fixture.sh"
py() { python3 -I "$@"; }
glied='{"glied":{"repo":"fs-tasks","tags":{"fs-types":"v0.0.3","fs-people":"v0.0.2","fs-interfaces":"v0.0.2"}}}'
# wt <name> <rev>: a work tree of fs-tasks whose fs-people-directory is pinned by `rev`
wt() { git clone -q "$origin/fs-tasks" "$tmp/$1"; tasks_toml "$2" >"$tmp/$1/Cargo.toml"; }

# ---- kaskade-fremde-pins ----
wt a "$rev_a"; lock_sum="$(cksum <"$tmp/a/Cargo.lock")"; toml_before="$(cksum <"$tmp/a/Cargo.toml")"
py "$here/kaskade-fremde-pins" --glied "$glied" --dir "$tmp/a" --nur-lesen >"$tmp/o.json"; rc=$?
chk "pins: --nur-lesen finds the rev pin and changes nothing (rc 0)" [ "$rc" -eq 0 ] && chk "pins: --nur-lesen finds the rev pin and changes nothing (rc 0) [part 2]" [ "$(cksum <"$tmp/a/Cargo.toml")" = "$toml_before" ] && chk "pins: --nur-lesen finds the rev pin and changes nothing (rc 0) [part 3]" [ "$(q "$tmp/o.json" '[p["crate"] for p in d["rev_pins"]]')" = "['fs-people-directory']" ]
py "$here/kaskade-fremde-pins" --glied "$glied" --dir "$tmp/a" >"$tmp/o.json"; rc=$?
chk "pins: rc 0 and Cargo.toml changed" [ "$rc" -eq 0 ] && chk "pins: rc 0 and Cargo.toml changed [part 2]" [ "$(q "$tmp/o.json" 'd["geaendert"]')" = "['Cargo.toml']" ]
chk "pins: the rev of fs-people-directory is now the tag v0.0.2 of fs-people" grep -q "fs-people-directory = { git = \"$GH/fs-people\", tag = \"v0.0.2\" }" "$tmp/a/Cargo.toml"
chk "pins: Cargo.lock is NEVER touched" [ "$(cksum <"$tmp/a/Cargo.lock")" = "$lock_sum" ]
chk "pins: the foreign source blinc-app-fix keeps its rev and is listed, not a finding" grep -q 'blinc-app-fix", rev = "5180d4eae' "$tmp/a/Cargo.toml"
chk "pins: foreign source listed" [ "$(q "$tmp/o.json" 'd["fremdquellen"]')" = "['FreeSynergy/blinc-app-fix']" ]
chk "pins: fs-types from the repo fs-interfaces is a movable wrong source (the lift moves it)" [ "$(q "$tmp/o.json" '[(w["crate"], w["verschiebbar"]) for w in d["falsche_quellen"]]')" = "[('fs-types', True)]" ]
py "$here/kaskade-fremde-pins" --glied "$glied" --dir "$tmp/a" >"$tmp/o.json"
chk "pins: a second run changes nothing" [ "$(q "$tmp/o.json" 'd["geaendert"]')" = "[]" ]
wt b "$rev_b"; b_before="$(cksum <"$tmp/b/Cargo.toml")"
py "$here/kaskade-fremde-pins" --glied "$glied" --dir "$tmp/b" >"$tmp/o.json"; rc=$?
chk "pins: a rev that is on NO tag (commit B after v0.0.2) is a finding, not lifted by guess (rc 1)" [ "$rc" -eq 1 ] && chk "pins: a rev that is on NO tag (commit B after v0.0.2) is a finding, not lifted by guess (rc 1) [part 2]" [ "$(cksum <"$tmp/b/Cargo.toml")" = "$b_before" ]
py "$here/kaskade-fremde-pins" --glied '{"glied":{"repo":"fs-tasks","tags":{"fs-types":"v0.0.3"}}}' --dir "$tmp/b" >"$tmp/o.json"; rc=$?
chk "pins: no tag known for the owner repo is a finding (rc 1)" [ "$rc" -eq 1 ]
py "$here/kaskade-fremde-pins" --glied "$glied" --dir "$tmp/none" >"$tmp/o.json"; rc=$?
chk "pins: a folder without Cargo.toml is bad input (rc 2)" [ "$rc" -eq 2 ]
wt c "$rev_a"; sed -i 's/^fs-people-directory = .*/[dependencies.fs-people-directory]\ngit = "https:\/\/github.com\/FreeSynergy\/fs-people"\nrev = "'"$rev_a"'"/' "$tmp/c/Cargo.toml"
py "$here/kaskade-fremde-pins" --glied "$glied" --dir "$tmp/c" >"$tmp/o.json"; rc=$?
chk "pins: a pin that cannot be edited on its line is a finding, nothing is changed silently (rc 1)" [ "$rc" -eq 1 ]
# the foreign-source list: without it blinc-app-fix is an unknown owner (finding)
printf '' >"$tmp/leer.txt"
wt d "$rev_a"
py "$here/kaskade-fremde-pins" --glied "$glied" --dir "$tmp/d" --fremdquellen "$tmp/leer.txt" >"$tmp/o.json"; rc=$?
chk "pins: without the foreign-source list the Blinc patch repo is a finding (rc 1)" [ "$rc" -eq 1 ]
# mutants
cp "$here/kaskade-fremde-pins" "$tmp/mut/"
mutate "$here/kaskade-fremde-pins" 'elif not contained(d["repo"], d["rev"], tag, cache):' 'elif False:' "$tmp/mut/kaskade-fremde-pins" || bad "mutant pins 1 not applied"
wt e "$rev_b"; py "$tmp/mut/kaskade-fremde-pins" --glied "$glied" --dir "$tmp/e" >/dev/null; rc=$?
chk "mutant pins (ancestor check removed) is caught by the rev-on-no-tag case" [ "$rc" -ne 1 ]
mutate "$here/kaskade-fremde-pins" 'fdeps.add(f"{d['"'"'owner'"'"']}/{d['"'"'repo'"'"']}")
                        continue' 'fdeps.add(f"{d['"'"'owner'"'"']}/{d['"'"'repo'"'"']}")' "$tmp/mut/kaskade-fremde-pins" || bad "mutant pins 2 not applied"
wt f "$rev_a"; py "$tmp/mut/kaskade-fremde-pins" --glied "$glied" --dir "$tmp/f" >/dev/null; rc=$?
chk "mutant pins (foreign source not left alone) is caught by the green case" [ "$rc" -ne 0 ]

# ---- lock-gleich ----
# fs-interfaces v0.0.2: lock on fs-types v0.0.3 from FreeSynergy/fs-types (green). fs-objects v0.0.2: lock on fs-types v0.0.2 (a second version). fs-tasks v0.0.9: a left rev pin and fs-types from the repo fs-interfaces.
lock fs-objects "git+$GH/fs-types?tag=v0.0.2#cccccccccccccccccccccccccccccccccccccccc" >"$tmp/work/fs-objects/Cargo.lock"
printf "strict = true\nonly_from_enabled = true\n" >"$tmp/work/fs-objects/.fs-lock-sources.toml"
commit_push fs-objects lock; tag_push fs-objects v0.0.2
tag_push fs-tasks v0.0.9
plan() { printf '{"erledigt":[%s]}' "$1"; }
e_ifc='{"repo":"fs-interfaces","tag":"v0.0.2","beleg":"b"}'; e_obj='{"repo":"fs-objects","tag":"v0.0.2","beleg":"b"}'; e_tsk='{"repo":"fs-tasks","tag":"v0.0.9","beleg":"b"}'
py "$here/lock-gleich" --plan "$(plan "$e_ifc")" --ausloeser v0.0.3 >"$tmp/o.json"; rc=$?
chk "lock-gleich: one Glied on the trigger tag from FreeSynergy/fs-types is green (rc 0)" [ "$rc" -eq 0 ] && chk "lock-gleich: one Glied on the trigger tag from FreeSynergy/fs-types is green (rc 0) [part 2]" [ "$(q "$tmp/o.json" 'd["glieder"][0]["fs_types_tags"]')" = "['v0.0.3']" ]
py "$here/lock-gleich" --plan "$(plan "$e_ifc,$e_obj")" --ausloeser v0.0.3 >"$tmp/o.json"; rc=$?
chk "lock-gleich: a Glied whose lock has another fs-types tag is red (rc 1)" [ "$rc" -eq 1 ] && chk "lock-gleich: a Glied whose lock has another fs-types tag is red (rc 1) [part 2]" [ "$(q "$tmp/o.json" '"v0.0.2" in d["befunde"][0]')" = True ]
py "$here/lock-gleich" --plan "$(plan "$e_tsk")" --ausloeser v0.0.3 >"$tmp/o.json"; rc=$?
chk "lock-gleich: a left rev pin and fs-types from the repo fs-interfaces are red" [ "$rc" -eq 1 ] && chk "lock-gleich: a left rev pin and fs-types from the repo fs-interfaces are red [part 2]" [ "$(q "$tmp/o.json" 'any("rev pin" in b for b in d["befunde"]) and d["glieder"][0]["lock_rc"] != 0')" = True ]
py "$here/lock-gleich" --plan "$(plan "$e_ifc")" --ausloeser v0.0.3 --env "FS_LOCK_ONLY_FROM=1" >"$tmp/o.json"; rc=$?
chk "lock-gleich: a weakened --env (no FS_LOCK_STRICT=1) is refused (rc 2)" [ "$rc" -eq 2 ]
py "$here/lock-gleich" --plan "$(plan "$e_ifc")" --ausloeser v0.0.3 --env "FS_LOCK_STRICT=1" >"$tmp/o.json"; rc=$?
chk "lock-gleich: a weakened --env (no FS_LOCK_ONLY_FROM=1) is refused (rc 2)" [ "$rc" -eq 2 ]
py "$here/lock-gleich" --plan '{"erledigt":[]}' --ausloeser v0.0.3 >"$tmp/o.json"; rc=$?
chk "lock-gleich: no done Glied is never green (rc 2)" [ "$rc" -eq 2 ]
py "$here/lock-gleich" --plan "$(plan '{"repo":"fs-interfaces","tag":"v9.9.9","beleg":"b"}')" --ausloeser v0.0.3 >"$tmp/o.json"; rc=$?
chk "lock-gleich: a tag that is not on the remote is no green (rc 2)" [ "$rc" -eq 2 ]
# a Glied at a tag without the strict tool file is red even when the lock itself is fine
newrepo fs-nolock; printf '[package]\nname = "x"\n' >"$tmp/work/fs-nolock/Cargo.toml"; lock fs-nolock "git+$GH/fs-types?tag=v0.0.3#dddddddddddddddddddddddddddddddddddddddd" >"$tmp/work/fs-nolock/Cargo.lock"
commit_push fs-nolock one; tag_push fs-nolock v0.0.1
nl='{"repo":"fs-nolock","tag":"v0.0.1","beleg":"b"}'
py "$here/lock-gleich" --plan "$(plan "$nl")" --ausloeser v0.0.3 >"$tmp/o.json"; rc=$?
chk "lock-gleich: a Glied without .fs-lock-sources.toml (strict, only_from) is red (rc 1)" [ "$rc" -eq 1 ] && chk "lock-gleich: a Glied without .fs-lock-sources.toml (strict, only_from) is red (rc 1) [part 2]" [ "$(q "$tmp/o.json" 'any("fs-lock-sources" in b for b in d["befunde"])')" = True ]
cp "$here/lock-gleich" "$tmp/mut/"
mutate "$here/lock-gleich" 'if sw.get("strict") is not True or' 'if False and' "$tmp/mut/lock-gleich" || bad "mutant lock 4 not applied"
py "$tmp/mut/lock-gleich" --plan "$(plan "$nl")" --ausloeser v0.0.3 >/dev/null; rc=$?
chk "mutant lock-gleich (tool file not checked) is caught by the missing-file case" [ "$rc" -ne 1 ]
mutate "$here/lock-gleich" 'if found - {a.ausloeser} or bad:' 'if False:' "$tmp/mut/lock-gleich" || bad "mutant lock 1 not applied"
py "$tmp/mut/lock-gleich" --plan "$(plan "$e_obj")" --ausloeser v0.0.3 >/dev/null; rc=$?
chk "mutant lock-gleich (version check removed) is caught by the other-version case" [ "$rc" -ne 1 ]
mutate "$here/lock-gleich" 'not all(r in env_pairs for r in REQUIRED)' 'False' "$tmp/mut/lock-gleich" || bad "mutant lock 2 not applied"
py "$tmp/mut/lock-gleich" --plan "$(plan "$e_ifc")" --ausloeser v0.0.3 --env "FS_LOCK_ONLY_FROM=1" >/dev/null; rc=$?
chk "mutant lock-gleich (required switches not enforced) is caught by the weakened-env case" [ "$rc" -ne 2 ]
mutate "$here/lock-gleich" 'if pins:' 'if False:' "$tmp/mut/lock-gleich" || bad "mutant lock 3 not applied"
py "$tmp/mut/lock-gleich" --plan "$(plan "$e_tsk")" --ausloeser v0.0.3 | python3 -I -c 'import json,sys; d=json.load(sys.stdin); sys.exit(0 if any("rev pin" in b for b in d["befunde"]) else 1)'; rc=$?
chk "mutant lock-gleich (rev pin check removed) is caught by the left-rev-pin case" [ "$rc" -ne 0 ]
wait; nfail="$(wc -l <"$tmp/.fails")"; if [ "$nfail" -eq 0 ] && [ "$fails" -eq 0 ]; then echo "all ok"; else echo "$nfail failed (counter file), $fails in the main shell"; exit 1; fi
