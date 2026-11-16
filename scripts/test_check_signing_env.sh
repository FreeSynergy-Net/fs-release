#!/usr/bin/env bash
# Self test of scripts/check-signing-env with counter-checks on copies of the real workflows: the real files are green; the signing job without
# `environment:` is red; the signing secrets in another job (build job, mirror job) are red; a mutation of the check (rules emptied) turns the counter-cases red.
#   bash scripts/test_check_signing_env.sh
set -uo pipefail
here="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
repo="$(cd "$here/.." && pwd)"
tool="$here/check-signing-env"
tmp="$(mktemp -d)"
trap 'rm -rf "$tmp"' EXIT
fails=0
ok() { echo "ok   $1"; }
bad() { echo "FAIL $1"; fails=$((fails + 1)); }
python3 -c "import yaml" 2>/dev/null || { echo "skipped: no PyYAML (nothing was checked)"; exit 2; }

python3 -I "$tool" >/dev/null 2>&1 && ok "the real workflows are green" || bad "the real workflows are red"
grep -q '^    environment: release-signing' "$repo/.github/workflows/_release-finalize.yml" && ok "the signing job names the environment release-signing" || bad "no environment in _release-finalize.yml"

# case <tool> <label>: the three counter-cases must all be red with the given tool
cases() {
  local t="$1" l="$2" d r
  d="$tmp/$l-a"; mkdir -p "$d"; sed '/^    environment: release-signing$/d' "$repo/.github/workflows/_release-finalize.yml" >"$d/_release-finalize.yml"
  python3 -I "$t" "$d" >"$tmp/o" 2>&1; r=$?
  if [ "$r" -eq 1 ] && grep -q 'no `environment:`' "$tmp/o"; then echo "ok   $l: signing job without environment is red"; else echo "FAIL $l: signing job without environment rc=$r"; fi
  d="$tmp/$l-b"; mkdir -p "$d"
  cat >"$d/mirror.yml" <<'Y'
name: mirror
on: { push: { tags: ["v*"] } }
jobs:
  build:
    runs-on: ubuntu-24.04
    steps:
      - run: echo "$KEY" | gpg --import
        env:
          KEY: ${{ secrets.GPG_PRIVATE_KEY }}
Y
  python3 -I "$t" "$d" >"$tmp/o" 2>&1; r=$?
  if [ "$r" -eq 1 ] && grep -q 'not the signing job' "$tmp/o"; then echo "ok   $l: signing secret in another job is red"; else echo "FAIL $l: secret outside the signing job rc=$r"; fi
  d="$tmp/$l-c"; mkdir -p "$d"; cp "$repo/.github/workflows/release.yml" "$d/release.yml"
  python3 - "$d/release.yml" <<'PY'
import sys
p = sys.argv[1]
s = open(p).read()
s = s.replace("  plan:\n    runs-on: ubuntu-24.04\n", "  plan:\n    runs-on: ubuntu-24.04\n    env:\n      K: ${{ secrets.GPG_PRIVATE_KEY }}\n", 1)
open(p, "w").write(s)
PY
  python3 -I "$t" "$d" >"$tmp/o" 2>&1; r=$?
  if [ "$r" -eq 1 ] && grep -q 'plan: touches' "$tmp/o"; then echo "ok   $l: secret in the plan job is red"; else echo "FAIL $l: secret in the plan job rc=$r"; fi
}
out="$(cases "$tool" real)"
echo "$out" | grep '^ok'
echo "$out" | grep -q '^FAIL' && { echo "$out" | grep '^FAIL'; fails=$((fails + 1)); }

cp "$tool" "$tmp/mutant"
sed -i 's/if signer and not job.get("environment"):/if False:/; s/if "GPG_" in text and not (signer or calls_final):/if False:/' "$tmp/mutant"
cmp -s "$tmp/mutant" "$tool" && bad "mutation changed nothing"
mout="$(cases "$tmp/mutant" mut)"
if grep -q '^FAIL' <<<"$mout"; then ok "mutation (rules removed) turns the counter-cases red"; else bad "mutation stayed green"; fi

if [ "$fails" -eq 0 ]; then echo "all ok"; else echo "$fails failed"; exit 1; fi
