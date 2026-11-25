#!/usr/bin/env bash
# Self test of scripts/set-signing-secrets.sh with a MOCK `gh` (and mock gpg/curl) in PATH that logs the arguments and the standard input.
# It checks the order PUT environment -> POST tag rule -> environment secrets -> ONLY THEN delete of the repository secrets, REVIEWER required (rc 2),
# no delete after a failed 2nd environment secret, no secret value in arguments or output, idempotence, --dry-run (runs nothing), the ruleset body,
# the --repo-secrets warning; then MUTANTS of the script, each of which must turn the suite red.
#   bash scripts/test_set_signing_secrets.sh
# shellcheck disable=SC2034  # a..f are used inside the quoted ck conditions
set -uo pipefail
here="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
tmp="$(mktemp -d "${TMPDIR:-/tmp}/sss-test.XXXXXX")"
trap 'rm -rf "$tmp"' EXIT
SECRETVAL="SECRET-TESTVALUE-7f3a91c2"
mkdir -p "$tmp/bin" "$tmp/state"

cat >"$tmp/bin/gh" <<'MOCK'
#!/usr/bin/env bash
printf '%s\n' "$*" >>"$MOCK_ARGS"
in=""
[ -t 0 ] || in=$(cat)
[ -z "$in" ] || printf 'STDIN[%s]: %s\n' "$*" "$in" >>"$MOCK_STDIN"
if [ -n "${MOCK_FAIL_MATCH:-}" ] && [[ "$*" == *"${MOCK_FAIL_MATCH//<space>/ }"* ]]; then echo "gh: failure (HTTP ${MOCK_FAIL_CODE:-502})" >&2; exit 1; fi
repo=""
for ((i = 1; i <= $#; i++)); do [ "${!i}" = -R ] && j=$((i + 1)) && repo=${!j//\//_}; done
case "$*" in
  "api user --jq .login") echo FreeSynergy ;;
  "api user --jq .id") echo 1 ;;
  "api orgs/"*"/memberships/"*" --jq "*) echo "${MOCK_ROLE:-admin} ${MOCK_MSTATE:-active}"; [ -z "${MOCK_ROLE_FAIL:-}" ] || exit 1 ;;
  "api users/alice --jq "*) echo "4242 User" ;;
  "api users/FreeSynergy --jq "* | "api users/freesynergy --jq "*) echo "1 User" ;;
  "api users/someorg --jq "*) echo "77 Organization" ;;
  "api users/weird --jq "*) echo "abc User" ;;
  "api -X PUT repos/"*"/environments/release-signing --input -") touch "$MOCK_STATE/env" ;;
  "api repos/"*"/environments/release-signing/deployment-branch-policies")
    if [ -n "${MOCK_POLICY_EXISTS:-}" ]; then echo '{"total_count":1,"branch_policies":[{"id":9,"name":"v*","type":"tag"}]}'
    elif [ -e "$MOCK_STATE/policy" ]; then echo '{"total_count":1,"branch_policies":[{"id":9,"name":"v*","type":"tag"}]}'
    else echo '{"total_count":0,"branch_policies":[]}'; fi ;;
  "api -X POST repos/"*"/deployment-branch-policies --input -") touch "$MOCK_STATE/policy" ;;
  "api repos/"*"/environments/release-signing")
    if [ ! -e "$MOCK_STATE/env" ]; then
      if [ -n "${MOCK_ENV_EXISTS:-}" ]; then echo '{"protection_rules":[],"deployment_branch_policy":null}'; exit 0; fi
      echo "gh: Not Found (HTTP 404)" >&2; exit 1
    fi
    echo "{\"protection_rules\":[{\"type\":\"required_reviewers\",\"prevent_self_review\":${MOCK_PREVENT:-true},\"reviewers\":[{\"type\":\"User\",\"reviewer\":{\"id\":${MOCK_RID:-4242}}}]}],\"deployment_branch_policy\":{\"protected_branches\":false,\"custom_branch_policies\":true}}" ;;
  "secret set GPG_"*" --env release-signing -R "*) touch "$MOCK_STATE/env.${3}" ;;
  "secret set GPG_"*" -R "*) touch "$MOCK_STATE/repo.${3}" ;;
  "secret delete "*) rm -f "$MOCK_STATE/repo.$3" ;;
  "secret list --env release-signing -R "*) for n in GPG_PRIVATE_KEY GPG_PASSPHRASE; do [ ! -e "$MOCK_STATE/env.$n" ] || echo "$n"; done ;;
  "secret list -R "*) for n in GPG_PRIVATE_KEY GPG_PASSPHRASE; do [ ! -e "$MOCK_STATE/repo.$n" ] || echo "$n"; done ;;
  "api repos/"*"/rulesets?includes_parents=false") if [ -n "${MOCK_RULESET_ID:-}" ]; then echo "[{\"id\":${MOCK_RULESET_ID},\"name\":\"tags v* immutable\"}]"; else echo '[]'; fi ;;
  "api -X POST repos/"*"/rulesets --input -" | "api -X PUT repos/"*"/rulesets/"*" --input -") : ;;
  *) echo "mock gh: unexpected call: $*" >&2; exit 9 ;;
