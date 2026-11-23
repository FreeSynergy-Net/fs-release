#!/usr/bin/env bash
# Chain gate (reads git objects only, no network except an unshallow fetch): a caller pin `release.yml@X` must not point at a state whose
# publish-release pin is older than the attestation-identity fix (scripts/pin-chain-fix.txt, commit A 9134bcf8).
# Chain checked from <start> (default HEAD):  <start>:release.yml -> _release-publish.yml@Z ; Z:_release-publish.yml -> publish-release@Y ;
#   Z and Y must both contain the fix commit (be it or a descendant), Y:publish.sh must contain `--source-ref`.
# Root cause it closes: fs-release pins itself one round behind, so the caller pin of a round shows the OLD publish-release.
# THREAT MODEL: protects against OVERSIGHT, not against someone who can change workflows, toolchain or scripts.
# Usage: scripts/check_pin_chain.sh [--repo <dir>] [--fix <sha>] [<start-ref>]    rc 0 green, 1 red, 2 bad input.
set -uo pipefail
here="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
repo="$(cd "$here/.." && pwd)"
fix=""
start=HEAD
while [ $# -gt 0 ]; do
  case $1 in
    --repo) repo=${2:?--repo needs a dir}; shift 2 ;;
    --fix) fix=${2:?--fix needs a sha}; shift 2 ;;
    *) start=$1; shift ;;
  esac
done
cd "$repo" || exit 2
[ -n "$fix" ] || fix="$(tr -d '[:space:]' <"$here/pin-chain-fix.txt")"
[[ $fix =~ ^[0-9a-f]{40}$ ]] || { echo "pin-chain: bad fix sha '$fix'" >&2; exit 2; }
if [ "$(git rev-parse --is-shallow-repository)" = true ]; then timeout 120 git fetch --quiet --unshallow || { echo "pin-chain: cannot unshallow" >&2; exit 2; }; fi
red() { echo "pin-chain RED: $*" >&2; exit 1; }
# pin_of <commit> <file> <path-regex>: the 40-char SHA after `<path>@` in a non-comment `uses:` line
pin_of() {
  git show "$1:$2" 2>/dev/null | grep -vE '^[[:space:]]*#' | grep -oE "uses:[[:space:]]*(FreeSynergy-Net|FreeSynergy)/fs-release/$3@[0-9a-f]{40}" | head -1 | sed -E 's/.*@//'
}
has_fix() { git cat-file -e "$1^{commit}" 2>/dev/null && git merge-base --is-ancestor "$fix" "$1"; }

z="$(pin_of "$start" .github/workflows/release.yml '\.github/workflows/_release-publish\.yml')"
[ -n "$z" ] || red "$start: release.yml has no _release-publish.yml@<sha> pin"
has_fix "$z" || red "$start: release.yml pins _release-publish.yml@$z, which does not contain the fix $fix"
y="$(pin_of "$z" .github/workflows/_release-publish.yml 'actions/publish-release')"
[ -n "$y" ] || red "$z: _release-publish.yml has no publish-release@<sha> pin"
has_fix "$y" || red "$z: _release-publish.yml pins publish-release@$y, older than the fix $fix"
git show "$y:actions/publish-release/publish.sh" 2>/dev/null | grep -q -- '--source-ref' || red "$y: publish.sh lacks --source-ref"
echo "pin-chain green: $start -> _release-publish@$z -> publish-release@$y (contains $fix)"
