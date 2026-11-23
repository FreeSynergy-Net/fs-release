#!/usr/bin/env bash
# Pattern: none (test). Self test of the steps `Release plan inputs` + `Release plan probe` of .github/workflows/check.yml: the run block of the first step is taken out of the
# workflow and run on fixture workspaces with a made-up caller release.yml; its outputs go to the REAL actions/plan/plan.py (the env mapping is read from actions/plan/action.yml).
# Cases: the fs-forms v0.0.3 case (a new binary crate under crates/ that is neither in `binaries` nor in `ignore-binaries`) is red, ignore-binaries / binaries listed = green,
# no call of the release workflow or a non-plain value = no probe. Also: the defaults of the reader equal those of release.yml. Mutants of the block must each fail a case.
#   bash scripts/test_plan_probe_step.sh      (needs bash, python3 with yaml, cargo for `metadata`; no network)
set -uo pipefail
here="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
repo="$(cd "$here/.." && pwd)"
tmp="$(mktemp -d "${TEST_TMPDIR:-${TMPDIR:-/tmp}}/plan-probe.XXXXXX")"
trap 'rm -rf "$tmp"' EXIT
fails=0
ok() { echo "ok   $1"; }
bad() { echo "FAIL $1"; fails=$((fails + 1)); }

# the run block of the reader step (python gets the heredoc from the shell, exactly as on the runner) and its defaults
python3 -I - "$repo/.github/workflows/check.yml" "$tmp/reader.sh" <<'PY' || { echo "FAIL the reader step is not there exactly once"; exit 1; }
import sys, yaml
steps = yaml.safe_load(open(sys.argv[1]))["jobs"]["check"]["steps"]
hit = [s for s in steps if s.get("id") == "release-inputs"]
assert len(hit) == 1 and hit[0].get("continue-on-error") is True and "run" in hit[0]
open(sys.argv[2], "w", encoding="utf-8").write(hit[0]["run"])
PY
[ -s "$tmp/reader.sh" ] || { echo "FAIL no run block"; exit 1; }

# the plan.py side: runs plan.py with the env that actions/plan/action.yml builds from the inputs, here taken from the reader's outputs (name=value lines)
cat >"$tmp/plan_run.py" <<'PY'
import os, re, subprocess, sys, yaml
repo, outputs, cwd = sys.argv[1:4]
vals = dict(l.rstrip("\n").split("=", 1) for l in open(outputs, encoding="utf-8") if "=" in l)
step = yaml.safe_load(open(os.path.join(repo, "actions/plan/action.yml"), encoding="utf-8"))["runs"]["steps"]
step = next(x for x in step if x.get("id") == "plan")  # the step that runs plan.py (steps before it, e.g. tag-on-main, have no plan env)
env = dict(os.environ, GITHUB_OUTPUT=os.devnull)
for k, v in step["env"].items():
    m = re.fullmatch(r"\$\{\{ inputs\.([a-z-]+) \}\}", v)
    env[k] = vals.get(m.group(1), "")
r = subprocess.run(["python3", "-I", os.path.join(repo, "actions/plan/plan.py")], cwd=cwd, env=env, capture_output=True, text=True)
sys.stdout.write(r.stdout + r.stderr)
sys.exit(r.returncode)
PY

