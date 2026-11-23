#!/usr/bin/env bash
# Pattern: none (test). Self test of N1d (review of bundle 7): the job `identity` of .github/workflows/check.yml and the toolchain pin of actions/setup-rust. Checks, on the real files (awk, no Python):
#  - the job `identity` exists, has ONLY the checkout and the identity step (no setup-rust, rust-cache, apt, no caller input except runs-on, no working-directory): no tool of the caller runs in it;
#  - the job `check` has `needs: identity` (the caller's code never starts before the identity check is green);
#  - the checkout of the identity job takes the PR head commit on pull_request (GitHub's merge commit has the committer GitHub <noreply@github.com>: it would be red by mistake, S15);
#  - the run block of the step is a byte copy of the body of scripts/identity-check, and RUN as the runner does it in a fixture repo (CI case): allowed identity green, foreign author/committer red;
#  - actions/setup-rust writes RUSTUP_TOOLCHAIN to GITHUB_ENV BEFORE rust-cache (a rust-toolchain.toml with `path = ...` of the caller can then never choose the compiler that rust-cache starts);
#    a stub `rustc` shows the effect: with RUSTUP_TOOLCHAIN set the toolchain file is not asked.
# Mutants (no needs, a setup-rust step in the identity job, a weakened block, the pin after rust-cache) must each be red. TEST_WORKFLOW=<check.yml> TEST_ACTION=<setup-rust action.yml> run the checks
# against other files (used to show them red against the old state).
#   bash scripts/test_check_job_identity.sh
set -uo pipefail
here="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
repo="$(cd "$here/.." && pwd)"
tmp="$(mktemp -d "${TEST_TMPDIR:-${TMPDIR:-/tmp}}/job-identity.XXXXXX")"
trap 'rm -rf "$tmp"' EXIT
fails=0
ok() { echo "ok   $1"; }
bad() { echo "FAIL $1"; fails=$((fails + 1)); }
wf="${TEST_WORKFLOW:-$repo/.github/workflows/check.yml}"
act="${TEST_ACTION:-$repo/actions/setup-rust/action.yml}"
jobtext() { awk -v j="  $2:" '$0 == j { s = 1; print; next } s && /^  [a-z]/ { exit } s { print }' "$1"; }  # <file> <job>
# structure <check.yml> <action.yml>: one line per problem
structure() {
  local f="$1" a="$2" job
  job="$(jobtext "$f" identity)"
  if [ -z "$job" ]; then echo "no job identity"; else
    [ "$(grep -c '^      - ' <<<"$job")" = 2 ] || echo "the job identity has not exactly 2 steps (checkout + identity)"
    [ "$(grep -c 'uses:' <<<"$job")" = 1 ] && grep -q 'uses: actions/checkout@' <<<"$job" || echo "the job identity uses something other than actions/checkout"
    grep -Eq 'setup-rust|rust-cache|rustup|apt-get|cargo ' <<<"$job" && echo "the job identity runs a tool of the toolchain or of apt"
    [ "$(grep -c 'inputs\.' <<<"$job")" = 1 ] && grep -q 'runs-on: \${{ inputs.runs-on }}' <<<"$job" || echo "the job identity reads an input other than runs-on"
    grep -Eq 'working-directory|defaults:' <<<"$job" && echo "the job identity takes a working directory from the caller"
    grep -q 'ref: ${{ github.event.pull_request.head.sha || github.sha }}' <<<"$job" || echo "the checkout of the identity job does not take the PR head commit (merge commit = committer GitHub)"
    awk '/^      - name: Identity/ { s = 1; next } s && /^        run: \|$/ { r = 1; next } s && r { if ($0 ~ /^          /) { print substr($0, 11); next } else exit }' <<<"$job" >"$tmp/jb.$$"
    [ -s "$tmp/jb.$$" ] || echo "no run block for the step Identity"
    sed -n '/^# --- body/,$p' "$repo/scripts/identity-check" | tail -n +2 | cmp -s - "$tmp/jb.$$" || echo "the identity block differs from the body of scripts/identity-check"
    grep -q '^        shell: bash {0}$' <<<"$job" || echo "the identity step does not run as shell: bash {0}"
  fi
  grep -Eq '^    needs: \[?identity\]?$' <<<"$(jobtext "$f" check)" || echo "the job check has no needs: identity"
  local p r
  p=$(grep -n 'RUSTUP_TOOLCHAIN=' "$a" | head -1 | cut -d: -f1); r=$(grep -n 'uses: Swatinem/rust-cache' "$a" | head -1 | cut -d: -f1)
  { [ -n "$p" ] && [ -n "$r" ] && [ "$p" -lt "$r" ] && grep -q 'RUSTUP_TOOLCHAIN=.*GITHUB_ENV' "$a"; } || echo "setup-rust does not write RUSTUP_TOOLCHAIN to GITHUB_ENV before rust-cache"
}
out="$(structure "$wf" "$act")"; if [ -z "$out" ]; then ok "structure: identity job isolated, check needs it, PR head commit, block = script, RUSTUP_TOOLCHAIN before rust-cache"; else bad "structure: $out"; fi
# RUN the block of the real workflow in a fixture repo (CI case)
jobtext "$wf" identity | awk '/^      - name: Identity/ { s = 1; next } s && /^        run: \|$/ { r = 1; next } s && r { if ($0 ~ /^          /) { print substr($0, 11); next } else exit }' >"$tmp/block.sh"
OK=support@freesynergy.net; BAD=agents@Beelink.hnc
mkdir "$tmp/r" && git -C "$tmp/r" init -q -b main
c() { echo "$3" >"$tmp/r/$3"; git -C "$tmp/r" add . && GIT_AUTHOR_NAME=a GIT_AUTHOR_EMAIL="$1" GIT_COMMITTER_NAME=c GIT_COMMITTER_EMAIL="$2" git -C "$tmp/r" -c commit.gpgsign=false commit -q -m "$3"; }
c "$BAD" "$BAD" old; git -C "$tmp/r" update-ref refs/remotes/origin/main HEAD
runblk() { (cd "$tmp/r" && env CI=true GITHUB_ACTIONS=true bash "$1" >"$tmp/o" 2>&1; echo $? >"$tmp/rc") ; rc=$(cat "$tmp/rc"); }
[ -s "$tmp/block.sh" ] || { bad "no block to run"; }
runblk "$tmp/block.sh"; [ "$rc" = 0 ] && ok "block run: empty range green" || bad "block run: empty range (rc $rc)"
c $OK $OK one; runblk "$tmp/block.sh"; [ "$rc" = 0 ] && ok "block run: allowed identity green" || bad "block run: allowed identity (rc $rc)"
c $BAD $OK two; runblk "$tmp/block.sh"; [ "$rc" = 1 ] && ok "block run: foreign author red" || bad "block run: foreign author (rc $rc)"
git -C "$tmp/r" reset -q --hard HEAD~1; c $OK "noreply@github.com" three; runblk "$tmp/block.sh"; [ "$rc" = 1 ] && ok "block run: committer GitHub noreply (merge commit of a PR) red - hence the PR head checkout" || bad "block run: committer GitHub (rc $rc)"
# the real merge commit of GitHub: Author = PR author, committer GitHub <noreply@github.com> (looked at in refs/pull/99/merge of rust-lang/log, 23.11.2026)
# the effect of RUSTUP_TOOLCHAIN: a stub rustup shim asks the toolchain file only when the variable is empty
mkdir -p "$tmp/bin"; cat >"$tmp/bin/rustc" <<'STUB'
#!/usr/bin/env bash
# stand-in for the rustup proxy: the environment beats rust-toolchain.toml, else the file (here: the caller's path toolchain) is used
if [ -n "${RUSTUP_TOOLCHAIN:-}" ]; then echo "rustc via $RUSTUP_TOOLCHAIN"; else echo "rustc via FILE $(sed -n 's/^path *= *"\(.*\)"/\1/p' rust-toolchain.toml)"; fi
STUB
chmod +x "$tmp/bin/rustc"; printf '[toolchain]\npath = "/evil/tc"\n' >"$tmp/r/rust-toolchain.toml"
r1=$(cd "$tmp/r" && PATH="$tmp/bin:$PATH" rustc); r2=$(cd "$tmp/r" && RUSTUP_TOOLCHAIN=1.99.1 PATH="$tmp/bin:$PATH" rustc)
{ [ "$r1" = "rustc via FILE /evil/tc" ] && [ "$r2" = "rustc via 1.99.1" ]; } && ok "model: the toolchain file chooses the compiler only while RUSTUP_TOOLCHAIN is empty" || bad "model of the toolchain file: $r1 / $r2"
if [ -z "${TEST_ACTION:-}" ]; then
  mut() { # <name> <file> <sed expr>   (file: check or act)
    cp "$wf" "$tmp/mw.yml"; cp "$act" "$tmp/ma.yml"
    if [ "$2" = check ]; then sed -i "$3" "$tmp/mw.yml"; else sed -i "$3" "$tmp/ma.yml"; fi
    out="$(structure "$tmp/mw.yml" "$tmp/ma.yml")"; if [ -n "$out" ]; then ok "mutant caught: $1"; else bad "mutant NOT caught: $1"; fi
  }
  mut "identity missing in needs of check" check '/^    needs: identity$/d'
  mut "setup-rust step in the identity job" check "s|^      - name: Identity.*|      - uses: FreeSynergy-Net/fs-release/actions/setup-rust@x\n&|"
  mut "a caller input (apt-packages) in the identity job" check '0,/^    runs-on: \${{ inputs.runs-on }}$/{n;s/^    steps:$/    env: { X: "${{ inputs.apt-packages }}" }\n    steps:/}'
  mut "PR head commit not taken" check 's/ref: \${{ github.event.pull_request.head.sha || github.sha }}/ref: ${{ github.sha }}/'
  mut "identity block weakened" check 's/\$2 != ok || \$3 != ok/$2 != ok/'
  mut "RUSTUP_TOOLCHAIN removed from setup-rust" act 's/RUSTUP_TOOLCHAIN=/XX_TOOLCHAIN=/'
  # the pin after rust-cache: move the marker line below the rust-cache line
  cp "$act" "$tmp/ma2.yml"; { grep -v 'RUSTUP_TOOLCHAIN=' "$act"; echo "# RUSTUP_TOOLCHAIN=x >> GITHUB_ENV"; } >"$tmp/ma3.yml"
  out="$(structure "$wf" "$tmp/ma3.yml")"; if [ -n "$out" ]; then ok "mutant caught: the pin after rust-cache"; else bad "mutant NOT caught: pin after rust-cache"; fi
fi
if [ "$fails" -eq 0 ]; then echo "all ok"; else echo "$fails FAIL"; exit 1; fi