esac
MOCK
cat >"$tmp/bin/gpg" <<'MOCK'
#!/usr/bin/env bash
echo "gpg $*" >>"$MOCK_GPG"
case "$*" in
  *--with-colons*) printf 'ssb:u:255:22:AAAA:1700000000::::::s:::::\nfpr:::::::::1111222233334444555566667777888899990000:\n' ;;
  *--export-secret-subkeys*) printf -- '-----BEGIN PGP PRIVATE KEY BLOCK-----\n%s\n-----END PGP PRIVATE KEY BLOCK-----\n' "$SECRETVAL" ;;
  *--export*) echo public ;;
esac
MOCK
printf '#!/usr/bin/env bash\ncat >/dev/null\n' >"$tmp/bin/curl"
chmod +x "$tmp/bin/gh" "$tmp/bin/gpg" "$tmp/bin/curl"
printf '%s\n' "$SECRETVAL-passphrase" >"$tmp/pass"
chmod 600 "$tmp/pass"

fails=0
ok() { echo "ok   $1"; }
bad() { echo "FAIL $1"; fails=$((fails + 1)); }
ck() { if eval "$2"; then ok "$1"; else bad "$1"; fi; }
line() { grep -nF -- "$1" "$tmp/args" | head -n1 | cut -d: -f1; }

# run <label> <env assignments as one string, may be empty> <stdin text> <script args...>: rc in $rc, output in $tmp/out
run() {
  local s=$SCRIPT_DIR/set-signing-secrets.sh in=$3
  shift 3
  : >"$tmp/args"; : >"$tmp/stdin"; : >"$tmp/gpg"; rm -f "$tmp/state/"{env,policy,env.GPG_PRIVATE_KEY,env.GPG_PASSPHRASE}
  touch "$tmp/state/repo.GPG_PRIVATE_KEY" "$tmp/state/repo.GPG_PASSPHRASE"
  # shellcheck disable=SC2086
  env PATH="$tmp/bin:$PATH" MOCK_ARGS="$tmp/args" MOCK_STDIN="$tmp/stdin" MOCK_STATE="$tmp/state" MOCK_GPG="$tmp/gpg" SECRETVAL="$SECRETVAL" \
    PASSFILE="$tmp/pass" OUT_DIR="$tmp/keys" OUT_OWNER="$(id -un):$(id -gn)" NO_UPLOAD=1 FS_SIGN_TMP="$tmp" $EXTRA \
    bash "$s" "$@" <<<"$in" >"$tmp/out" 2>&1
  rc=$?
}

