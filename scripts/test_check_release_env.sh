#!/usr/bin/env bash
# Self test of scripts/check_release_env.sh with a MOCK `gh` in PATH (no network, no real gh). The mock answers `gh api <path>` from fixture files and
# records every call; a call with a write flag (-X, --method, -f, -F, --field, --raw-field, --input) or any command other than `gh api` creates
# the marker WRITE, which turns the test red. Cases: all OK, environment 404, environment without protection, secrets only in the repository,
# no reviewer, no tag rule, 403 (rc 2, not "fehlt"), empty rulesets, wrong tag commit, --strict. Then mutants of the script must all turn the test red.
#   bash scripts/test_check_release_env.sh
set -uo pipefail
here="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
real="$here/check_release_env.sh"
tmp="$(mktemp -d)"
trap 'rm -rf "$tmp"' EXIT
command -v jq >/dev/null 2>&1 || { echo "skipped: no jq (nothing was checked)"; exit 2; }
mkdir -p "$tmp/bin"
cat >"$tmp/bin/gh" <<'MOCK'
#!/usr/bin/env bash
echo "$*" >>"$MOCK_DIR/calls.log"
for a in "$@"; do
  case "$a" in -X|-X?*|--method|--method=*|-f|-F|--field|--raw-field|--input) touch "$MOCK_DIR/WRITE"; echo "mock: write flag" >&2; exit 1 ;; esac
done
[ "${1:-}" = api ] || { touch "$MOCK_DIR/WRITE"; echo "mock: not gh api" >&2; exit 1; }
shift
path=""
while [ "$#" -gt 0 ]; do case "$1" in -H) shift 2 ;; *) path="$1"; shift ;; esac; done
f="$MOCK_DIR/fx/${path//\//_}"
if [ -f "$f.err" ]; then [ ! -f "$f.errout" ] || cat "$f.errout"; cat "$f.err" >&2; exit 1
elif [ -f "$f.json" ]; then cat "$f.json"
else echo "gh: Not Found (HTTP 404)" >&2; exit 1; fi
MOCK
chmod +x "$tmp/bin/gh"

