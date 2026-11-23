#!/usr/bin/env bash
# Pattern: none (test).
# Self test of scripts/gates-komplett (workflow pin-kaskade) with a FAKE build slot and a fake gates.sh (no cargo): the scripts come from the highest fs-release tag the repo's CI pins
# (not main), the three runs (local, CI=true, flat clone without tags) go one after the other through the build slot with CARGO_BUILD_JOBS/BUILD_PRIORITY, a red run stops the rest
# and is red with its rc, the NAMED EXCEPTION (fs-release, flat run, one test) is applied only there and always visible, the rc lines and error lines are listed whole, a fmt that changes files is red. Mutants: the rc ignored, the oldest tag taken.
#   bash scripts/test_kaskade_gates.sh
set -uo pipefail
here="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
mkdir -p "$here/../.test-tmp"; tmp="$(TMPDIR="${TEST_TMPDIR:-$here/../.test-tmp}" mktemp -d)"  # never /tmp (RAM, rule [PLATTE])
trap 'rm -rf "$tmp"; rmdir "$here/../.test-tmp" 2>/dev/null' EXIT
fails=0
: >"$tmp/.fails"  # every failure is ALSO written to this file: a failure in a subshell, pipe or background job cannot be lost (Opus N4)
ok() { echo "ok   $1"; }
bad() { echo "FAIL $1"; fails=$((fails + 1)); echo "$1" >>"$tmp/.fails"; }
chk() { local name="$1"; shift; if "$@"; then ok "$name"; else bad "$name"; fi; }
export GIT_AUTHOR_NAME=t GIT_AUTHOR_EMAIL=t@t GIT_COMMITTER_NAME=t GIT_COMMITTER_EMAIL=t@t
q() { python3 -I -c 'import json,sys; d=json.load(open(sys.argv[1])); print(eval(sys.argv[2]))' "$1" "$2"; }

# fs-release stand-in: v0.0.9 has a gates.sh that FAILS, v0.0.10 one that works (the CI of `app` pins both, the highest counts)
rel="$tmp/fs-release"; git init -q -b main "$rel"; mkdir -p "$rel/scripts"; echo 1.90.0 >"$rel/RUST_VERSION"
printf '#!/usr/bin/env bash\necho "gate fmt: rc=0"; exit 1\n' >"$rel/scripts/gates.sh"; git -C "$rel" add -A; git -C "$rel" commit -qm old; git -C "$rel" tag v0.0.9
cat >"$rel/scripts/gates.sh" <<'G'
#!/usr/bin/env bash
# fake: records the mode in $FAKE_OUT, fails in the mode named by $FAKE_FAIL (local | ci)
mode=local; [ "${CI:-}" = true ] && mode=ci
printf '%s prio=%s jobs=%s tags=%s root=%s target=%s skip=%s\n' "$mode" "$BUILD_PRIORITY" "$CARGO_BUILD_JOBS" "$(git -C "$1" tag | wc -l)" "$1" "${CARGO_TARGET_DIR:-}" "${FS_GATES_SKIP_TEST:-}" >>"$FAKE_OUT"
echo "gate fmt: rc=0"
if [ "${FAKE_FAIL:-}" = "$mode" ]; then echo "error: boom in $mode"; echo "gates FAILED: line 3: rc=1" >&2; exit 1; fi
echo "gate test: rc=0"; echo "gates ok"
G
git -C "$rel" add -A; git -C "$rel" commit -qm new; git -C "$rel" tag v0.0.10
printf '#!/usr/bin/env bash\nexec "$@"\n' >"$tmp/slot.sh"; chmod +x "$tmp/slot.sh"
app="$tmp/app"; git init -q -b main "$app"; mkdir -p "$app/.github/workflows"
printf 'jobs:\n  a:\n    uses: FreeSynergy-Net/fs-release/.github/workflows/check.yml@v0.0.9\n  b:\n    uses: FreeSynergy-Net/fs-release/.github/workflows/check.yml@v0.0.10\n' >"$app/.github/workflows/check.yml"
echo x >"$app/a.txt"; git -C "$app" add -A; git -C "$app" commit -qm one; git -C "$app" tag v0.0.1
export FS_RELEASE_REPO="$rel" FS_GATES_BUILD_SLOT="$tmp/slot.sh" FS_GATES_FMT=true FS_GATES_SCRATCH="$tmp/scratch" FAKE_OUT="$tmp/out.txt"
# the remote base for the deadline check (ls-remote of fs-release): a local bare repo without the tag v0.0.12 (never the network)
base="$tmp/base"; git init -q --bare -b main "$base/fs-release"; export FS_KASKADE_BASE="file://$base"
export CI=true CARGO_TERM_COLOR=always  # a caller in CI mode must not leak into the local run
unset CARGO_TARGET_DIR  # the real run (build slot) has it set; the cases below look at the target of the tool only
export FS_GATES_SKIP_TEST=leaked  # nor a skip of the caller (the real flat run of fs-release has it set): only the named exception may set it
run() { python3 -I "$here/gates-komplett" --repo-dir "$app" --log-dir "$tmp/logs" "$@"; }