suite() {
  SCRIPT_DIR=$1
  EXTRA=""
  run x "" "" --env --repos texts --yes
  ck "REVIEWER missing: rc 2, clear message, no gh call" '[ "$rc" = 2 ] && grep -q "REVIEWER" "$tmp/out" && [ ! -s "$tmp/args" ]'
  EXTRA="REVIEWER=alice"
  run x "" "" --env --repos texts --yes
  ck "normal run rc 0" '[ "$rc" = 0 ]'
    a=$(line "-X PUT repos/FreeSynergy-Net/fs-texts/environments/release-signing"); b=$(line "-X POST repos/FreeSynergy-Net/fs-texts/environments/release-signing/deployment-branch-policies")
  c=$(line "secret set GPG_PRIVATE_KEY --env release-signing -R FreeSynergy-Net/fs-texts"); d=$(line "secret set GPG_PASSPHRASE --env release-signing -R FreeSynergy-Net/fs-texts")
  e=$(line "secret delete GPG_PRIVATE_KEY -R FreeSynergy-Net/fs-texts"); f=$(line "secret delete GPG_PASSPHRASE -R FreeSynergy-Net/fs-texts")
  ck "order: PUT env < POST tag rule < set private < set passphrase < delete private < delete passphrase" '[ -n "$a" ] && [ -n "$b" ] && [ -n "$c" ] && [ -n "$d" ] && [ -n "$e" ] && [ -n "$f" ] && [ "$a" -lt "$b" ] && [ "$b" -lt "$c" ] && [ "$c" -lt "$d" ] && [ "$d" -lt "$e" ] && [ "$e" -lt "$f" ]'
  ck "environment body: reviewer id, prevent_self_review true, tag-only deployment" 'grep -q "\"prevent_self_review\":true" "$tmp/stdin" && grep -q "\"reviewers\":\[{\"type\":\"User\",\"id\":4242}\]" "$tmp/stdin" && grep -q "\"custom_branch_policies\":true" "$tmp/stdin"'
  ck "tag rule body" 'grep -q "{\"name\":\"v\*\",\"type\":\"tag\"}" "$tmp/stdin"'
  ck "secret value reached gh on stdin" 'grep -q "$SECRETVAL" "$tmp/stdin"'
  ck "secret value in NO argument and NOT in the output" '! grep -q "$SECRETVAL" "$tmp/args" && ! grep -q "$SECRETVAL" "$tmp/out"'
  ck "check at the end green" 'grep -q -- "--- check" "$tmp/out" && ! grep -q DEVIATION "$tmp/out"'
  run x "" "" --env --yes
  ck "default list = the 9 release.yml callers + messages, broadcast, draw, planning, person, starter (15), all hardened" '[ "$rc" = 0 ] && [ "$(grep -c -- "-X PUT repos/FreeSynergy-Net/fs-[a-z-]*/environments" "$tmp/args")" = 15 ] && for n in texts people calendars tasks groups maps capnpc packages files messages broadcast draw modules-planning modules-person starter; do grep -q "PUT repos/FreeSynergy-Net/fs-$n/environments" "$tmp/args" || exit 1; done'
  EXTRA="REVIEWER=alice MOCK_ENV_EXISTS=1"
  run x "" "" --env --repos packages --yes
  ck "fixture existing environment without reviewer/rule/env secrets: updated and hardened, rc 0" '[ "$rc" = 0 ] && grep -q "PUT repos/FreeSynergy-Net/fs-packages/environments/release-signing" "$tmp/args" && grep -q "POST repos/FreeSynergy-Net/fs-packages/environments/release-signing/deployment-branch-policies" "$tmp/args" && grep -q "secret set GPG_PASSPHRASE --env" "$tmp/args"'
  EXTRA="REVIEWER=alice"
  run x "" "" --repos texts --yes
  ck "fixture environment absent (404 on read): created by PUT, rc 0" '[ "$rc" = 0 ] && grep -q "PUT repos/FreeSynergy-Net/fs-texts/environments/release-signing" "$tmp/args"'
  ck "default mode is --env" 'grep -q "environments/release-signing" "$tmp/args"'
  EXTRA="REVIEWER=alice MOCK_FAIL_MATCH=GPG_PASSPHRASE"
  run x "" "" --env --repos texts --yes
  ck "2nd environment secret fails: rc != 0, NO delete, clear note" '[ "$rc" != 0 ] && ! grep -q "secret delete" "$tmp/args" && grep -q "NOT deleted" "$tmp/out"'
  EXTRA="REVIEWER=alice MOCK_FAIL_MATCH=GPG_PRIVATE_KEY"
  run x "" "" --env --repos texts --yes
  ck "1st environment secret fails: rc != 0, NO delete" '[ "$rc" != 0 ] && ! grep -q "secret delete" "$tmp/args"'
  EXTRA="REVIEWER=alice MOCK_FAIL_MATCH=PUT MOCK_FAIL_CODE=403"
  run x "" "" --env --repos texts --yes
  ck "403 is explained as refused (not as 404)" '[ "$rc" != 0 ] && grep -q "HTTP 403: refused" "$tmp/out" && ! grep -q "secret set" "$tmp/args"'
  EXTRA="REVIEWER=alice MOCK_POLICY_EXISTS=1"
  run x "" "" --env --repos texts --yes
  ck "idempotent: tag rule exists, no second POST" '[ "$rc" = 0 ] && ! grep -q -- "-X POST" "$tmp/args"'
  EXTRA="REVIEWER=alice"
  run x "" $'y\nn' --env --repos texts
  ck "delete declined: no delete, rc 1 (unprotected copy remains)" '[ "$rc" = 1 ] && ! grep -q "secret delete" "$tmp/args" && grep -q "still exists" "$tmp/out"'
  run x "" $'y\ny' --env --repos texts
  ck "delete confirmed by answer y" '[ "$rc" = 0 ] && grep -q "secret delete GPG_PASSPHRASE" "$tmp/args"'
  EXTRA="REVIEWER=FreeSynergy"
  run x "" "" --env --repos texts --yes
  ck "reviewer = logged-in account: rc 2 without --allow-self-review" '[ "$rc" = 2 ] && grep -q "prevent_self_review" "$tmp/out"'
  EXTRA="REVIEWER=freesynergy"
  run x "" "" --env --repos texts --yes
  ck "reviewer = logged-in account in other case (freesynergy): still rc 2 (compared by id)" '[ "$rc" = 2 ] && ! grep -q "environments" "$tmp/args"'
  EXTRA="REVIEWER=someorg"
  run x "" "" --env --repos texts --yes
  ck "reviewer is an organisation: rc 2 with message, nothing set" '[ "$rc" = 2 ] && grep -q "Organization" "$tmp/out" && ! grep -q "environments" "$tmp/args"'
  EXTRA="REVIEWER=weird"
  run x "" "" --env --repos texts --yes
  ck "non numeric reviewer id: abort, nothing set" '[ "$rc" != 0 ] && ! grep -q "environments" "$tmp/args" && ! grep -q "secret set" "$tmp/args"'
  EXTRA="REVIEWER=bad;login"
  run x "" "" --env --repos texts --yes
  ck "invalid REVIEWER: rc 2, no environment call" '[ "$rc" = 2 ] && ! grep -q "environments" "$tmp/args"'
  EXTRA="REVIEWER=alice MOCK_FAIL_MATCH=-X<space>POST"
  run x "" "" --env --repos texts --yes
  ck "tag rule POST fails: rc != 0, no secrets set, no delete" '[ "$rc" != 0 ] && ! grep -q "secret set" "$tmp/args" && ! grep -q "secret delete" "$tmp/args"'
  EXTRA="REVIEWER=alice MOCK_FAIL_MATCH=secret<space>delete MOCK_FAIL_CODE=403"
  run x "" "" --env --repos texts --yes
  ck "403 on secret delete: rc != 0, not reported as already absent" '[ "$rc" != 0 ] && ! grep -q "already absent" "$tmp/out"'
  EXTRA="REVIEWER=alice MOCK_FAIL_MATCH=secret<space>delete MOCK_FAIL_CODE=404"
  run x "" "" --env --repos texts --yes
  ck "404 on secret delete is fine (already absent)" 'grep -q "already absent" "$tmp/out"'
  ck "script runs under set -euo pipefail" 'grep -q "^set -euo pipefail" "$SCRIPT_DIR/set-signing-secrets.sh"'
  EXTRA="REVIEWER=FreeSynergy MOCK_PREVENT=false MOCK_RID=1"
  run x "" "" --env --repos texts --yes --allow-self-review
  ck "--allow-self-review: body false, rc 0" '[ "$rc" = 0 ] && grep -q "\"prevent_self_review\":false" "$tmp/stdin"'
  EXTRA="REVIEWER=alice"
  run x "" "" --env --repos texts,packages --dry-run
  ck "dry run: rc 0, no gh call, no gpg call, nothing read" '[ "$rc" = 0 ] && [ ! -s "$tmp/args" ] && [ ! -s "$tmp/gpg" ] && [ ! -s "$tmp/stdin" ]'
  ck "dry run prints the calls, redacted, for both repos, no value" 'grep -q "redacted" "$tmp/out" && grep -q "fs-packages" "$tmp/out" && grep -q "secret set GPG_PRIVATE_KEY" "$tmp/out" && ! grep -q "$SECRETVAL" "$tmp/out"'
  ck "dry run left the state untouched (repo secrets still there)" '[ -e "$tmp/state/repo.GPG_PRIVATE_KEY" ]'
  EXTRA=""
  run x "" "y" --ruleset-tags fs-release
  ck "ruleset only: rc 0, no REVIEWER needed, no secrets, POST" '[ "$rc" = 0 ] && grep -q "^api -X POST repos/FreeSynergy-Net/fs-release/rulesets --input -" "$tmp/args" && ! grep -q "secret" "$tmp/args" && [ ! -s "$tmp/gpg" ]'
  ck "ruleset body: tag, active, refs/tags/v*, deletion + non_fast_forward + update, NO creation" 'grep -q "\"target\":\"tag\"" "$tmp/stdin" && grep -q "\"enforcement\":\"active\"" "$tmp/stdin" && grep -q "\"include\":\[\"refs/tags/v\*\"\]" "$tmp/stdin" && grep -q "\"rules\":\[{\"type\":\"deletion\"},{\"type\":\"non_fast_forward\"},{\"type\":\"update\"}\]" "$tmp/stdin" && ! grep -q "creation" "$tmp/stdin"'
  EXTRA="MOCK_RULESET_ID=7"
  run x "" "y" --ruleset-tags fs-release
  ck "ruleset idempotent: existing one is updated (PUT /7), not duplicated" '[ "$rc" = 0 ] && grep -q "^api -X PUT repos/FreeSynergy-Net/fs-release/rulesets/7 --input -" "$tmp/args" && ! grep -q -- "-X POST" "$tmp/args"'
  EXTRA=""
  run x "" "" --ruleset-tags fs-release --dry-run
  ck "ruleset dry run: nothing run" '[ "$rc" = 0 ] && [ ! -s "$tmp/args" ] && grep -q "rulesets" "$tmp/out"'
  EXTRA="REVIEWER=alice"
  run x "" "" --env --repos texts,modules-planning --dry-run
  ck "dry run: every gh call names FreeSynergy-Net/<repo>, NEVER the foreign org FreeSynergy/" '[ "$rc" = 0 ] && grep -q "FreeSynergy-Net/fs-texts" "$tmp/out" && grep -q "FreeSynergy-Net/fs-modules-planning" "$tmp/out" && ! grep -q "FreeSynergy/fs-" "$tmp/out" && ! grep -q "FreeSynergy/" "$tmp/args"'
  run x "" "" --env --repos fs-modules-planning --yes
  ck "--repos fs-modules-planning accepted; real run only talks to FreeSynergy-Net, never FreeSynergy/" '[ "$rc" = 0 ] && grep -q "PUT repos/FreeSynergy-Net/fs-modules-planning/environments" "$tmp/args" && ! grep -q "FreeSynergy/" "$tmp/args"'
  EXTRA="REVIEWER=alice FS_ORG=OtherOrg"
  run x "" "" --env --repos texts --yes
  ck "FS_ORG overrides the organisation" '[ "$rc" = 0 ] && grep -q "PUT repos/OtherOrg/fs-texts/environments" "$tmp/args" && ! grep -q "FreeSynergy-Net" "$tmp/args"'
  EXTRA="REVIEWER=alice"
  run x "" "n" --env --repos texts
  ck "target line shown; answer n: nothing written, rc != 0" '[ "$rc" != 0 ] && grep -q "Ziel: Organisation FreeSynergy-Net, Repositories: fs-texts" "$tmp/out" && ! grep -q "environments" "$tmp/args"'
  EXTRA="REVIEWER=alice MOCK_ROLE=member"
  run x "" "" --env --repos texts --yes
  ck "account is member but not admin: refused, nothing written" '[ "$rc" != 0 ] && grep -q "role admin is required" "$tmp/out" && ! grep -q "environments" "$tmp/args"'
  EXTRA="REVIEWER=alice MOCK_ROLE_FAIL=1"
  run x "" "" --env --repos texts --yes
  ck "account not in the organisation: clear message, nothing written" '[ "$rc" != 0 ] && grep -q "not a member of the organisation" "$tmp/out" && ! grep -q "environments" "$tmp/args"'
  EXTRA="REVIEWER=alice MOCK_MSTATE=pending"
  run x "" "" --env --repos texts --yes
  ck "pending invitation (state != active): refused, nothing written" '[ "$rc" != 0 ] && grep -q "state active is required" "$tmp/out" && ! grep -q "environments" "$tmp/args"'
  for bad in FreeSynergy freesynergy FREESYNERGY "-x" "a_b"; do
    EXTRA="REVIEWER=alice FS_ORG=$bad"
    run x "" "" --env --repos texts --dry-run
    ck "FS_ORG '$bad' rejected: rc 2, nothing run" '[ "$rc" = 2 ] && [ ! -s "$tmp/args" ] && grep -q "FS_ORG" "$tmp/out"'
  done
  EXTRA="REVIEWER=alice"
  run x "" "" --env --repos texts --dry-run
  ck "dry run also prints the Ziel line" 'grep -q "Ziel: Organisation FreeSynergy-Net, Repositories: fs-texts" "$tmp/out"'
  run x "" "y" --env-only --repos texts,messages
  ck "--env-only: environment + tag rule only; no secret set/delete, no gpg, no curl-free export, no delete question, no secret check" '[ "$rc" = 0 ] && grep -q "PUT repos/FreeSynergy-Net/fs-texts/environments/release-signing" "$tmp/args" && grep -q "POST repos/FreeSynergy-Net/fs-texts/environments/release-signing/deployment-branch-policies" "$tmp/args" && grep -q "PUT repos/FreeSynergy-Net/fs-messages/environments" "$tmp/args" && ! grep -q "secret" "$tmp/args" && [ ! -s "$tmp/gpg" ] && ! grep -qi "delete the repository" "$tmp/out" && ! grep -q "public key written" "$tmp/out"'
  run x "" "" --env-only --repos texts --yes
  ck "--env-only does not need the passphrase file or gpg" '[ "$rc" = 0 ] && [ ! -s "$tmp/gpg" ]'
  EXTRA=""
  run x "" "" --env-only --repos texts --dry-run
  ck "--env-only needs REVIEWER: rc 2" '[ "$rc" = 2 ] && grep -q "REVIEWER" "$tmp/out"'
  EXTRA="REVIEWER=alice"
  run x "" "" --env --env-only --repos texts
  ck "--env with --env-only: rc 2" '[ "$rc" = 2 ]'
  EXTRA="REVIEWER=alice"
  run x "" "" --repo-secrets --repos texts --yes
  ck "--repo-secrets: warning (deprecated, unprotected), repo secrets set, no environment, no delete" '[ "$rc" = 0 ] && grep -qi "deprecated" "$tmp/out" && grep -qi "unprotected" "$tmp/out" && grep -q "^secret set GPG_PRIVATE_KEY -R FreeSynergy-Net/fs-texts" "$tmp/args" && ! grep -q "environments" "$tmp/args" && ! grep -q "secret delete" "$tmp/args"'
  run x "" "" --env --repo-secrets --repos texts
  ck "--env with --repo-secrets: rc 2" '[ "$rc" = 2 ]'
  run x "" "" --bogus
  ck "unknown option: rc 2" '[ "$rc" = 2 ]'
}

