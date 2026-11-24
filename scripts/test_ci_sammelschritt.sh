#!/usr/bin/env bash
# Pattern: none (test).
# [GATES-SAMMELLISTE] in the CI jobs: a step that can find something runs with `continue-on-error: true` and an id, and the job ends with ONE collecting step (`if: always()`) that reads every
# `steps.<id>.outcome`, lists them (job summary) and turns the job red when one step was red. This test reads the real workflow files, RUNS the run block of every collecting step with
# made-up outcomes (all green, one red, two red, skipped, cancelled) and checks mutants of the step (missing, always green, first red only, one id not collected, no `always()`).
#   bash scripts/test_ci_sammelschritt.sh
set -uo pipefail
here="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
root="$(cd "$here/.." && pwd)"
mkdir -p "$root/.test-tmp"; tmp="$(TMPDIR="${TEST_TMPDIR:-$root/.test-tmp}" mktemp -d)"
trap 'rm -rf "$tmp"; rmdir "$root/.test-tmp" 2>/dev/null' EXIT
fails=0
ok() { echo "ok   $1"; }
bad() { echo "FAIL $1"; fails=$((fails + 1)); }

# check.py <dir with the workflow files>: structure + run of the collecting step; prints one line per finding; exit 1 when something is wrong
cat >"$tmp/check.py" <<'PY'
import os, re, subprocess, sys, tempfile
import yaml
root = sys.argv[1]
FILES = {"check.yml": ["check"], "_windows-check.yml": ["windows"], "branch-ci.yml": ["gates"], "lint.yml": ["actionlint", "changelog-released", "packaging-tests"]}
bad = []
for fname, jobs in FILES.items():
    doc = yaml.safe_load(open(os.path.join(root, ".github/workflows", fname), encoding="utf-8"))
    for jname in jobs:
        steps = doc["jobs"][jname]["steps"]
        ids = [s["id"] for s in steps if s.get("continue-on-error") is True and "id" in s]
        if any(s.get("continue-on-error") is True and "id" not in s for s in steps):
            bad.append(f"{fname}:{jname}: a continue-on-error step has no id")
        if not ids:
            bad.append(f"{fname}:{jname}: no step with continue-on-error")
            continue
        last = steps[-1]
        if last.get("if") != "always()" or "RESULTS" not in (last.get("env") or {}):
            bad.append(f"{fname}:{jname}: the last step is no collecting step with if: always()")
            continue
        results = last["env"]["RESULTS"]
        for ref in re.findall(r"steps\.([A-Za-z0-9_-]+)\.outcome", results):   # every collected id must itself run with continue-on-error (else a red step stops the later ones)
            st = [x for x in steps if x.get("id") == ref]
            if not st or st[0].get("continue-on-error") is not True:
                bad.append(f"{fname}:{jname}: collected step {ref} has no continue-on-error")
        for i in ids:
            if f"steps.{i}.outcome" not in results:
                bad.append(f"{fname}:{jname}: step {i} is not collected")
        # run the block with made-up outcomes
        def run(outcomes):
            env = dict(os.environ, RESULTS="\n".join(f"{i} {o}" for i, o in outcomes), GITHUB_STEP_SUMMARY=os.devnull)
            r = subprocess.run(["bash", "-c", last["run"]], env=env, capture_output=True, text=True)
            return r.returncode, r.stdout
        rc, out = run([(i, "success") for i in ids]); 
        if rc != 0: bad.append(f"{fname}:{jname}: all green but the collecting step is red")
        rc, out = run([(i, "skipped") for i in ids])
        if rc != 0: bad.append(f"{fname}:{jname}: all skipped but the collecting step is red")
        for k, i in enumerate(ids):
            rc, out = run([(j, "failure" if j == i else "success") for j in ids])
            if rc == 0 or f"- {i}: failure" not in out: bad.append(f"{fname}:{jname}: a red step {i} does not turn the job red / is not listed")
        rc, out = run([(i, "cancelled") if k == 0 else (i, "success") for k, i in enumerate(ids)])
        if rc == 0: bad.append(f"{fname}:{jname}: a cancelled step does not turn the job red")
        if len(ids) >= 2:
            rc, out = run([(i, "failure") for i in ids])
            if rc == 0 or not all(f"- {i}: failure" in out for i in ids): bad.append(f"{fname}:{jname}: two red steps are not both listed")
print("\n".join(bad))
sys.exit(1 if bad else 0)
PY
if out="$(python3 -I "$tmp/check.py" "$root" 2>&1)"; then ok "every CI job collects: continue-on-error + id per step, one collecting step (always) that reads every outcome, red on one red step, both red listed"; else bad "CI collecting steps: $out"; fi
chk_in() { python3 -I "$tmp/check.py" "$1" >"$tmp/out.txt" 2>&1; }

# mutants: each must turn the check red
mut() { # <name> <file> <python old> <python new>
  local name="$1" file="$2" old="$3" new="$4" m="$tmp/m-$RANDOM"
  mkdir -p "$m/.github/workflows"; cp "$root/.github/workflows/"*.yml "$m/.github/workflows/"
  if ! python3 -I - "$m/.github/workflows/$file" "$old" "$new" <<'PY'
import sys
p, old, new = sys.argv[1:4]
t = open(p, encoding="utf-8").read()
if old not in t:
    sys.exit(1)
open(p, "w", encoding="utf-8").write(t.replace(old, new, 1))
PY
  then bad "mutant $name: NOT APPLIED"; rm -rf "$m"; return; fi
  if chk_in "$m"; then bad "mutant $name SURVIVED"; else ok "mutant $name is red"; fi
  rm -rf "$m"
}
mut "collecting step always green"      branch-ci.yml 'exit "$red"' 'exit 0'
mut "only the first red step counts"    check.yml 'case "$outcome" in failure|cancelled) red=1 ;; esac' 'case "$outcome" in failure|cancelled) red=1; break ;; esac'
mut "a red step is not listed"          _windows-check.yml 'echo "- $label: $outcome"' ':'
mut "cancelled does not count"          lint.yml 'failure|cancelled) red=1' 'failure) red=1'
mut "one id is not collected"          _windows-check.yml '            clippy ${{ steps.clippy.outcome }}
' ''
mut "no always() on the collecting step" check.yml '        if: always()
        env:
          RESULTS' '        env:
          RESULTS'
mut "step without continue-on-error"    branch-ci.yml '        id: gates-of-this
        continue-on-error: true' '        id: gates-of-this'
mut "gates step not collected"          check.yml '            gates-fmt-clippy ${{ steps.gates-fmt-clippy.outcome }}
' ''
mut "lint packaging job without collector" lint.yml '          RESULTS: |
            no-composite-action' '          NOTHING: |
            no-composite-action'
if [ "$fails" -ne 0 ]; then echo; echo "test_ci_sammelschritt: $fails failures"; exit 1; fi
echo; echo "test_ci_sammelschritt: all ok"
