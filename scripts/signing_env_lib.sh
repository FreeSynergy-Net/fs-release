#!/usr/bin/env bash
# Helpers of scripts/set-signing-secrets.sh (source it, do not execute it): the GitHub environment `release-signing`, its tag rule,
# the environment secrets, the removal of the old repository secrets, the read-only check and the tag ruleset.
# Expects: ORG, TMP (private dir), DRY (0|1), REVIEWER_ID_JSON, PREVENT (true|false), PASSFILE, ASK, PW, DEV (deviation counter).
# Secret values are only ever handed to gh on STANDARD INPUT (`< file`), never as an argument, never printed.
# shellcheck shell=bash
ENV_NAME=release-signing
SECRETS=(GPG_PRIVATE_KEY GPG_PASSPHRASE)
RULESET_NAME="tags v* immutable"

say() { if [ "$DRY" = 1 ]; then printf '(dry run, not done) %s\n' "$*"; else printf '%s\n' "$*"; fi; }

# gh_do <stdin-note> <gh args...>: runs gh (stdin from $GH_IN, else /dev/null; stdout -> $TMP/out, stderr -> $TMP/err).
# With DRY=1 it only prints the call (the note says what would be on stdin; never a secret value) and runs nothing.
gh_do() {
  local note=$1
  shift
  if [ "$DRY" = 1 ]; then
    printf '+ gh'
    printf ' %q' "$@"
    [ -z "$note" ] || printf '   < %s' "$note"
    printf '\n'
    : >"$TMP/out"
    : >"$TMP/err"
    return 0
  fi
  gh "$@" <"${GH_IN:-/dev/null}" >"$TMP/out" 2>"$TMP/err"
}

# pass_gh_do <gh args>: the passphrase on standard input, from the file or (ask mode) from a pipe fed by the shell variable (never a file, never an argument).
pass_gh_do() {
  if [ "$ASK" != 1 ]; then GH_IN=$PASSFILE gh_do "<redacted: passphrase file>" "$@"; return; fi
  if [ "$DRY" = 1 ]; then gh_do "<redacted: typed passphrase>" "$@"; return; fi
  gh "$@" < <(printf '%s' "$PW") >"$TMP/out" 2>"$TMP/err"
}

http_code() { grep -oE 'HTTP [0-9]{3}' "$TMP/err" | head -n1 | cut -d' ' -f2 || true; }

# explain <what>: after a failed gh call, say WHY in one line (404 is not 403 is not 5xx).
explain() {
  local c m
  c=$(http_code)
  case "${c:-}" in
    404) m="HTTP 404: not found (GitHub also answers 404 when this account has no admin right on the repository)" ;;
    401 | 403) m="HTTP $c: refused (missing right or rate limit); do not repeat at once, wait" ;;
    422) m="HTTP 422: refused as invalid (for example the reviewer has no access to the repository)" ;;
    5??) m="HTTP $c: GitHub server error, try again later" ;;
    *) m="failed (no HTTP code in the answer)" ;;
  esac
  printf 'error: %s: %s\n' "$1" "$m" >&2
}

deviation() {
  printf 'DEVIATION: %s\n' "$*" >&2
  DEV=$((DEV + 1))
}

# (1) environment with required reviewer and tag-only deployment
env_put() {
  local r=$1
  jq -nc --argjson id "$REVIEWER_ID_JSON" --argjson p "$PREVENT" \
    '{prevent_self_review:$p, reviewers:[{type:"User", id:$id}], deployment_branch_policy:{protected_branches:false, custom_branch_policies:true}}' >"$TMP/body"
  GH_IN=$TMP/body gh_do "$(cat "$TMP/body")" api -X PUT "repos/$ORG/$r/environments/$ENV_NAME" --input - || { explain "$r: environment $ENV_NAME"; return 1; }
  say "environment $ENV_NAME set: $r"
}

# (2) tag rule v* (list first, never twice)
env_policy() {
  local r=$1 base ids
  base="repos/$ORG/$r/environments/$ENV_NAME/deployment-branch-policies"
  gh_do "" api "$base" || { explain "$r: list tag rules"; return 1; }
  ids=$(jq -r '.branch_policies[]? | select(.name == "v*" and .type == "tag") | .id' "$TMP/out") || return 1
  if [ -n "$ids" ]; then
    say "tag rule v* exists: $r"
    return 0
  fi
  printf '%s' '{"name":"v*","type":"tag"}' >"$TMP/body"
  GH_IN=$TMP/body gh_do "$(cat "$TMP/body")" api -X POST "$base" --input - || { explain "$r: create tag rule v*"; return 1; }
  say "tag rule v* created: $r"
}

# (3) the two secrets, BOTH as environment secrets, values only on stdin
env_secrets() {
  local r=$1
  GH_IN=$TMP/sub.asc gh_do "<redacted: signing subkey file>" secret set GPG_PRIVATE_KEY --env "$ENV_NAME" -R "$ORG/$r" || { explain "$r: environment secret GPG_PRIVATE_KEY"; return 1; }
  pass_gh_do secret set GPG_PASSPHRASE --env "$ENV_NAME" -R "$ORG/$r" || { explain "$r: environment secret GPG_PASSPHRASE"; return 1; }
  say "environment secrets set: $r"
}

# (4) the old repository secrets; a secret that is already gone (404) is fine
delete_repo_secrets() {
  local r=$1 n
  for n in "${SECRETS[@]}"; do
    if gh_do "" secret delete "$n" -R "$ORG/$r"; then
      say "repository secret deleted: $r $n"
    elif [ "$(http_code)" = 404 ]; then
      say "repository secret already absent: $r $n"
    else
      explain "$r: delete repository secret $n"
      return 1
    fi
  done
}

