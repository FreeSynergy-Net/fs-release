#!/usr/bin/env bash
# Self test of scripts/check_pin_chain.sh on a synthetic repo: P0 (old publish-release), A (the fix), B (pins A), C (caller pins B).
# Green: C. Red mutants: B (caller one round behind, its _release-publish still pins P0), publish-release pin back on P0, fix marker removed.
set -uo pipefail
here="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
chk="$here/check_pin_chain.sh"
tmp="$(mktemp -d)"
trap 'rm -rf "$tmp"' EXIT
fails=0
ok() { echo "ok   $1"; }
bad() { echo "FAIL $1"; fails=$((fails + 1)); }
r="$tmp/r"; mkdir -p "$r/.github/workflows" "$r/actions/publish-release"; cd "$r" || exit 1
g() { git -c user.name=t -c user.email=t@t "$@"; }
g init -q .
w() { # w <release.yml pin> <publish pin> : write both workflows
  echo "    uses: FreeSynergy-Net/fs-release/.github/workflows/_release-publish.yml@$1" >.github/workflows/release.yml
  echo "      - uses: FreeSynergy-Net/fs-release/actions/publish-release@$2" >.github/workflows/_release-publish.yml
}
z=0000000000000000000000000000000000000000
w $z $z; echo 'old' >actions/publish-release/publish.sh; g add -A; g commit -qm P0; p0=$(g rev-parse HEAD)
echo 'x --source-ref' >actions/publish-release/publish.sh; g commit -qam A; a=$(g rev-parse HEAD)
w $p0 $a; g commit -qam B; b=$(g rev-parse HEAD)
w $b $a; g commit -qam C; c=$(g rev-parse HEAD)
if bash "$chk" --repo "$r" --fix "$a" "$c" >/dev/null 2>&1; then ok "end of the chain C is green"; else bad "C is red"; fi
if bash "$chk" --repo "$r" --fix "$a" "$b" >/dev/null 2>&1; then bad "mutant B (caller one round behind) stays green"; else ok "mutant B (pin on an old state) is red"; fi
w $b $p0; g commit -qam M1; m1=$(g rev-parse HEAD)
w $m1 $p0; g commit -qam M1b; m1b=$(g rev-parse HEAD)
if bash "$chk" --repo "$r" --fix "$a" "$m1b" >/dev/null 2>&1; then bad "mutant publish-release pin on P0 stays green"; else ok "mutant publish-release pin on P0 is red"; fi
if bash "$chk" --repo "$r" --fix "$a" "$p0" >/dev/null 2>&1; then bad "mutant start at P0 stays green"; else ok "mutant start at the pre-fix state is red"; fi
if bash "$chk" --repo "$r" --fix 1111111111111111111111111111111111111111 "$c" >/dev/null 2>&1; then bad "unknown fix commit stays green"; else ok "fix commit not in history is red"; fi
if [ "$fails" -eq 0 ]; then echo "all ok"; else echo "$fails failed"; exit 1; fi