# fx <name> <json>: fixture for an API path, fxe <name> <text>: error answer
fx() { printf '%s' "$2" >"$M/fx/$1.json"; }
fxe() { printf '%s' "$2" >"$M/fx/$1.err"; }
R=repos_FreeSynergy
ENVP="environments_release-signing"
# mkrepo <short> <variant>
mkrepo() {
  local r="fs-$1" v="$2" rev es rs
  rev='{"type":"required_reviewers","reviewers":[{"type":"User","reviewer":{"login":"owner"}}]}'
  local pol='{"total_count":1,"branch_policies":[{"name":"v*","type":"tag"}]}'
  local bp='{"protected_branches":false,"custom_branch_policies":true}'
  local both='{"total_count":2,"secrets":[{"name":"GPG_PRIVATE_KEY"},{"name":"GPG_PASSPHRASE"}]}'
  local none='{"total_count":0,"secrets":[]}'
  fx "${R}_${r}_contents_.github_workflows" '[{"type":"file","name":"release.yml"},{"type":"file","name":"ci.yml"}]'
  fx "${R}_${r}_contents_.github_workflows_release.yml" 'uses: FreeSynergy/fs-release/.github/workflows/release.yml@abc'
  rs="$none"; es="$both"
  case "$v" in
    ok) fx "${R}_${r}_$ENVP" "{\"name\":\"release-signing\",\"protection_rules\":[$rev,{\"type\":\"branch_policy\"}],\"deployment_branch_policy\":$bp}"; fx "${R}_${r}_$ENVP"_deployment-branch-policies "$pol" ;;
    missing) es="" ;;
    open) fx "${R}_${r}_$ENVP" '{"name":"release-signing","protection_rules":[],"deployment_branch_policy":null}'; es="$none"; rs="$both" ;;
    reponly) fx "${R}_${r}_$ENVP" "{\"name\":\"release-signing\",\"protection_rules\":[$rev],\"deployment_branch_policy\":$bp}"; fx "${R}_${r}_$ENVP"_deployment-branch-policies "$pol"; es="$none"; rs="$both" ;;
    norev) fx "${R}_${r}_$ENVP" "{\"name\":\"release-signing\",\"protection_rules\":[{\"type\":\"required_reviewers\",\"reviewers\":[]}],\"deployment_branch_policy\":$bp}"; fx "${R}_${r}_$ENVP"_deployment-branch-policies "$pol" ;;
    notag) fx "${R}_${r}_$ENVP" "{\"name\":\"release-signing\",\"protection_rules\":[$rev],\"deployment_branch_policy\":$bp}"; fx "${R}_${r}_$ENVP"_deployment-branch-policies '{"total_count":1,"branch_policies":[{"name":"main","type":"branch"}]}' ;;
    envbad) fx "${R}_${r}_$ENVP" '{not json' ;;
    envempty) fx "${R}_${r}_$ENVP" '' ;;
    brtag) fx "${R}_${r}_$ENVP" "{\"name\":\"release-signing\",\"protection_rules\":[$rev],\"deployment_branch_policy\":$bp}"; fx "${R}_${r}_$ENVP"_deployment-branch-policies '{"total_count":1,"branch_policies":[{"name":"v*","type":"branch"}]}' ;;
    nocust) fx "${R}_${r}_$ENVP" "{\"name\":\"release-signing\",\"protection_rules\":[$rev],\"deployment_branch_policy\":{\"protected_branches\":false,\"custom_branch_policies\":false}}"; fx "${R}_${r}_$ENVP"_deployment-branch-policies "$pol" ;;
    nocall404) es=""; fx "${R}_${r}_contents_.github_workflows" '[{"type":"file","name":"ci.yml"}]' ;;
    unk404) es=""; fxe "${R}_${r}_contents_.github_workflows" 'gh: Resource not accessible (HTTP 403)' ;;
    e403) fxe "${R}_${r}_$ENVP" 'gh: Resource not accessible by personal access token (HTTP 403)' ;;
  esac
  fx "${R}_${r}_actions_secrets" "$rs"
  [ "$v" = missing ] || fx "${R}_${r}_$ENVP"_secrets "$es"
}
# setup <variant of fs-a> [rulesets json] [tag sha]
setup() {
  M="$tmp/m$((++n))"; rm -rf "$M"; mkdir -p "$M/fx"; : >"$M/calls.log"
  mkrepo a "$1"; mkrepo b ok
  local rsd='[{"id":1,"target":"tag"}]'
  case "${2-$rsd}" in
    404) fxe "${R}_fs-release_rulesets" 'gh: Not Found (HTTP 404)'
         printf '%s' '{"message":"Not Found","documentation_url":"https://docs.github.com","status":"404"}' >"$M/fx/${R}_fs-release_rulesets.errout" ;;
    404arr) fxe "${R}_fs-release_rulesets" 'gh: Not Found (HTTP 404)'; printf '%s' "$rsd" >"$M/fx/${R}_fs-release_rulesets.errout" ;;
    *) fx "${R}_fs-release_rulesets" "${2-$rsd}" ;;
  esac
  fx "${R}_fs-release_git_ref_tags_v0.0.10" "{\"object\":{\"sha\":\"${3:-a084f8b96ecac9c1eccbeca9244ae7fa2bd9a38a}\",\"type\":\"commit\"}}"
}
n=0; fails=0
# shellcheck disable=SC2034  # rc, w and o are read inside the eval-ed conditions
# run <script> <args...>: output in $o, rc in $rc
run() { local s="$1"; shift; o="$(PATH="$tmp/bin:$PATH" MOCK_DIR="$M" bash "$s" "$@" 2>"$M/stderr")"; rc=$?; }
# chk <label> <condition as shell text>: green when the condition holds
chk() { if eval "$2"; then echo "ok   $1"; else echo "FAIL $1"; fails=$((fails + 1)); fi; }
has() { grep -q -- "$1" <<<"$o"; }
row() { grep -P "^fs-$1\\s" <<<"$o" | head -1; }
args=(--repos "a,b" --tag v0.0.10 --sha a084f8b9)