# mk DIR NAME [BIN]: a cargo package (one binary NAME or BIN)
mk() { mkdir -p "$1/src"; printf '[package]\nname = "%s"\nversion = "%s"\nedition = "2021"\nauthors = ["T <t@example.org>"]\ndescription = "d"\nlicense = "MIT"\nrepository = "https://example.org/r"\n' "$2" "$3" >"$1/Cargo.toml"; echo 'fn main() {}' >"$1/src/main.rs"; }
# fixture DIR: the fs-forms v0.0.3 shape: crates/fs-forms (the packaged binary, 0.3.1) + crates/fs-forms-run (a new binary crate, 0.4.0: the owner of a binary decides the tag)
fixture() { rm -rf "$1"; mk "$1/crates/fs-forms" fs-forms 0.3.1; mk "$1/crates/fs-forms-run" fs-forms-run 0.4.0; printf '[workspace]\nmembers = ["crates/fs-forms", "crates/fs-forms-run"]\nresolver = "2"\n' >"$1/Cargo.toml"; mkdir -p "$1/.github/workflows"; }
# release_yml DIR USES WITH-LINES...
release_yml() { local d=$1 u=$2; shift 2; { printf 'name: release\non:\n  push:\n    tags: ["v*"]\njobs:\n  release:\n    uses: %s\n    with:\n' "$u"; printf '      %s\n' "$@"; } >"$d/.github/workflows/release.yml"; }
REL=FreeSynergy/fs-release/.github/workflows/release.yml@v0.0.17
RELNEW=FreeSynergy-Net/fs-release/.github/workflows/release.yml@v0.0.17   # the new owner: the probe must really run (it was skipped silently before)

# run_case SCRIPT DIR -> prints "probe=<true|false|none> plan_rc=<rc>" and leaves the output in $tmp/case.out
run_case() {
  local script=$1 d=$2 rc=0
  : >"$tmp/gh_out"
  (cd "$d" && REPO_ROOT="$d" GITHUB_OUTPUT="$tmp/gh_out" bash -eo pipefail "$script" >"$tmp/case.out" 2>&1) || rc=$?
  local probe; probe=$(sed -n 's/^probe=//p' "$tmp/gh_out")
  cat "$tmp/gh_out" >>"$tmp/case.out"   # the reader's outputs are part of what the cases may look at
  [ "$rc" = 0 ] || { echo "probe=reader-rc$rc plan_rc=-"; return; }
  if [ "$probe" = true ]; then
    local prc=0; python3 -I "$tmp/plan_run.py" "$repo" "$tmp/gh_out" "$d" >>"$tmp/case.out" 2>&1 || prc=$?
    echo "probe=true plan_rc=$prc"
  else echo "probe=${probe:-none} plan_rc=-"; fi
}
# expect NAME SCRIPT DIR WANT [OUTPUT TEXT]
expect() {
  local got good=1; got=$(run_case "$2" "$3")
  [ "$got" = "$4" ] || good=0
  if [ -n "${5:-}" ] && ! grep -Fq -- "$5" "$tmp/case.out"; then good=0; fi
  if [ -n "${CASEQ:-}" ]; then [ "$good" = 1 ] || cases_fail=$((cases_fail + 1)); return 0; fi
  if [ "$good" = 1 ]; then ok "$1"; else bad "$1 (got: $got, wanted: $4${5:+, output with: $5})"; sed 's/^/     /' "$tmp/case.out"; fi
}