if [ -n "${DEBUGRUN:-}" ]; then SCRIPT_DIR=$here; EXTRA="REVIEWER=alice"; run x "" "y" --env --repos texts; echo rc=$rc; cat "$tmp/out"; exit; fi
suite "$here"
echo "real script: $fails failure(s)"
real=$fails

# Mutants: each must turn the suite red.
mutant() { # <label> <file> <old> <new>
  local d="$tmp/mut" before=$fails
  rm -rf "$d"; mkdir -p "$d"; cp "$here/set-signing-secrets.sh" "$here/signing_env_lib.sh" "$d/"
  python3 - "$d/$2" "$3" "$4" <<'PY'
import sys
p, old, new = sys.argv[1:4]
s = open(p).read()
if s.count(old) != 1:
    sys.exit("mutation anchor not found exactly once: " + old)
open(p, "w").write(s.replace(old, new))
PY
  [ $? = 0 ] || { echo "FAIL mutant $1: anchor missing"; fails=$((fails + 1)); return; }
  fails=0
  suite "$d" >"$tmp/mutout"
  if [ "$fails" -gt 0 ]; then echo "ok   mutant red ($fails): $1"; fails=$before; else echo "FAIL mutant survived: $1"; fails=$((before + 1)); fi
}
fails=$real
mutant "delete before set" signing_env_lib.sh '  env_secrets "$r" ||' '  delete_repo_secrets "$r" || true; env_secrets "$r" ||'
mutant "secret value as argument" signing_env_lib.sh 'secret set GPG_PRIVATE_KEY --env' 'secret set GPG_PRIVATE_KEY --body "$(cat "$TMP/sub.asc")" --env'
mutant "ORG back to the foreign organisation" set-signing-secrets.sh 'ORG=${FS_ORG:-FreeSynergy-Net}' 'ORG=FreeSynergy'
mutant "admin role not checked" set-signing-secrets.sh '[ "${role% *}" = admin ] ||' 'true ||'
mutant "no target confirmation" set-signing-secrets.sh '[ "$ans" = y ] || [ "$ans" = Y ] || die "not confirmed' 'true || die "not confirmed'
mutant "env-only still sets secrets" set-signing-secrets.sh 'KEYS=1; [ "$MODE" != env-only ] || KEYS=0' 'KEYS=1'
mutant "env-only still runs the secret sequence" set-signing-secrets.sh '    if [ "$MODE" = env-only ]; then' '    if false; then'
mutant "FS_ORG not validated against FreeSynergy" set-signing-secrets.sh '[ "${ORG,,}" != freesynergy ] ||' 'true ||'
mutant "pending membership accepted" set-signing-secrets.sh '[ "${role#* }" = active ] ||' 'true ||'
mutant "no Ziel line in dry run" set-signing-secrets.sh 'echo "Ziel: Organisation' ': "Ziel: Organisation'
mutant "REVIEWER default" set-signing-secrets.sh 'REVIEWER=${REVIEWER:-} DEV=0' 'REVIEWER=${REVIEWER:-alice} DEV=0'
mutant "dry run executes" signing_env_lib.sh 'if [ "$DRY" = 1 ]; then
    printf' 'if false; then
    printf'
