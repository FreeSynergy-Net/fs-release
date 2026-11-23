#!/usr/bin/env bash
# Set up the release signing key in FreeSynergy repositories: GitHub environment `release-signing` with a required reviewer,
# tag rule `v*`, the two signing secrets as ENVIRONMENT secrets, then removal of the old (unprotected) repository secrets.
#
#   REVIEWER=<login> bash scripts/set-signing-secrets.sh --env --repos texts,packages --dry-run     # first: only prints the gh calls
#   REVIEWER=<login> bash scripts/set-signing-secrets.sh --env --repos texts,packages               # asks before deleting, per repository
#   REVIEWER=<login> bash scripts/set-signing-secrets.sh --env --yes                                # all callers of release.yml, no questions
#   bash scripts/set-signing-secrets.sh --ruleset-tags fs-release                                   # only the tag ruleset (no secrets)
#
# Modes:
#   --env             (default) per repository, in this order: (1) PUT environment release-signing (required reviewer, prevent_self_review,
#                     deployment only for named refs), (2) tag rule v* (listed first, never twice), (3) GPG_PRIVATE_KEY and GPG_PASSPHRASE as
#                     environment secrets, (4) delete the same-named repository secrets, ONLY after both environment secrets were set and only
#                     after confirmation (--yes, else a question per repository), (5) read-only check; any deviation = rc 1.
#   --repo-secrets    old mode: repository secrets (deprecated, unprotected: any workflow of the repo can read them).
# Options:
#   --repos a,b       repositories (program names `texts` or `fs-texts`); default: the repositories that call release.yml. `all` = every non-archived fs-*.
#   --reviewer LOGIN  or the variable REVIEWER: the account that approves a signing run (REQUIRED for --env, no default; never an agent account).
#   --allow-self-review  switch prevent_self_review off (default: on, see docs/usage.md); needed when REVIEWER is the account that pushes the tag.
#   --yes             delete the repository secrets without asking.
#   --ruleset-tags R  tag ruleset "tags v* immutable" in repository R (rules deletion, non_fast_forward, update: tags v* cannot be deleted or moved; new tags can still be created).
#                     Alone it does nothing else; with --repos it runs in addition.
#   --dry-run         only print the gh calls (no value, `<redacted>`), run nothing, read nothing (also no gpg, no gh); rc 0.
# The signing subkey is exported from the keyring, only the secret SUBKEY (the primary key never leaves the keyring):
#   1. newest valid signing subkey of the release key (one is created, ed25519, 1 year, if there is none), 2. export of that subkey only,
#   3. the public key (primary + subkeys) and the subkey fingerprint go to $OUT_DIR, 4. upload to keys.openpgp.org (NO_UPLOAD=1 skips).
# Settings (environment): PASSFILE (passphrase file, mode 600, default ~/.config/freesynergy/release-passphrase), GNUPGHOME,
#   OUT_DIR (default /srv/FreeSynergy/.scratch/keys), OUT_OWNER (default kal:kal), FS_SIGN_TMP (private dir for the export, default /dev/shm).
# Needs: gpg, gh (logged in as FreeSynergy; the account must be admin of the repositories), curl, jq.
# shellcheck disable=SC2034  # YES, REVIEWER_ID_JSON, PREVENT, DEV, MODE, DRY are read by signing_env_lib.sh
set -euo pipefail
set +x
umask 077

PRIMARY=16A4A3EE563AEF51FCF0C55DED7B57C4A1904342
PASSFILE=${PASSFILE:-$HOME/.config/freesynergy/release-passphrase}
OUT_DIR=${OUT_DIR:-/srv/FreeSynergy/.scratch/keys}
OUT_OWNER=${OUT_OWNER:-kal:kal}
ORG=FreeSynergy-Net
DEFAULT_REPOS=(fs-texts fs-people fs-calendars fs-tasks fs-groups fs-maps fs-capnpc fs-packages fs-files)  # the 9 callers of release.yml

die() { echo "error: $*" >&2; exit 1; }
usage() { echo "error: $*" >&2; echo "usage: REVIEWER=<login> $0 [--env|--repo-secrets] [--repos a,b] [--allow-self-review] [--yes] [--ruleset-tags <repo>] [--dry-run]" >&2; exit 2; }