suite() {
  local s="$1" d w=0
  fails=0
  setup ok; run "$s" "${args[@]}"
  chk "all OK: rc 0" '[ "$rc" -eq 0 ]'
  chk "all OK: both repos OK" 'row a | grep -q " OK" && row b | grep -q " OK"'
  chk "all OK: TSV block with a tab separated row" "grep -qP '^fs-a\\tja\\t1\\tja\\t2/2\\t0/2\\tja\\tOK\\t' <<<\"\$o\""
  chk "all OK: tag points at the expected commit" "has 'expected commit'"
  chk "all OK: no write call (no WRITE marker)" '[ ! -e "$M/WRITE" ]'
  run "$s" "${args[@]}" --strict; chk "all OK: --strict rc 0" '[ "$rc" -eq 0 ]'
  setup missing; run "$s" "${args[@]}"
  chk "environment 404: FEHLT, rc 0" 'row a | grep -q FEHLT && [ "$rc" -eq 0 ]'
  run "$s" "${args[@]}" --strict; chk "environment 404: --strict rc 1" '[ "$rc" -eq 1 ]'
  setup open; run "$s" "${args[@]}"
  chk "environment without protection: WARN" 'row a | grep -q WARN'
  chk "environment without protection: names the missing parts" 'row a | grep -q "no required reviewer" && row a | grep -q "no tag rule" && row a | grep -q "secrets only in the repository"'
  setup reponly; run "$s" "${args[@]}"
  chk "secrets only in the repository: WARN" 'row a | grep -q WARN'
  setup norev; run "$s" "${args[@]}"
  chk "no reviewer in required_reviewers: WARN, not OK" 'row a | grep -q WARN'
  setup notag; run "$s" "${args[@]}"
  chk "no tag policy v*: WARN, not OK" 'row a | grep -q WARN'
  setup e403; run "$s" "${args[@]}"
  chk "403: rc 2" '[ "$rc" -eq 2 ]'
  chk "403: not reported as fehlt" '! has FEHLT'
  chk "403: error on stderr, nothing on stdout" '[ -z "$o" ] && grep -q 403 "$M/stderr"'
  setup ok '[]'; run "$s" "${args[@]}"
  chk "empty rulesets: Tag-Schutz fehlt" "has 'Tag-Schutz fehlt'"
  setup ok '[{"id":1,"target":"tag"}]' 'ffffffffffffffffffffffffffffffffffffffff'; run "$s" "${args[@]}"
  chk "wrong tag commit: MISMATCH" 'has MISMATCH'
  for v in envbad envempty; do
    setup $v; run "$s" "${args[@]}"
    chk "environment JSON $v: WARN, 0 reviewers (jq error never counts)" "row a | grep -q WARN && grep -qP '^fs-a\\tja\\t0\\t' <<<\"\$o\""
    chk "environment JSON $v: rc 0, no jq noise" '[ "$rc" -eq 0 ] && ! grep -qi "jq:" "$M/stderr"'
  done
  setup brtag; run "$s" "${args[@]}"
  chk "policy v* of type branch: no tag rule, WARN" 'row a | grep -q WARN && row a | grep -q "no tag rule"'
  setup nocust; run "$s" "${args[@]}"
  chk "custom_branch_policies false with policy list: no tag rule, WARN" 'row a | grep -q WARN && row a | grep -q "no tag rule"'
  setup nocall404; run "$s" "${args[@]}" --strict
  chk "404 and no release.yml call: nicht nötig, not FEHLT" 'row a | grep -q "nicht nötig" && ! row a | grep -q FEHLT'
  chk "nicht nötig does not count: --strict rc 0, not OK: 0" '[ "$rc" -eq 0 ] && has "not OK: 0"'
  setup missing; run "$s" "${args[@]}"
  chk "404 and release.yml call: FEHLT, not nicht nötig" 'row a | grep -q FEHLT && ! row a | grep -q "nicht nötig"'
  setup unk404; run "$s" "${args[@]}" --strict
  chk "404 and release unbekannt: FEHLT (safe), --strict rc 1" 'row a | grep -q FEHLT && row a | grep -q unbekannt && [ "$rc" -eq 1 ]'
  setup ok 404; run "$s" "${args[@]}"
  chk "rulesets 404 with error JSON: Tag-Schutz unbekannt, no count" "has 'Tag-Schutz unbekannt' && ! has 'rulesets: [0-9]' && [ \"\$rc\" -eq 0 ]"
  setup ok 404arr; run "$s" "${args[@]}"
  chk "rulesets 404 even with an array body: Tag-Schutz unbekannt" "has 'Tag-Schutz unbekannt' && ! has 'rulesets: [0-9]'"
  setup ok '{"message":"x","a":1,"b":2}'; run "$s" "${args[@]}"
  chk "rulesets ok but not an array: Tag-Schutz unbekannt" "has 'Tag-Schutz unbekannt' && ! has 'rulesets: [0-9]'"
  setup ok ''; run "$s" "${args[@]}"
  chk "rulesets empty answer: Tag-Schutz unbekannt, no empty count" "has 'Tag-Schutz unbekannt' && ! has 'target tag: )'"
  setup ok '[{"id":1,"target":"tag"}]' 'ffffffffffffffffffffffffffffffffffffffff'; run "$s" "${args[@]}"
  chk "MISMATCH without --strict: rc 0" '[ "$rc" -eq 0 ]'
  run "$s" "${args[@]}" --strict; chk "MISMATCH with --strict: rc 1" '[ "$rc" -eq 1 ]'
  setup ok; run "$s" --repos a --out "$M/out.txt" --tag v0.0.10
  chk "--out writes the report to the file" 'grep -q "# TSV" "$M/out.txt"'
  setup ok; run "$s" --repos a --bogus; chk "unknown argument: rc 2" '[ "$rc" -eq 2 ]'
  for d in "$tmp"/m*; do [ ! -e "$d/WRITE" ] || w=1; done
  if [ "$w" -eq 0 ]; then echo "ok   write guard: no write call in any run"; else echo "FAIL write guard: a write call was seen"; fails=$((fails + 1)); fi
}

