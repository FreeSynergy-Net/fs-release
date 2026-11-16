#!/usr/bin/env bash
# Self test of scripts/check_job_status.py: the OLD publish `if` (fixtures/job-status/old.yml, the line of fs-release v0.0.9) is red, the new
# one is green, no `if` at all below a `!cancelled()` job is red, a transitive descendant is red, a mutant (the lax pattern emptied) lets the old line through.
#   bash scripts/test_check_job_status.sh
set -uo pipefail
here="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
tool="$here/check_job_status.py"; fx="$here/../tests/fixtures/job-status"
mkdir -p "$here/../.test-tmp"; tmp="$(TMPDIR="$here/../.test-tmp" mktemp -d)"
trap 'rm -rf "$tmp"; rmdir "$here/../.test-tmp" 2>/dev/null' EXIT
fails=0
ok() { echo "ok   $1"; }
bad() { echo "FAIL $1"; fails=$((fails + 1)); }
rc_of() { python3 -I "${2:-$tool}" "$1" >"$tmp/o" 2>&1; echo $?; }
chk() { local r; r=$(rc_of "$2" "${4:-}"); if [ "$r" = "$3" ]; then ok "$1"; else bad "$1: rc=$r wanted $3: $(cat "$tmp/o")"; fi; }
chk "old publish line is red" "$fx/old.yml" 1
grep -q 'job `publish` is below `finalize`' "$tmp/o" && ok "message names job and ancestor" || bad "message: $(cat "$tmp/o")"
chk "new publish line is green" "$fx/new.yml" 0
sed '/^    if: \${{ needs.finalize/d' "$fx/old.yml" > "$tmp/noif.yml"; chk "no if at all below a !cancelled() job is red" "$tmp/noif.yml" 1
cat "$fx/new.yml" > "$tmp/deep.yml"; printf '  after:\n    needs: publish\n    runs-on: ubuntu-24.04\n    steps: [{ run: "true" }]\n' >> "$tmp/deep.yml"
chk "a transitive descendant without status function is red" "$tmp/deep.yml" 1
sed 's/!cancelled() \&\& needs.plan/always() \&\& needs.plan/' "$fx/new.yml" > "$tmp/al.yml"; chk "always() counts as a status function" "$tmp/al.yml" 0
chk "the real repo is green" "$here/.." 0
sed 's/LAX = re.compile(.*/LAX = re.compile(r"(?!)")/' "$tool" > "$tmp/mut.py"
chk "mutant (lax pattern emptied) lets the old line through, so the red test is real" "$fx/old.yml" 0 "$tmp/mut.py"
[ "$fails" -eq 0 ] || { echo "$fails failed"; exit 1; }
echo "all ok"