: >"$FAKE_OUT"; run >"$tmp/o.json"; rc=$?
chk "gates: all three runs green (rc 0), scripts of the highest pinned tag v0.0.10" [ "$rc" -eq 0 ] && chk "gates: all three runs green (rc 0), scripts of the highest pinned tag v0.0.10 [part 2]" [ "$(q "$tmp/o.json" '(d["skripte"]["tag"], [l["name"] for l in d["laeufe"]])')" = "('v0.0.10', ['lokal', 'ci', 'flach-ohne-tags'])" ]
chk "gates: CARGO_BUILD_JOBS=4 and BUILD_PRIORITY=0 reach every run" [ "$(grep -c 'prio=0 jobs=4' "$FAKE_OUT")" = 3 ]
chk "gates: run 1 local, runs 2 and 3 with CI=true" [ "$(cut -d' ' -f1 "$FAKE_OUT" | paste -sd, -)" = "local,ci,ci" ]
chk "gates: the flat clone has NO tags, the checkout has one" [ "$(grep -o 'tags=[0-9]*' "$FAKE_OUT" | paste -sd, -)" = "tags=1,tags=1,tags=0" ]
chk "gates: the rc lines of every gate are listed whole" [ "$(q "$tmp/o.json" 'all("gate test: rc=0" in l["gates"] for l in d["laeufe"])')" = True ]
: >"$FAKE_OUT"; FAKE_FAIL=ci run --jobs 2 --prioritaet 3 >"$tmp/o.json"; rc=$?
chk "gates: a red CI run is red with its rc (rc 1)" [ "$rc" -eq 1 ] && chk "gates: a red CI run is red with its rc (rc 1) [part 2]" [ "$(q "$tmp/o.json" '(d["laeufe"][1]["rc"], d["red"])')" = "(1, True)" ]
chk "gates: the error lines are listed, never cut" [ "$(q "$tmp/o.json" '"error: boom in ci" in d["laeufe"][1]["fehler"] and any("FAILED" in x for x in d["laeufe"][1]["fehler"])')" = True ]
chk "gates: after a red run the rest still runs ([GATES-SAMMELLISTE]: no stop at the first red; the flat run is red too, rc 1)" [ "$(q "$tmp/o.json" 'd["laeufe"][2]["rc"]')" = 1 ] && chk "gates: after a red run the rest still runs [part 2]" [ "$(wc -l <"$FAKE_OUT")" = 3 ]
chk "gates: jobs and priority are passed on" grep -q 'prio=3 jobs=2' "$FAKE_OUT"
run --ohne-flachprobe >"$tmp/o.json"
chk "gates: --ohne-flachprobe runs two" [ "$(q "$tmp/o.json" 'len(d["laeufe"])')" = 2 ]
FS_GATES_FMT="sh -c 'echo y >>a.txt'" run >"$tmp/o.json"; rc=$?
git -C "$app" checkout -q a.txt
chk "gates: a fmt that changes files is red (commit the formatted tree first) and the gates still run to collect everything" [ "$rc" -eq 1 ] && chk "gates: a fmt that changes files is red [part 2]" [ "$(q "$tmp/o.json" '(d["fmt"]["geaendert"], d["laeufe"][0]["rc"])')" = "(True, 0)" ]
run --jobs 0 >"$tmp/o.json"; rc=$?
chk "gates: bad jobs is bad input (rc 2)" [ "$rc" -eq 2 ]
run --skripte-tag v9.9.9 >"$tmp/o.json"; rc=$?
chk "gates: a tag that fs-release does not have is bad input (rc 2), never main" [ "$rc" -eq 2 ]
# mutants
mkdir -p "$tmp/mut"; cp -r "$here/kaskade_lib" "$tmp/mut/"
python3 -I - "$here/gates-komplett" "$tmp/mut/gates-komplett" <<'PY'
import sys
t = open(sys.argv[1], encoding="utf-8").read()
old = 'if laeufe[-1]["rc"] != 0:'
assert old in t
open(sys.argv[2], "w", encoding="utf-8").write(t.replace(old, "if False:", 1))
PY
FAKE_FAIL=ci python3 -I "$tmp/mut/gates-komplett" --repo-dir "$app" --log-dir "$tmp/logs" >"$tmp/o.json"; rc=$?
chk "mutant gates (rc of a run ignored) is caught by the red-run case" [ "$rc" -ne 1 ]
python3 -I - "$here/gates-komplett" "$tmp/mut/gates-komplett" <<'PY'
import sys
t = open(sys.argv[1], encoding="utf-8").read()
old = "max(tags, key=kgit.version_key)"
assert old in t
open(sys.argv[2], "w", encoding="utf-8").write(t.replace(old, "min(tags, key=kgit.version_key)", 1))
PY
python3 -I "$tmp/mut/gates-komplett" --repo-dir "$app" --log-dir "$tmp/logs" >"$tmp/o.json"; rc=$?
chk "mutant gates (oldest pinned tag taken) is caught by the highest-tag case" [ "$rc" -ne 0 ]
# mutate <script> <old> <new> <out>: a copy with ONE exact text replaced; fails when the text is not found (a mutation that changes nothing is a test error)
mutate() {
  python3 -I - "$@" <<'PY'
import sys
src, old, new, out = sys.argv[1:5]
t = open(src, encoding="utf-8").read()
if old not in t:
    sys.exit(f"mutation text not found in {src}: {old!r}")
open(out, "w", encoding="utf-8").write(t.replace(old, new, 1))
PY
}
# the list of named exceptions is EMPTY since v0.0.12 (the flat run of fs-release fetches its tags); the MECHANISM is tested with a sample entry in an own copy ($mech)
mech="$tmp/mech"; mkdir -p "$mech"; cp -r "$here/kaskade_lib" "$mech/"
python3 -I - "$here/gates-komplett" "$mech/gates-komplett" <<'PY'
import sys
t = open(sys.argv[1], encoding="utf-8").read()
old = "AUSNAHMEN = []\n"
assert t.count(old) == 1
new = 'AUSNAHMEN = [{"name": "flach-changelog-tags", "repo": "fs-release", "lauf": "flach-ohne-tags", "bis": "v0.0.12", "test": "released_changelog_sections_equal_their_tags", "grund": "sample entry of the self test"}]\n'
open(sys.argv[2], "w", encoding="utf-8").write(t.replace(old, new, 1))
PY
# ---- the named exception (c0): only repo fs-release, only the flat run, only the one test; visible, never green for that test ----
exrun() { python3 -I "$mech/gates-komplett" --repo-dir "$1" --log-dir "$tmp/logs" 2>"$tmp/err.txt"; }
T=released_changelog_sections_equal_their_tags
: >"$FAKE_OUT"; exrun "$app" >"$tmp/o.json"
chk "exception: a repo that is not fs-release (origin-less, folder app) gets no skip in any run" [ "$(grep -c 'skip=$' "$FAKE_OUT")" = 3 ] && chk "exception: a repo that is not fs-release (origin-less, folder app) gets no skip in any run [part 2]" [ "$(q "$tmp/o.json" 'd["ausnahmen"]')" = "[]" ]
for r in fs-release fs-tasks; do
  git clone -q "$app" "$tmp/wt-$r"; git -C "$tmp/wt-$r" remote set-url origin "https://github.com/FreeSynergy-Net/$r.git"