whoid="" rinfo="" rid="" rtype="" MODE="" DRY=0 YES=0 PREVENT=true RULESET_REPO="" REPOS_ARG="" REVIEWER=${REVIEWER:-} DEV=0
REPOS=()
while [ $# -gt 0 ]; do
  case "$1" in
    --env | --repo-secrets)
      [ -z "$MODE" ] || [ "$MODE" = "${1#--}" ] || usage "--env and --repo-secrets exclude each other"
      MODE=${1#--} ;;
    --repos) [ $# -ge 2 ] || usage "--repos needs a list"; REPOS_ARG=$2; shift ;;
    --reviewer) [ $# -ge 2 ] || usage "--reviewer needs a login"; REVIEWER=$2; shift ;;
    --ruleset-tags) [ $# -ge 2 ] || usage "--ruleset-tags needs a repository"; RULESET_REPO=$2; shift ;;
    --allow-self-review) PREVENT=false ;;
    --yes) YES=1 ;;
    --dry-run) DRY=1 ;;
    -h | --help) sed -n "2,29p" "$0"; exit 0 ;;
    -*) usage "unknown option $1" ;;
    *) REPOS_ARG=${REPOS_ARG:+$REPOS_ARG,}$1 ;;
  esac
  shift
done

norm() { local n=${1#fs-}; [[ "$n" =~ ^[a-z0-9][a-z0-9-]*$ ]] || usage "bad repository name '$1'"; echo "fs-$n"; }

DO_SECRETS=1
if [ -n "$RULESET_REPO" ] && [ -z "$REPOS_ARG" ] && [ -z "$MODE" ]; then DO_SECRETS=0; fi
MODE=${MODE:-env}
RULESET_REPO=${RULESET_REPO:+$(norm "$RULESET_REPO")}

if [ "$DO_SECRETS" = 1 ]; then
  if [ "$MODE" = env ]; then
    [ -n "$REVIEWER" ] || usage "REVIEWER (login of the approving account) is required for --env; there is no default"
    [[ "$REVIEWER" =~ ^[A-Za-z0-9]([A-Za-z0-9-]{0,38})$ ]] || usage "REVIEWER '$REVIEWER' is not a GitHub login"
  else
    echo "WARNING: --repo-secrets is deprecated and unprotected: repository secrets reach every workflow run of the repository, with no approval and no tag rule. Use --env." >&2
  fi
  if [ "$REPOS_ARG" = all ]; then
    [ "$DRY" = 0 ] || usage "'all' needs gh (a listing) and cannot be a dry run; name the repositories"
  elif [ -n "$REPOS_ARG" ]; then
    IFS=, read -ra raw <<<"$REPOS_ARG"
    for x in "${raw[@]}"; do REPOS+=("$(norm "$x")"); done
  else
    REPOS=("${DEFAULT_REPOS[@]}")
  fi
fi

command -v jq >/dev/null || die "jq is needed"
if [ "$DRY" = 0 ]; then
  for tool in gh curl; do command -v "$tool" >/dev/null || die "$tool is needed"; done
  who=$(gh api user --jq .login </dev/null) || die "gh is not logged in (gh auth login)"
  whoid=$(gh api user --jq .id </dev/null) || die "cannot read the id of the logged-in account"
  [ "$who" = "$ORG" ] || die "gh is logged in as '$who', not '$ORG' (gh auth switch -u $ORG)"
fi

TMP=$(mktemp -d "${FS_SIGN_TMP:-/dev/shm}/fs-sign.XXXXXX") || die "no private temp dir"
chmod 700 "$TMP"
# shellcheck disable=SC2154  # f is the loop variable inside the single-quoted trap
trap 'for f in sub.asc body out err names; do shred -u "$TMP/$f" 2>/dev/null || rm -f "$TMP/$f"; done; rmdir "$TMP" 2>/dev/null || true' EXIT
# shellcheck source=signing_env_lib.sh
. "$(dirname "${BASH_SOURCE[0]}")/signing_env_lib.sh"

REVIEWER_ID_JSON='"<id of the reviewer>"'
if [ "$DO_SECRETS" = 1 ] && [ "$MODE" = env ] && [ "$DRY" = 0 ]; then
  rinfo=$(gh api "users/$REVIEWER" --jq '"\(.id) \(.type)"' </dev/null 2>"$TMP/err") || { explain "user $REVIEWER"; exit 1; }
  rid=${rinfo%% *} rtype=${rinfo#* }
  [[ "$rid" =~ ^[0-9]+$ ]] || die "no numeric id for $REVIEWER"
  [ "$rtype" = User ] || usage "REVIEWER '$REVIEWER' is of type '$rtype', not a User (an organisation cannot approve a run)"
  # compare by numeric id (logins are case-insensitive): the same account as the one that pushes the tags can never approve its own run
  if [ "$PREVENT" = true ] && [ "$rid" = "$whoid" ]; then
    usage "REVIEWER is the logged-in account '$who': with prevent_self_review the run that account starts could never be approved; use another account or --allow-self-review"
  fi
  REVIEWER_ID_JSON=$rid
elif [ "$DO_SECRETS" = 1 ] && [ "$MODE" = env ]; then
  REVIEWER_ID_JSON=$(jq -nc --arg r "$REVIEWER" '"<id of \($r)>"')
fi

if [ "$DO_SECRETS" = 1 ] && [ "$DRY" = 0 ]; then
  command -v gpg >/dev/null || die "gpg is needed"
  [ -f "$PASSFILE" ] || die "passphrase file $PASSFILE not found"
  [ "$(stat -c %a "$PASSFILE")" = 600 ] || die "$PASSFILE must have mode 600 (chmod 600 $PASSFILE)"
  gpg --list-secret-keys "$PRIMARY" >/dev/null 2>&1 || die "secret key $PRIMARY is not in this keyring"
  GPGP=(gpg --batch --pinentry-mode loopback --passphrase-file "$PASSFILE")
  newest_sign_subkey() {
    # ssb/sub lines: field 2 validity, 6 created, 12 capabilities; fpr line follows.
    gpg --with-colons --with-subkey-fingerprints --list-secret-keys "$PRIMARY" | awk -F: -v now="$(date +%s)" '
      $1=="ssb" { ok = ($2 !~ /[rei]/) && ($12 ~ /s/) && ($7=="" || $7>now); created=$6; want=1; next }
      $1=="fpr" && want { if (ok && created>best) { best=created; fpr=$10 } want=0 }
      END { print fpr }'
  }
  SUB=$(newest_sign_subkey)
  if [ -z "$SUB" ]; then
    echo "no valid signing subkey: creating one (ed25519, 1 year)"
    "${GPGP[@]}" --quick-add-key "$PRIMARY" ed25519 sign 1y
    SUB=$(newest_sign_subkey)
    [ -n "$SUB" ] || die "could not create a signing subkey"
  fi
  echo "signing subkey: $SUB"
  "${GPGP[@]}" --armor --export-secret-subkeys "${SUB}!" >"$TMP/sub.asc"
  grep -q "BEGIN PGP PRIVATE KEY BLOCK" "$TMP/sub.asc" || die "export of the subkey failed"
  if [ "$REPOS_ARG" = all ]; then
    mapfile -t REPOS < <(gh repo list "$ORG" --limit 200 --json name,isArchived </dev/null \
      -q '.[] | select(.isArchived|not) | .name' | grep '^fs-' | grep -v -- '-archive$')
  fi
fi

FAILED=0
if [ "$DO_SECRETS" = 1 ]; then
  for r in "${REPOS[@]}"; do
    if [ "$MODE" = env ]; then
      env_run "$r" || FAILED=$((FAILED + 1))
    else
      GH_IN=$TMP/sub.asc gh_do "<redacted: signing subkey file>" secret set GPG_PRIVATE_KEY -R "$ORG/$r" || { explain "$r: GPG_PRIVATE_KEY"; FAILED=$((FAILED + 1)); continue; }
      GH_IN=$PASSFILE gh_do "<redacted: passphrase file>" secret set GPG_PASSPHRASE -R "$ORG/$r" || { explain "$r: GPG_PASSPHRASE"; FAILED=$((FAILED + 1)); continue; }
      echo "repository secrets set (deprecated): $r"
    fi
  done
fi
if [ -n "$RULESET_REPO" ]; then
  ruleset_tags "$RULESET_REPO" || FAILED=$((FAILED + 1))
fi

if [ "$DO_SECRETS" = 1 ]; then
  echo "--- check (read only) ---"
  for r in "${REPOS[@]}"; do verify_repo "$r"; done
fi

if [ "$DO_SECRETS" = 1 ] && [ "$DRY" = 0 ]; then
  mkdir -p "$OUT_DIR"
  gpg --armor --export "$PRIMARY" >"$OUT_DIR/release-signing-key.asc"
  echo "$SUB" >"$OUT_DIR/signing-subkey-fingerprint.txt"
  chown "$OUT_OWNER" "$OUT_DIR" "$OUT_DIR/release-signing-key.asc" "$OUT_DIR/signing-subkey-fingerprint.txt" 2>/dev/null \
    || echo "note: could not chown to $OUT_OWNER (run as root or as that user)"
  echo "public key written: $OUT_DIR/release-signing-key.asc"
  if [ "${NO_UPLOAD:-0}" != 1 ]; then
    gpg --export "$PRIMARY" | curl -fsS -T - https://keys.openpgp.org >/dev/null && echo "uploaded to keys.openpgp.org"
  fi
fi

if [ "$FAILED" -gt 0 ] || [ "$DEV" -gt 0 ]; then
  echo "NOT OK: $FAILED step failure(s), $DEV deviation(s)" >&2
  exit 1
fi
echo "done"
