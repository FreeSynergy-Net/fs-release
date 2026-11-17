#!/usr/bin/env bash
# Self test of scripts/gates-komplett (workflow pin-kaskade) with a FAKE build slot and a fake gates.sh (no cargo): the scripts come from the highest fs-release tag the repo's CI pins
# (not main), the three runs (local, CI=true, flat clone without tags) go one after the other through the build slot with CARGO_BUILD_JOBS/BUILD_PRIORITY, a red run stops the rest
# and is red with its rc, the rc lines and error lines are listed whole, a fmt that changes files is red. Mutants: the rc ignored, the oldest tag taken.
#   bash scripts/test_kaskade_gates.sh
set -uo pipefail
here="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
mkdir -p "$here/../.test-tmp"; tmp="$(TMPDIR="${TEST_TMPDIR:-$here/../.test-tmp}" mktemp -d)"  # never /tmp (RAM, rule [PLATTE])
trap 'rm -rf "$tmp"; rmdir "$here/../.test-tmp" 2>/dev/null' EXIT
fails=0
ok() { echo "ok   $1"; }
bad() { echo "FAIL $1"; fails=$((fails + 1)); }
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
printf '%s prio=%s jobs=%s tags=%s root=%s\n' "$mode" "$BUILD_PRIORITY" "$CARGO_BUILD_JOBS" "$(git -C "$1" tag | wc -l)" "$1" >>"$FAKE_OUT"
echo "gate fmt: rc=0"
if [ "${FAKE_FAIL:-}" = "$mode" ]; then echo "error: boom in $mode"; echo "gates FAILED: line 3: rc=1" >&2; exit 1; fi
echo "gate test: rc=0"; echo "gates ok"
G
git -C "$rel" add -A; git -C "$rel" commit -qm new; git -C "$rel" tag v0.0.10
printf '#!/usr/bin/env bash\nexec "$@"\n' >"$tmp/slot.sh"; chmod +x "$tmp/slot.sh"
app="$tmp/app"; git init -q -b main "$app"; mkdir -p "$app/.github/workflows"
printf 'jobs:\n  a:\n    uses: FreeSynergy/fs-release/.github/workflows/check.yml@v0.0.9\n  b:\n    uses: FreeSynergy/fs-release/.github/workflows/check.yml@v0.0.10\n' >"$app/.github/workflows/check.yml"
echo x >"$app/a.txt"; git -C "$app" add -A; git -C "$app" commit -qm one; git -C "$app" tag v0.0.1
export FS_RELEASE_REPO="$rel" FS_GATES_BUILD_SLOT="$tmp/slot.sh" FS_GATES_FMT=true FS_GATES_SCRATCH="$tmp/scratch" FAKE_OUT="$tmp/out.txt"
run() { python3 -I "$here/gates-komplett" --repo-dir "$app" --log-dir "$tmp/logs" "$@"; }

: >"$FAKE_OUT"; run >"$tmp/o.json"; rc=$?
chk "gates: all three runs green (rc 0), scripts of the highest pinned tag v0.0.10" [ "$rc" -eq 0 ] && [ "$(q "$tmp/o.json" '(d["skripte"]["tag"], [l["name"] for l in d["laeufe"]])')" = "('v0.0.10', ['lokal', 'ci', 'flach-ohne-tags'])" ]
chk "gates: CARGO_BUILD_JOBS=4 and BUILD_PRIORITY=0 reach every run" [ "$(grep -c 'prio=0 jobs=4' "$FAKE_OUT")" = 3 ]
chk "gates: run 1 local, runs 2 and 3 with CI=true" [ "$(cut -d' ' -f1 "$FAKE_OUT" | paste -sd, -)" = "local,ci,ci" ]
chk "gates: the flat clone has NO tags, the checkout has one" [ "$(grep -o 'tags=[0-9]*' "$FAKE_OUT" | paste -sd, -)" = "tags=1,tags=1,tags=0" ]
chk "gates: the rc lines of every gate are listed whole" [ "$(q "$tmp/o.json" 'all("gate test: rc=0" in l["gates"] for l in d["laeufe"])')" = True ]
: >"$FAKE_OUT"; FAKE_FAIL=ci run --jobs 2 --prioritaet 3 >"$tmp/o.json"; rc=$?
chk "gates: a red CI run is red with its rc (rc 1)" [ "$rc" -eq 1 ] && [ "$(q "$tmp/o.json" '(d["laeufe"][1]["rc"], d["red"])')" = "(1, True)" ]
chk "gates: the error lines are listed, never cut" [ "$(q "$tmp/o.json" '"error: boom in ci" in d["laeufe"][1]["fehler"] and any("FAILED" in x for x in d["laeufe"][1]["fehler"])')" = True ]
chk "gates: after a red run the rest is 'nicht gelaufen' (no flat run)" [ "$(q "$tmp/o.json" 'd["laeufe"][2]["rc"]')" = None ] && [ "$(wc -l <"$FAKE_OUT")" = 2 ]
chk "gates: jobs and priority are passed on" grep -q 'prio=3 jobs=2' "$FAKE_OUT"
run --ohne-flachprobe >"$tmp/o.json"
chk "gates: --ohne-flachprobe runs two" [ "$(q "$tmp/o.json" 'len(d["laeufe"])')" = 2 ]
FS_GATES_FMT="sh -c 'echo y >>a.txt'" run >"$tmp/o.json"; rc=$?
git -C "$app" checkout -q a.txt
chk "gates: a fmt that changes files is red and no gate runs (commit the formatted tree first)" [ "$rc" -eq 1 ] && [ "$(q "$tmp/o.json" '(d["fmt"]["geaendert"], d["laeufe"][0]["rc"])')" = "(True, None)" ]
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
if [ "$fails" -eq 0 ]; then echo "all ok"; else echo "$fails failed"; exit 1; fi