done
: >"$FAKE_OUT"; exrun "$tmp/wt-fs-tasks" >"$tmp/o.json"
chk "exception: the repo name comes from origin (fs-tasks in a folder wt-fs-tasks): no skip" [ "$(grep -c 'skip=$' "$FAKE_OUT")" = 3 ]
: >"$FAKE_OUT"; exrun "$tmp/wt-fs-release" >"$tmp/o.json"; rc=$?
chk "exception: fs-release (origin name, folder wt-fs-release): local and CI run without skip" [ "$(grep -c '^local .*skip=$\|^ci .*tags=1.*skip=$' "$FAKE_OUT")" = 2 ]
chk "exception: fs-release flat run (the third) gets exactly the one test as skip" [ "$(tail -n 1 "$FAKE_OUT" | grep -c "tags=1.*skip=$T\$")" = 1 ] && chk "exception: fs-release flat run (the third) gets exactly the one test as skip [part 2]" [ "$(grep -c "skip=$T\$" "$FAKE_OUT")" = 1 ]
chk "exception: rc 0, and the result lists it as applied and NOT green for the test" [ "$rc" -eq 0 ] && chk "exception: rc 0, and the result lists it as applied and NOT green for the test [part 2]" [ "$(q "$tmp/o.json" '[(x["test"], x["angewendet"], x["gruen_fuer_test"]) for x in d["ausnahmen"]]')" = "[('$T', True, False)]" ]
chk "exception: it is printed visibly on every run (stderr names test, exception and reason)" grep -q "SKIPPED test $T by named exception flach-changelog-tags, reason:" "$tmp/err.txt"
chk "exception: the flat target is named after the repo (flach under target/fs-release), not the folder" [ "$(q "$tmp/o.json" '[l["log"] for l in d["laeufe"] if l["name"]=="flach-ohne-tags"][0]')" = "$tmp/logs/fs-release-gates-flach-ohne-tags.log" ]
# the REAL list (empty): the flat run of fs-release has the tags (own fetch step) and no skip; the flat run of every other repo stays without tags
rexrun() { python3 -I "$here/gates-komplett" --repo-dir "$1" --log-dir "$tmp/logs" 2>"$tmp/err4.txt"; }
: >"$FAKE_OUT"; rc=0; rexrun "$tmp/wt-fs-release" >"$tmp/o4.json" || rc=$?
chk "real list: the AUSNAHMEN list is empty and nothing is skipped" [ "$(q "$tmp/o4.json" '(d["ausnahmen_liste"], d["ausnahmen"])')" = "([], [])" ] && chk "real list: no SKIPPED line on stderr" [ "$(grep -c 'SKIPPED' "$tmp/err4.txt")" = 0 ]
chk "real list: the flat run of fs-release has the tags (own fetch step), no skip, rc 0" [ "$rc" -eq 0 ] && chk "real list: the flat run of fs-release has the tags (own fetch step), no skip, rc 0 [part 2]" [ "$(tail -n 1 "$FAKE_OUT" | grep -c 'tags=1.*skip=$')" = 1 ]
: >"$FAKE_OUT"; rexrun "$tmp/wt-fs-tasks" >/dev/null
chk "real list: the flat run of another repo stays WITHOUT tags" [ "$(tail -n 1 "$FAKE_OUT" | grep -c 'tags=0.*skip=$')" = 1 ]
mutate "$here/gates-komplett" 'if repo == "fs-release":  #' 'if False:  #' "$tmp/mut/gates-komplett" || bad "mutant no-fetch not applied"
: >"$FAKE_OUT"; python3 -I "$tmp/mut/gates-komplett" --repo-dir "$tmp/wt-fs-release" --log-dir "$tmp/logs" >/dev/null 2>&1
chk "mutant real list (no tag fetch in the flat run of fs-release) is caught by the tags case" [ "$(tail -n 1 "$FAKE_OUT" | grep -c 'tags=1')" = 0 ]
# mutants: widen the exception to every repo / every run, make it silent, count it green
mx() { # <old> <new> <label> <check expression over the 3 case outputs>
  mutate "$mech/gates-komplett" "$1" "$2" "$tmp/mut/gates-komplett" || bad "mutant $3 not applied"
}
mx 'x["repo"] == repo and x["lauf"] == lauf' 'x["lauf"] == lauf' "exception-all-repos"
: >"$FAKE_OUT"; python3 -I "$tmp/mut/gates-komplett" --repo-dir "$app" --log-dir "$tmp/logs" >/dev/null 2>&1
chk "mutant exception (widened to every repo) is caught by the repo case" [ "$(grep -c 'skip=$' "$FAKE_OUT")" != 3 ]
mx 'x["repo"] == repo and x["lauf"] == lauf' 'x["repo"] == repo' "exception-all-runs"
: >"$FAKE_OUT"; python3 -I "$tmp/mut/gates-komplett" --repo-dir "$tmp/wt-fs-release" --log-dir "$tmp/logs" >/dev/null 2>&1
chk "mutant exception (widened to every run) is caught by the local/CI-run case" [ "$(grep -c '^local .*skip=$\|^ci .*tags=1.*skip=$' "$FAKE_OUT")" != 2 ]
mx 'print(f"gates-komplett: run' 'pass  # print(f"gates-komplett: run' "exception-silent"
python3 -I "$tmp/mut/gates-komplett" --repo-dir "$tmp/wt-fs-release" --log-dir "$tmp/logs" >/dev/null 2>"$tmp/err2.txt"
chk "mutant exception (not printed) is caught by the visibility case" [ "$(grep -c "SKIPPED test" "$tmp/err2.txt")" = 0 ]
mx '"gruen_fuer_test": False' '"gruen_fuer_test": True' "exception-green"
python3 -I "$tmp/mut/gates-komplett" --repo-dir "$tmp/wt-fs-release" --log-dir "$tmp/logs" >"$tmp/o2.json" 2>/dev/null
chk "mutant exception (counted green for the test) is caught by the not-green case" [ "$(q "$tmp/o2.json" '[x["gruen_fuer_test"] for x in d["ausnahmen"]]')" != "[False]" ]
# MX3: the exception is REPORTED as applied for a foreign repo
mutate "$mech/gates-komplett" '            ex = ausnahme_for(repo, name)
' '            ex = ausnahme_for(repo, name)
            if not ex:
                ausnahmen.append({**AUSNAHMEN[0], "angewendet": True, "gruen_fuer_test": False})
