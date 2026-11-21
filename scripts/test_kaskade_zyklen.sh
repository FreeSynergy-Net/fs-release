#!/usr/bin/env bash
# Pattern: none (test).
# Self test of scripts/kaskade-zyklen (workflow pin-kaskade, step `zyklen-check`), no network: the acyclic fixture repos of scripts/kaskade_fixture.sh (work clones) are green,
# own checkout folders carry a 2-cycle (cyc-a <-> cyc-b, as in test_kaskade_plan.sh), a 3-cycle, a self edge and the fs-starter <-> fs-forms case through a repo that is no
# Glied; bad input is rc 2. MUTANTS: the cycle search disabled, a self edge ignored and the `rev` pins not read must each turn the test red (the cases are run against the
# mutant copy and the result must be RED: that proves the test bites).
#   bash scripts/test_kaskade_zyklen.sh
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

# co <name> [dep-repo:tag-or-rev ...]: a checkout folder with a Cargo.toml that pins the given FreeSynergy repos (tag, or rev when the value has 8+ hex chars)
co() {
  local n="$1"; shift; mkdir -p "$tmp/co/$n"; printf '[package]\nname = "%s"\nversion = "0.0.1"\n[dependencies]\n' "$n" >"$tmp/co/$n/Cargo.toml"
  local d; for d in "$@"; do
    if [[ "${d##*:}" =~ ^[0-9a-f]{8,}$ ]]; then printf '%s = { git = "%s/%s", rev = "%s" }\n' "${d%%:*}" "$GH" "${d%%:*}" "${d##*:}" >>"$tmp/co/$n/Cargo.toml"
    else printf '%s = { git = "%s/%s", tag = "%s" }\n' "${d%%:*}" "$GH" "${d%%:*}" "${d##*:}" >>"$tmp/co/$n/Cargo.toml"; fi
  done
}
Z="$here/kaskade-zyklen"
# ---- the acyclic fixture graph (fs-tasks -> fs-interfaces, fs-people; fs-people -> fs-interfaces; fs-interfaces, fs-objects -> fs-types) ----
w="$tmp/work"
py "$Z" --repos "$w/fs-types,$w/fs-interfaces,$w/fs-objects,$w/fs-people,$w/fs-tasks" >"$tmp/o.json"; rc=$?
chk "no cycle: rc 0" [ "$rc" -eq 0 ]
chk "no cycle: red false, no cycles" [ "$(q "$tmp/o.json" '(d["red"], d["cycles"])')" = "(False, [])" ]
chk "no cycle: 5 repos" [ "$(q "$tmp/o.json" 'd["repos"]')" = 5 ]
chk "no cycle: edges counted (interfaces 1, objects 1, people 1, tasks 2: interfaces and, by rev pin, people; the foreign source and the fs-types-via-interfaces pin add none)" [ "$(q "$tmp/o.json" 'd["edges"]')" = 5 ]
printf '%s\n' "# list file" "$w/fs-types" "$w/fs-interfaces  # with a comment" >"$tmp/list.txt"
py "$Z" --list "$tmp/list.txt" >"$tmp/o.json"; rc=$?
chk "--list FILE works (rc 0, 2 repos)" [ "$rc" -eq 0 ] && chk "--list FILE works (2 repos) [part 2]" [ "$(q "$tmp/o.json" 'd["repos"]')" = 2 ]
# ---- 2-cycle (the cyc-a / cyc-b mechanism of test_kaskade_plan.sh) ----
co cyc-a cyc-b:v0.0.1; co cyc-b cyc-a:v0.0.1
py "$Z" --repos "$tmp/co/cyc-a,$tmp/co/cyc-b" >"$tmp/o2.json"; rc=$?
chk "2-cycle: rc 1" [ "$rc" -eq 1 ]
chk "2-cycle: red, both repos listed in one cycle" [ "$(q "$tmp/o2.json" '(d["red"], d["cycles"])')" = "(True, [['cyc-a', 'cyc-b']])" ]
# ---- 3-cycle with a repo outside it ----
co c3-a c3-b:v0.0.1; co c3-b c3-c:v0.0.1 fs-types:v0.0.3; co c3-c c3-a:v0.0.1; co c3-out c3-a:v0.0.1
py "$Z" --repos "$tmp/co/c3-a,$tmp/co/c3-b,$tmp/co/c3-c,$tmp/co/c3-out" >"$tmp/o3.json"; rc=$?
chk "3-cycle: rc 1" [ "$rc" -eq 1 ]
chk "3-cycle: the three repos, not the repo that only points into it" [ "$(q "$tmp/o3.json" 'd["cycles"]')" = "[['c3-a', 'c3-b', 'c3-c']]" ]
# ---- self edge ----
co self-a self-a:v0.0.1; co self-ok fs-types:v0.0.3
py "$Z" --repos "$tmp/co/self-a,$tmp/co/self-ok" >"$tmp/o4.json"; rc=$?
chk "self edge: rc 1" [ "$rc" -eq 1 ]
chk "self edge: listed as a cycle of one repo" [ "$(q "$tmp/o4.json" 'd["cycles"]')" = "[['self-a']]" ]
# ---- fs-starter <-> fs-forms through a repo that is no Glied, pinned by rev ----
co fs-forms fs-rpc:v0.0.3; co fs-rpc fs-starter:0123456789ab; co fs-starter fs-forms:v0.0.1
py "$Z" --repos "$tmp/co/fs-forms,$tmp/co/fs-rpc,$tmp/co/fs-starter" >"$tmp/o5.json"; rc=$?
chk "fs-starter <-> fs-forms (rev pin, via fs-rpc): rc 1" [ "$rc" -eq 1 ]
chk "fs-starter <-> fs-forms: all three repos in the cycle" [ "$(q "$tmp/o5.json" 'sorted(d["cycles"][0])')" = "['fs-forms', 'fs-rpc', 'fs-starter']" ]
# ---- the default: the plan input (plan.offen) + root, repos mentioned by the pins are read too ----
printf '{"offen":[{"repo":"fs-forms"}]}' >"$tmp/plan.json"
py "$Z" --plan "$tmp/plan.json" --root "$tmp/co" >"$tmp/o6.json"; rc=$?
chk "plan default: a cycle through repos that are not in plan.offen is found (rc 1)" [ "$rc" -eq 1 ] && chk "plan default: a cycle through repos not in plan.offen [part 2]" [ "$(q "$tmp/o6.json" 'sorted(d["cycles"][0])')" = "['fs-forms', 'fs-rpc', 'fs-starter']" ]
printf '{"offen":[{"repo":"fs-objects"}]}' >"$tmp/plan2.json"
py "$Z" --plan "$tmp/plan2.json" --root "$w" >"$tmp/o7.json"; rc=$?
chk "plan default: the acyclic fixture through the plan input gives no cycle (rc 0)" [ "$rc" -eq 0 ] && chk "plan default: every mentioned repo of the fixture has a checkout, so ungelesen is empty [part 2]" [ "$(q "$tmp/o7.json" 'd["ungelesen"]')" = "[]" ]
# a REAL missing checkout: fs-lonely pins fs-missing, there is no folder for fs-missing
co fs-lonely fs-missing:v0.0.1
printf '{"offen":[{"repo":"fs-lonely"}]}' >"$tmp/plan3.json"
py "$Z" --plan "$tmp/plan3.json" --root "$tmp/co" >"$tmp/o8.json"; rc=$?
chk "plan default: a pinned repo without checkout is listed as ungelesen and is no cycle (rc 0)" [ "$rc" -eq 0 ]
chk "plan default: ungelesen names fs-missing" [ "$(q "$tmp/o8.json" 'd["ungelesen"]')" = "['fs-missing']" ]
# ---- --prefix: the checkouts are folders pk-<repo> in the root (the worktrees folder of the cascade) ----
mkdir -p "$tmp/pk"; for r in fs-forms fs-rpc fs-starter; do cp -r "$tmp/co/$r" "$tmp/pk/pk-$r"; done
py "$Z" --plan "$tmp/plan.json" --root "$tmp/pk" --prefix pk- >"$tmp/o9.json"; rc=$?
chk "--prefix pk-: a cycle through pk-<repo> folders is found (rc 1)" [ "$rc" -eq 1 ]
chk "--prefix pk-: the cycle names the repos, not the prefixed folders" [ "$(q "$tmp/o9.json" 'sorted(d["cycles"][0])')" = "['fs-forms', 'fs-rpc', 'fs-starter']" ]
py "$Z" --plan "$tmp/plan.json" --root "$tmp/pk" >"$tmp/o.json"; rc=$?
chk "--prefix missing: the pk- folders are not found without the prefix (rc 2)" [ "$rc" -eq 2 ]
# ---- bad input ----
py "$Z" >"$tmp/o.json" 2>/dev/null; rc=$?
chk "bad input: no arguments is rc 2" [ "$rc" -eq 2 ]
py "$Z" --repos "$tmp/does-not-exist" >"$tmp/o.json"; rc=$?
chk "bad input: a folder that does not exist is rc 2" [ "$rc" -eq 2 ]
mkdir -p "$tmp/co/broken"; printf '[dependencies\n' >"$tmp/co/broken/Cargo.toml"
py "$Z" --repos "$tmp/co/broken" >"$tmp/o.json"; rc=$?
chk "bad input: a broken Cargo.toml is rc 2" [ "$rc" -eq 2 ]
py "$Z" --plan '{"nope":1}' --root "$tmp/co" >"$tmp/o.json"; rc=$?
chk "bad input: a plan without offen is rc 2" [ "$rc" -eq 2 ]
py "$Z" --plan "$tmp/plan.json" --root "$tmp/nowhere" >"$tmp/o.json"; rc=$?
chk "bad input: a plan repo without checkout is rc 2" [ "$rc" -eq 2 ]
chk "bad input: the result says skipped and red (never green)" [ "$(q "$tmp/o.json" '(d["status"], d["red"])')" = "('skipped', True)" ]