echo "== real script"; out="$(suite "$real")"; echo "$out"
real_fails="$(grep -c '^FAIL' <<<"$out")"
total=$real_fails

# mutants: each must turn the suite red
mut() { # <label> <sed expression>
  cp "$real" "$tmp/mutant.sh"; sed -i "$2" "$tmp/mutant.sh"
  if cmp -s "$tmp/mutant.sh" "$real"; then echo "FAIL mutant '$1' changed nothing"; total=$((total + 1)); return; fi
  local mo; mo="$(suite "$tmp/mutant.sh")"
  if grep -q '^FAIL' <<<"$mo"; then echo "ok   mutant '$1' turns the test red ($(grep -c '^FAIL' <<<"$mo") checks)"; else echo "FAIL mutant '$1' stayed green"; total=$((total + 1)); fi
}
echo "== mutants"
mut "404 and 403 treated alike" "s/grep -q 'HTTP 404'/grep -q 'HTTP 4'/"
mut "required_reviewers not checked" 's/if \[ "\$rev" -ge 1 \] && /if /'
mut "tag rule not checked" 's/ \&\& \[ "\$trule" = ja \] \&\& \[ "\$ecount"/ \&\& [ "$ecount"/'
mut "environment secrets not checked" 's/ \&\& \[ "\$ecount" -eq "\${#required\[@\]}" \]; then res="OK"/; then res="OK"/'
mut "write call allowed" 's|^  ghapi_guard "\$@"$|  "$gh_bin" api -X POST "repos/x/y" >/dev/null 2>\&1 \|\| true|'
mut "rulesets empty not reported" 's/Tag-Schutz fehlt/Tag-Schutz ok/'
mut "MA: jq error counts as reviewer" 's/<<<"\$envjson" 2>\/dev\/null || echo 0)"/<<<"$envjson" 2>\/dev\/null || echo 1)"/'
mut "empty reviewer answer kept" '/never counts as reviewer/d'
mut "MB: branch policy v* counts as tag rule" 's/ and \.type=="tag"//'
mut "ME: policy list read without custom_branch_policies" 's/\(custom_branch_policies \/\/ false.*\)= true \]/\1!= x ]/'
mut "nicht nötig for every 404" 's/if \[ "\$call" = nein \]; then res=/if true; then res=/'
mut "nicht nötig also for unbekannt" 's/if \[ "\$call" = nein \]; then res=/if [ "$call" != ja ]; then res=/'
mut "nicht nötig counted as not OK" 's/ || \[ "\$res" = "nicht nötig" \]//'
mut "rulesets read at any status" 's/if \[ "\$API_ST" = ok \] \&\& jq -e/if true \&\& jq -e/'
mut "rulesets: array type not checked" 's/ \&\& jq -e .type=="array". <<<"\$API_OUT" >\/dev\/null 2>\&1//'
mut "tag MISMATCH not counted for --strict" 's/; mismatch=1; fi/; fi/'
if [ "$total" -eq 0 ]; then echo "all ok"; else echo "$total failed"; exit 1; fi