' "$tmp/mut/gates-komplett" || bad "mutant MX3 not applied"
python3 -I "$tmp/mut/gates-komplett" --repo-dir "$app" --log-dir "$tmp/logs" >"$tmp/o2.json" 2>/dev/null
chk "mutant exception (reported as applied for a foreign repo, MX3) is caught by the empty-ausnahmen case" [ "$(q "$tmp/o2.json" 'd["ausnahmen"]')" != "[]" ]
mutate "$mech/gates-komplett" ', "FS_GATES_SKIP_TEST")}' ')}' "$tmp/mut/gates-komplett" || bad "mutant skip-leak not applied"
: >"$FAKE_OUT"; python3 -I "$tmp/mut/gates-komplett" --repo-dir "$app" --log-dir "$tmp/logs" >/dev/null 2>&1
chk "mutant exception (a skip of the caller leaks into the runs) is caught by the no-skip case" [ "$(grep -c 'skip=$' "$FAKE_OUT")" != 3 ]
# ---- the deadline `bis` of the named exception (c0/user: not for ever) ----
: >"$FAKE_OUT"; exrun "$tmp/wt-fs-release" >"$tmp/o.json"
chk "deadline: every run says until when the exception holds (stderr and result field bis)" grep -q 'valid until fs-release v0.0.12' "$tmp/err.txt" && chk "deadline: the result carries bis" [ "$(q "$tmp/o.json" '(d["ausnahmen"][0]["bis"], d["ausnahmen_liste"][0]["bis"])')" = "('v0.0.12', 'v0.0.12')" ]
: >"$FAKE_OUT"; exrun "$app" >"$tmp/o.json"
chk "deadline: also a run of another repo prints it" grep -q 'valid until fs-release v0.0.12' "$tmp/err.txt"
# fs-release v0.0.12 tagged: the list is not empty -> RED, no gate runs
git clone -q "$base/fs-release" "$tmp/frw" 2>/dev/null; git -C "$tmp/frw" commit -q --allow-empty -m x; git -C "$tmp/frw" tag v0.0.12; git -C "$tmp/frw" push -q origin main v0.0.12 2>/dev/null
: >"$FAKE_OUT"; rc=0; exrun "$app" >"$tmp/o.json" || rc=$?
chk "deadline: fs-release v0.0.12 is tagged and the list is not empty: RED (rc 1)" [ "$rc" -eq 1 ] && chk "deadline: fs-release v0.0.12 is tagged and the list is not empty: RED (rc 1) [part 2]" [ "$(q "$tmp/o.json" '"EXPIRED" in d["befunde"][0]')" = True ] && chk "deadline: fs-release v0.0.12 is tagged and the list is not empty: RED (rc 1) [part 3]" [ "$(wc -l <"$FAKE_OUT")" = 3 ]
mutate "$mech/gates-komplett" 'if bis in tags:' 'if False:' "$tmp/mut/gates-komplett" || bad "mutant deadline-ignored not applied"
rc=0; python3 -I "$tmp/mut/gates-komplett" --repo-dir "$app" --log-dir "$tmp/logs" >/dev/null 2>&1 || rc=$?
chk "mutant deadline (tag check always false) is caught by the expired case" [ "$rc" -ne 1 ]
mutate "$mech/gates-komplett" 'out.append(f"named exception {x['"'"'name'"'"']} has EXPIRED' 'pass; (f"named exception {x['"'"'name'"'"']} has EXPIRED' "$tmp/mut/gates-komplett" || bad "mutant deadline-ignored 2 not applied"
rc=0; python3 -I "$tmp/mut/gates-komplett" --repo-dir "$app" --log-dir "$tmp/logs" >/dev/null 2>&1 || rc=$?
chk "mutant deadline (the deadline is ignored) is caught by the expired case" [ "$rc" -ne 1 ]
# MX1: the deadline is checked only AFTER the runs (the gates run although the exception has expired)
python3 -I - "$mech/gates-komplett" "$tmp/mut/gates-komplett" <<'PY'
import sys
t = open(sys.argv[1], encoding="utf-8").read()
old = "        befunde.extend(fristen())\n"
assert t.count(old) == 1 and t.count("    kio.emit({") == 1
t = t.replace(old, "").replace("    kio.emit({", "    befunde.extend(fristen())\n    kio.emit({", 1)
open(sys.argv[2], "w", encoding="utf-8").write(t)
PY
: >"$FAKE_OUT"; rc=0; python3 -I "$tmp/mut/gates-komplett" --repo-dir "$app" --log-dir "$tmp/logs" >/dev/null 2>&1 || rc=$?
chk "mutant deadline (checked only after the runs, MX1) is caught: a gate ran although the exception expired" [ "$(wc -l <"$FAKE_OUT")" != 0 ]
# the unreadable remote is red, never a guess
rc=0; FS_KASKADE_BASE="file://$tmp/nowhere" exrun "$app" >"$tmp/o.json" || rc=$?
chk "deadline: a remote that cannot be read is RED (never a guess)" [ "$rc" -eq 1 ] && chk "deadline: a remote that cannot be read is RED (never a guess) [part 2]" [ "$(q "$tmp/o.json" '"cannot check" in d["befunde"][0]')" = True ]
mutate "$mech/gates-komplett" 'out.append(f"cannot check whether' 'pass; (f"cannot check whether' "$tmp/mut/gates-komplett" || bad "mutant unreadable not applied"
rc=0; FS_KASKADE_BASE="file://$tmp/nowhere" python3 -I "$tmp/mut/gates-komplett" --repo-dir "$app" --log-dir "$tmp/logs" >/dev/null 2>&1 || rc=$?
chk "mutant deadline (unreadable remote swallowed) is caught by the unreadable case" [ "$rc" -ne 1 ]
# a list entry without `bis` is red; the mutant removes that check too
mutate "$mech/gates-komplett" '"bis": "v0.0.12", ' '' "$tmp/mut/gates-komplett" || bad "mutant no-bis not applied"
rc=0; python3 -I "$tmp/mut/gates-komplett" --repo-dir "$app" --log-dir "$tmp/logs" >"$tmp/o.json" 2>/dev/null || rc=$?
chk "deadline: an entry without bis is RED" [ "$rc" -eq 1 ] && chk "deadline: an entry without bis is RED [part 2]" [ "$(q "$tmp/o.json" '"no valid" in d["befunde"][0]')" = True ]
python3 -I - "$tmp/mut/gates-komplett" <<'PY'
import sys
p = sys.argv[1]
t = open(p, encoding="utf-8").read()
old = "if not isinstance(bis, str) or not kio.TAG.match(bis):"
assert old in t
open(p, "w", encoding="utf-8").write(t.replace(old, "if False:", 1))
PY
rc=0; python3 -I "$tmp/mut/gates-komplett" --repo-dir "$app" --log-dir "$tmp/logs" >/dev/null 2>&1 || rc=$?
chk "mutant deadline (a missing bis is accepted) is caught by the no-bis case" [ "$rc" -ne 1 ]
mutate "$mech/gates-komplett" 'print(f"gates-komplett: named exception {x['"'"'name'"'"']} ({x['"'"'repo'"'"']}, run' 'pass  # print(f"gates-komplett: named exception {x['"'"'name'"'"']} ({x['"'"'repo'"'"']}, run' "$tmp/mut/gates-komplett" || bad "mutant no-output not applied"
rc=0; python3 -I "$tmp/mut/gates-komplett" --repo-dir "$app" --log-dir "$tmp/logs" >/dev/null 2>"$tmp/err3.txt" || rc=$?
chk "mutant deadline (the until-output is missing) is caught by the output case" [ "$(grep -c 'valid until' "$tmp/err3.txt")" = 0 ]
git -C "$tmp/frw" push -q origin :refs/tags/v0.0.12 2>/dev/null  # the tag is gone again for the cases below
# per-clone flat target: two clones of the same repo never share it ([EIGENES-TARGET])
git clone -q "$app" "$tmp/wt2-fs-release"; git -C "$tmp/wt2-fs-release" remote set-url origin "https://github.com/FreeSynergy-Net/fs-release.git"
: >"$FAKE_OUT"; exrun "$tmp/wt-fs-release" >/dev/null; flat1="$(tail -n 1 "$FAKE_OUT" | grep -o 'target=[^ ]*')"
: >"$FAKE_OUT"; exrun "$tmp/wt2-fs-release" >/dev/null; flat2="$(tail -n 1 "$FAKE_OUT" | grep -o 'target=[^ ]*')"
chk "flat target: named after repo and clone folder (target/fs-release/wt-fs-release-flach)" [ "$flat1" = "target=$tmp/scratch/target/fs-release/wt-fs-release-flach" ]
chk "flat target: a second clone of the same repo has ITS OWN" [ "$flat2" = "target=$tmp/scratch/target/fs-release/wt2-fs-release-flach" ]
chk "flat target: the local and CI runs set no target of the tool" [ "$(grep -c '^local .*target= \|^ci .*target= ' "$FAKE_OUT")" = 2 ]
# K2 / MF: gates-komplett ignores a fmt that changes files (the result must still say geaendert=True and run NO gate)
mutate "$here/gates-komplett" '"geaendert": dirty != before[1]' '"geaendert": False' "$tmp/mut/gates-komplett" || bad "mutant MF not applied"
FS_GATES_FMT="sh -c 'echo y >>a.txt'" python3 -I "$tmp/mut/gates-komplett" --repo-dir "$app" --log-dir "$tmp/logs" >"$tmp/o2.json" 2>/dev/null; git -C "$app" checkout -q a.txt
chk "mutant MF (fmt change ignored) is caught by the fmt-changes-files case (both parts checked)" [ "$(q "$tmp/o2.json" '(d["fmt"]["geaendert"], d["laeufe"][0]["rc"])')" != "(True, None)" ]
# the failure counter of THIS script, tested in every form and with six mutants (shared test scripts/test_kaskade_counter.sh)
# shellcheck source=scripts/test_kaskade_counter.sh
source "$here/test_kaskade_counter.sh"; counter_selftest "${BASH_SOURCE[0]}"
wait; nfail="$(wc -l <"$tmp/.fails")"; if [ "$nfail" -eq 0 ] && [ "$fails" -eq 0 ]; then echo "all ok"; else echo "$nfail failed (counter file), $fails in the main shell"; exit 1; fi