# confirm_delete <repo>: --yes, or ask (stdin; no answer = no). Dry run: no question.
confirm_delete() {
  local ans=n
  [ "$DRY" = 1 ] && return 0
  [ "${YES:-0}" = 1 ] && return 0
  printf 'Delete the repository secrets GPG_PRIVATE_KEY and GPG_PASSPHRASE of %s now? [y/N] ' "$1" >&2
  read -r ans || ans=n
  [ "$ans" = y ] || [ "$ans" = Y ]
}

# --env-only: environment with reviewer and tag rule, nothing else (the secrets are organisation secrets).
env_only_run() {
  local r=$1
  env_put "$r" || return 1
  env_policy "$r" || return 1
}

# The whole sequence for one repository. Deleting happens ONLY after both environment secrets are set.
env_run() {
  local r=$1
  env_put "$r" || return 1
  env_policy "$r" || return 1
  env_secrets "$r" || { echo "error: $r: environment secrets incomplete, the repository secrets were NOT deleted" >&2; return 1; }
  if confirm_delete "$r"; then
    delete_repo_secrets "$r" || return 1
  else
    echo "note: $r: repository secrets kept (not confirmed); they stay unprotected until deleted" >&2
  fi
}

# (5) read-only check; every mismatch counts as a deviation (rc != 0 at the end)
# secret_names <gh secret list args>: the NAMES (never values) into $TMP/names
secret_names() { gh_do "" secret list "$@" --json name --jq '.[].name' || return 1; cp "$TMP/out" "$TMP/names"; }

verify_repo() {
  local r=$1 n names
  if [ "$MODE" = repo-secrets ]; then
    secret_names -R "$ORG/$r" || { explain "$r: list repository secrets"; DEV=$((DEV + 1)); return 0; }
    names=$(cat "$TMP/names")
    for n in "${SECRETS[@]}"; do
      [ "$DRY" = 1 ] || grep -qx "$n" <<<"$names" || deviation "$r: repository secret $n missing"
    done
    return 0
  fi
  if gh_do "" api "repos/$ORG/$r/environments/$ENV_NAME"; then
    if [ "$DRY" != 1 ]; then
      jq -e --argjson id "$REVIEWER_ID_JSON" '[.protection_rules[]? | select(.type == "required_reviewers") | .reviewers[]?.reviewer.id] | index($id) != null' "$TMP/out" >/dev/null || deviation "$r: reviewer not set"
      jq -e --argjson p "$PREVENT" '[.protection_rules[]? | select(.type == "required_reviewers") | .prevent_self_review] | first == $p' "$TMP/out" >/dev/null || deviation "$r: prevent_self_review differs"
      jq -e '.deployment_branch_policy.custom_branch_policies == true' "$TMP/out" >/dev/null || deviation "$r: deployment is not limited to named refs"
    fi
  else
    explain "$r: read environment"
    deviation "$r: environment $ENV_NAME not readable"
  fi
  if gh_do "" api "repos/$ORG/$r/environments/$ENV_NAME/deployment-branch-policies"; then
    [ "$DRY" = 1 ] || jq -e '[.branch_policies[]? | select(.name == "v*" and .type == "tag")] | length > 0' "$TMP/out" >/dev/null || deviation "$r: tag rule v* missing"
  else
    explain "$r: read tag rules"
    deviation "$r: tag rules not readable"
  fi
  [ "$MODE" != env-only ] || return 0
  if secret_names --env "$ENV_NAME" -R "$ORG/$r"; then
    names=$(cat "$TMP/names")
    for n in "${SECRETS[@]}"; do
      [ "$DRY" = 1 ] || grep -qx "$n" <<<"$names" || deviation "$r: environment secret $n missing"
    done
  else
    explain "$r: list environment secrets"
    deviation "$r: environment secrets not readable"
  fi
  if secret_names -R "$ORG/$r"; then
    names=$(cat "$TMP/names")
    for n in "${SECRETS[@]}"; do
      [ "$DRY" = 1 ] || ! grep -qx "$n" <<<"$names" || deviation "$r: repository secret $n still exists (unprotected copy)"
    done
  else
    explain "$r: list repository secrets"
    deviation "$r: repository secrets not readable"
  fi
}

# Tag ruleset: tags v* cannot be deleted or moved (`deletion`, `non_fast_forward`, `update`: without `update` a tag could still be moved to a successor commit);
# creating a new tag stays allowed (NO `creation` rule).
ruleset_tags() {
  local r=$1 base id
  base="repos/$ORG/$r/rulesets"
  jq -nc --arg n "$RULESET_NAME" '{name:$n, target:"tag", enforcement:"active", conditions:{ref_name:{include:["refs/tags/v*"], exclude:[]}}, rules:[{type:"deletion"}, {type:"non_fast_forward"}, {type:"update"}]}' >"$TMP/body"
  gh_do "" api "$base?includes_parents=false" || { explain "$r: list rulesets"; return 1; }
  id=$(jq -r --arg n "$RULESET_NAME" '[.[] | select(.name == $n) | .id] | first // empty' "$TMP/out") || return 1
  if [ -n "$id" ]; then
    GH_IN=$TMP/body gh_do "$(cat "$TMP/body")" api -X PUT "$base/$id" --input - || { explain "$r: update ruleset"; return 1; }
    say "ruleset updated: $r ($id)"
  else
    GH_IN=$TMP/body gh_do "$(cat "$TMP/body")" api -X POST "$base" --input - || { explain "$r: create ruleset"; return 1; }
    say "ruleset created: $r"
    [ "$DRY" != 1 ] || say "(dry run: an existing ruleset of that name would be updated with PUT $base/<id> instead)"
  fi
}