mutant "failed secret ignored (delete follows)" signing_env_lib.sh "echo \"error: \$r: environment secrets incomplete, the repository secrets were NOT deleted\" >&2; return 1; }" 'true; }'
mutant "tag rule not idempotent" signing_env_lib.sh 'if [ -n "$ids" ]; then' 'if false; then'
mutant "ruleset without update rule" signing_env_lib.sh '{type:"non_fast_forward"}, {type:"update"}' '{type:"non_fast_forward"}'
mutant "REVIEWER pattern removed" set-signing-secrets.sh '[[ "$REVIEWER" =~ ^[A-Za-z0-9]([A-Za-z0-9-]{0,38})$ ]] ||' 'true ||'
mutant "set -e removed" set-signing-secrets.sh 'set -euo pipefail' 'set -uo pipefail'
mutant "any delete error = already gone" signing_env_lib.sh 'elif [ "$(http_code)" = 404 ]; then' 'elif true; then'
mutant "tag rule error ignored" signing_env_lib.sh $'  env_policy "$r" || return 1\n  env_secrets' $'  env_policy "$r" || true\n  env_secrets'
mutant "reviewer id not checked numeric" set-signing-secrets.sh '[[ "$rid" =~ ^[0-9]+$ ]] ||' 'true ||'
mutant "self review compared by login, case sensitive" set-signing-secrets.sh '[ "$rid" = "$whoid" ]' '[ "$REVIEWER" = "$who" ]'
mutant "reviewer type not checked" set-signing-secrets.sh '[ "$rtype" = User ] ||' 'true ||'
mutant "no warning for --repo-secrets" set-signing-secrets.sh 'echo "WARNING: --repo-secrets is deprecated' ': "WARNING: --repo-secrets is deprecated'

echo "total failures: $fails"
[ "$fails" -eq 0 ]