D="$tmp/ws" W3="$tmp/ws3"
mkdir -p "$W3/.github/workflows"
all_cases() { # <script>: every case against it
  local s=$1
  fixture "$D"; release_yml "$D" "$REL" "binaries: fs-forms" "kind: cli"
  expect "fs-forms-run case: a new binary crate that is not in binaries is red" "$s" "$D" "probe=true plan_rc=1" "binaries under crates/ are not packaged: fs-forms-run (fs-forms-run)"
  release_yml "$D" "$REL" "binaries: fs-forms" "ignore-binaries: fs-forms-run"
  expect "ignore-binaries names it: green" "$s" "$D" "probe=true plan_rc=0"
  release_yml "$D" "$REL" "binaries: fs-forms" "ignore-binaries: '*'"
  expect "ignore-binaries *: green" "$s" "$D" "probe=true plan_rc=0"
  release_yml "$D" "$REL" "binaries: 'fs-forms, fs-forms-run'" "app-id: net.freesynergy.Forms"
  expect "both binaries listed: green" "$s" "$D" "probe=true plan_rc=0"
  release_yml "$D" "$REL" "binaries: fs-forms-run" "ignore-binaries: fs-forms" "kind: daemon" "macos: false"
  expect "other kind, bool input, other owner (tag from ITS version): green" "$s" "$D" "probe=true plan_rc=0" "macos=false"
  fixture "$W3/app"; release_yml "$W3" "$REL" "binaries: fs-forms" "ignore-binaries: fs-forms-run" "working-directory: app"
  expect "working-directory is honoured: green" "$s" "$W3" "probe=true plan_rc=0" "working-directory=app"
  fixture "$D"
  release_yml "$D" "$REL" "binaries: fs-nope"
  expect "a binary no package has: red (the plan's own message)" "$s" "$D" "probe=true plan_rc=1" "no cargo package"
  release_yml "$D" "$REL" "kind: library"
  expect "kind library without binaries: green" "$s" "$D" "probe=true plan_rc=0"
  release_yml "$D" "$REL" "binaries: fs-forms" "ignore-binaries: fs-forms-run" "windows-targets: ''" "linux-targets: x86_64"
  expect "empty windows-targets stays empty: green" "$s" "$D" "probe=true plan_rc=0"
  release_yml "$D" "$REL" "binaries: fs-forms" "ignore-binaries: fs-forms-run" "linux-targets: arm9000"
  expect "an unknown target is red (inputs are really passed)" "$s" "$D" "probe=true plan_rc=1" "unknown linux target"
  release_yml "$D" "$RELNEW" "binaries: fs-forms" "kind: cli"
  expect "the NEW owner's release workflow is probed, not skipped silently (red: fs-forms-run)" "$s" "$D" "probe=true plan_rc=1" "fs-forms-run"
  release_yml "$D" "$RELNEW" "binaries: fs-forms" "ignore-binaries: fs-forms-run"
  expect "the new owner, nothing to complain about: green" "$s" "$D" "probe=true plan_rc=0"
  release_yml "$D" "FreeSynergy-Netx/fs-release/.github/workflows/release.yml@v0.0.17" "binaries: fs-forms"
  expect "a third owner that only looks like the new one: no probe" "$s" "$D" "probe=false plan_rc=-"
  release_yml "$D" "someone/else/.github/workflows/release.yml@main" "binaries: fs-forms"
  expect "a release.yml that does not call fs-release: no probe" "$s" "$D" "probe=false plan_rc=-"
  release_yml "$D" "$REL" 'binaries: ${{ vars.BINARIES }}'
  expect "an expression as input: no probe, with a warning" "$s" "$D" "probe=false plan_rc=-" "::warning::"
  release_yml "$D" "$REL" "binaries: fs-forms" "apt-packages: libx" "flatpak: true" "smoke: false"
  expect "inputs the plan does not read are ignored (red because of fs-forms-run)" "$s" "$D" "probe=true plan_rc=1" "fs-forms-run"
}
cases_fail=0
all_cases "$tmp/reader.sh"

# the defaults of the reader equal the defaults of release.yml (a drift would make the probe plan something else than the release)
python3 -I - "$repo/.github/workflows/check.yml" "$repo/.github/workflows/release.yml" <<'PY' && ok "the reader defaults equal the input defaults of release.yml" || bad "the reader defaults differ from release.yml"
import re, sys, yaml
chk = yaml.safe_load(open(sys.argv[1], encoding="utf-8"))
run = [s for s in chk["jobs"]["check"]["steps"] if s.get("id") == "release-inputs"][0]["run"]
ns = {}
exec(re.search(r"^DEFAULTS = \{.*?\}$", run, re.M | re.S).group(0), ns)
rel = yaml.safe_load(open(sys.argv[2], encoding="utf-8"))
inputs = (rel.get("on") or rel.get(True))["workflow_call"]["inputs"]
for k, v in ns["DEFAULTS"].items():
    d = inputs[k]["default"]
    d = ("true" if d else "false") if isinstance(d, bool) else str(d)
    assert d == v, f"{k}: release.yml {d!r}, reader {v!r}"