# ---- MUTANTS: each must make a case red (the same cases against a mutated copy) ----
mutant_red() { # <label> <old> <new>: the mutant copy must turn at least one of the three cycle cases (2-cycle, self edge, rev pin) from red to not-red
  mutate "$Z" "$2" "$3" "$tmp/mut/kaskade-zyklen" || { bad "mutant $1 not applied"; return; }
  local m="$tmp/mut/kaskade-zyklen" r1 r2 r3
  py "$m" --repos "$tmp/co/cyc-a,$tmp/co/cyc-b" >/dev/null; r1=$?
  py "$m" --repos "$tmp/co/self-a" >/dev/null; r2=$?
  py "$m" --repos "$tmp/co/fs-forms,$tmp/co/fs-rpc,$tmp/co/fs-starter" >/dev/null; r3=$?
  chk "mutant ($1) is caught: a cycle case no longer gives rc 1 (rcs $r1 $r2 $r3)" [ "$r1$r2$r3" != 111 ]
}
mutant_red "cycle search disabled" 'cyc = [sorted(c, key=order.get) for c in sccs(graph) if len(c) > 1 or c[0] in graph[c[0]]]' 'cyc = []'
mutant_red "self edge ignored" 'if len(c) > 1 or c[0] in graph[c[0]]]' 'if len(c) > 1]'
# a mutant that calls everything a cycle must turn the GREEN case red (the other direction)
mutate "$Z" 'cyc = [sorted(c, key=order.get) for c in sccs(graph) if len(c) > 1 or c[0] in graph[c[0]]]' 'cyc = [sorted(c, key=order.get) for c in sccs(graph)]' "$tmp/mut/kaskade-zyklen" || bad "mutant all-cycle not applied"
m="$tmp/mut/kaskade-zyklen"
py "$m" --repos "$w/fs-types,$w/fs-interfaces,$w/fs-objects,$w/fs-people,$w/fs-tasks" >/dev/null; rc=$?
chk "mutant (every repo is a cycle) turns the acyclic case red (rc 1)" [ "$rc" -eq 1 ]
mutate "$Z" 'if d["owner"].lower() == kcargo.ORG.lower() and not kcargo.is_fremd(d, fremd):' 'if d["owner"].lower() == kcargo.ORG.lower() and not kcargo.is_fremd(d, fremd) and not d["rev"]:' "$m" || bad "mutant rev not applied"
py "$m" --repos "$tmp/co/fs-forms,$tmp/co/fs-rpc,$tmp/co/fs-starter" >/dev/null; rc=$?
chk "mutant (rev pins not read) misses the fs-starter cycle, so the rev case bites (rc $rc, not 1)" [ "$rc" -eq 0 ]
mutate "$Z" 'd = os.path.join(a.root, a.prefix + r)' 'd = os.path.join(a.root, r)' "$m" || bad "mutant prefix not applied"
py "$m" --plan "$tmp/plan.json" --root "$tmp/pk" --prefix pk- >/dev/null; rc=$?
chk "mutant (prefix ignored) turns the --prefix case red (rc $rc, not 1)" [ "$rc" -eq 2 ]
# the failure counter of THIS script (shared test scripts/test_kaskade_counter.sh)
# shellcheck source=scripts/test_kaskade_counter.sh
source "$here/test_kaskade_counter.sh"; counter_selftest "${BASH_SOURCE[0]}"
wait; nfail="$(wc -l <"$tmp/.fails")"; if [ "$nfail" -eq 0 ] && [ "$fails" -eq 0 ]; then echo "all ok"; else echo "$nfail failed (counter file), $fails in the main shell"; exit 1; fi