PY

# the probe step: the real plan action, pinned like the other internal uses, after the reader, and both collected
python3 -I - "$repo/.github/workflows/check.yml" "$repo/actions/plan/action.yml" <<'PY' && ok "the probe step uses actions/plan, passes every plan input and is collected" || bad "the probe step is wrong"
import re, sys, yaml
steps = yaml.safe_load(open(sys.argv[1], encoding="utf-8"))["jobs"]["check"]["steps"]
ids = [s.get("id") for s in steps]
assert ids.index("release-inputs") < ids.index("release-plan-probe"), "order"
p = steps[ids.index("release-plan-probe")]
assert p["continue-on-error"] is True and re.fullmatch(r"FreeSynergy(-Net)?/fs-release/actions/plan@[0-9a-f]{40}", p["uses"]), p["uses"]
assert "release-inputs" in p["if"] and "probe" in p["if"]
want = set(yaml.safe_load(open(sys.argv[2], encoding="utf-8"))["inputs"])
assert set(p["with"]) == want, sorted(want ^ set(p["with"]))
for i in ("release-inputs", "release-plan-probe"):
    assert f"steps.{i}.outcome" in steps[-1]["env"]["RESULTS"], i
PY

# actionlint lexes every `${{ ... }}` of a workflow, also inside a run block, and knows only '...' literals: no double quote and no literal `${{` inside check.yml's run text
python3 -I - "$repo/.github/workflows/check.yml" <<'PY' && ok "no double quote inside a \${{ }} expression of check.yml" || bad "an expression of check.yml has a double quote (actionlint: unexpected character)"
import re, sys
text = open(sys.argv[1], encoding="utf-8").read()
bad = [m.group(0) for m in re.finditer(r"\$\{\{.*?(\}\}|$)", text, re.M) if '"' in m.group(0)]
assert not bad, bad
PY

# mutants of the reader block: each must fail at least one case
mutant() { # <name> <python/sed -E expression>
  sed -E "$2" "$tmp/reader.sh" >"$tmp/m.sh"
  if cmp -s "$tmp/m.sh" "$tmp/reader.sh"; then bad "mutant '$1' changed nothing (NOT APPLIED)"; return; fi
  cases_fail=0; CASEQ=1 all_cases "$tmp/m.sh"
  if [ "$cases_fail" -gt 0 ]; then ok "mutant '$1' is red ($cases_fail cases)"; else bad "mutant '$1' SURVIVES"; fi
}
mutant "ignore-binaries not read" 's/"ignore-binaries": ""/"ignore-binariez": ""/'
mutant "binaries not read" 's/"binaries": ""/"binariez": ""/'
mutant "kind default empty" 's/"kind": "cli"/"kind": ""/'
mutant "tag always 0.0.0" 's/put\("tag", .*$/put("tag", "v0.0.0")/'
mutant "probe never true" 's/put\("probe", "true"\)/put("probe", "false")/'
mutant "no check of the called workflow" 's/startswith\(RELEASE_USES\)/startswith("")/'
mutant "the new owner is not probed (silent skip)" 's/, "FreeSynergy-Net\/fs-release\/.github\/workflows\/release.yml@"\)$/)/'
mutant "expressions are passed on" 's/"\$" \+ "\{\{" in v or //'
mutant "booleans are passed as Python words" 's/\("true" if v else "false"\) if isinstance\(v, bool\) else str\(v\)/str(v)/'
mutant "owner not looked for (first package)" 's/ or mine$/ or mine\nowners = mine/'
mutant "working directory ignored" 's/workdir = os.path.join\(root, vals\["working-directory"\]\)/workdir = root/'

echo
if [ "$fails" = 0 ]; then echo "test_plan_probe_step: all ok"; else echo "test_plan_probe_step: $fails failures"; exit 1; fi
