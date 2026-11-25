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
#   --env-only        for organisation secrets: ONLY (1) the environment release-signing with required reviewer and (2) the tag rule v*; no signing key is
#                     exported, no secret is set or deleted, no gpg, no keys.openpgp.org, no delete question. The check reads environment and tag rule only.
#   --repo-secrets    old mode: repository secrets (deprecated, unprotected: any workflow of the repo can read them).
# Options:
#   --repos a,b       repositories (program names `texts` or `fs-texts`); default: the repositories that call release.yml. `all` = every non-archived fs-*.
#   --reviewer LOGIN  or the variable REVIEWER: the account that approves a signing run (REQUIRED for --env, no default; never an agent account).
#   --allow-self-review  switch prevent_self_review off (default: on, see docs/usage.md); needed when REVIEWER is the account that pushes the tag.
#   --ask-passphrase  type the signing-key passphrase (no echo, twice, from the terminal; the DEFAULT when PASSFILE is not set and ~/.config/freesynergy/release-passphrase does
#                     not exist). It is kept only in a shell variable, goes to gpg on a pipe (file descriptor) and to `gh secret set` on standard input; no file, no argument.
#                     GitHub still needs the passphrase as a secret (CI unlocks the subkey with it); only the local file is gone. Not used by --env-only (no key work).
#   --yes             no questions: neither the target confirmation (organisation + repositories) nor the delete confirmation.
#   --ruleset-tags R  tag ruleset "tags v* immutable" in repository R (rules deletion, non_fast_forward, update: tags v* cannot be deleted or moved; new tags can still be created).
#                     Alone it does nothing else; with --repos it runs in addition.
#   --dry-run         only print the gh calls (no value, `<redacted>`), run nothing, read nothing (also no gpg, no gh); rc 0.
# The signing subkey is exported from the keyring, only the secret SUBKEY (the primary key never leaves the keyring):
#   1. newest valid signing subkey of the release key (one is created, ed25519, 1 year, if there is none), 2. export of that subkey only,
#   3. the public key (primary + subkeys) and the subkey fingerprint go to $OUT_DIR, 4. upload to keys.openpgp.org (NO_UPLOAD=1 skips).
# Settings (environment): PASSFILE (alternative to typing: passphrase file, mode 600; an existing ~/.config/freesynergy/release-passphrase is used as before), GNUPGHOME,
#   OUT_DIR (default /srv/FreeSynergy/.scratch/keys), OUT_OWNER (default kal:kal), FS_SIGN_TMP (private dir for the export, default /dev/shm).
#   FS_ORG            the GitHub organisation (default FreeSynergy-Net, where our repositories are). Never `FreeSynergy` (a different organisation).
# Needs: gpg, gh (logged in with YOUR account; it must be admin of the organisation, checked via orgs/$ORG/memberships/<you>), curl, jq.
# The line "Ziel: Organisation ..., Repositories: ..." is always printed (also in a dry run); before every real write it is confirmed (without --yes).
# shellcheck disable=SC2034  # ASK, PW, YES, REVIEWER_ID_JSON, PREVENT, DEV, MODE, DRY are read by signing_env_lib.sh
set -euo pipefail
set +x
umask 077

PRIMARY=16A4A3EE563AEF51FCF0C55DED7B57C4A1904342
PASSFILE=${PASSFILE:-}
PW=""   # the typed passphrase (ask mode): only ever in this shell variable, never in a file, argument or output; unset on EXIT
OUT_DIR=${OUT_DIR:-/srv/FreeSynergy/.scratch/keys}
OUT_OWNER=${OUT_OWNER:-kal:kal}
ORG=${FS_ORG:-FreeSynergy-Net}  # the GitHub organisation of our repositories; NOT the unrelated organisation `FreeSynergy`
DEFAULT_REPOS=(fs-texts fs-people fs-calendars fs-tasks fs-groups fs-maps fs-capnpc fs-packages fs-files fs-messages fs-broadcast fs-draw fs-modules-planning fs-modules-person fs-starter)  # callers of release.yml: 9 old ones, fs-messages, fs-broadcast, fs-draw (checked 25.11.), plus fs-modules-planning, fs-modules-person (no caller yet) and fs-starter, planned for tag releases

die() { echo "error: $*" >&2; exit 1; }
usage() { echo "error: $*" >&2; echo "usage: REVIEWER=<login> $0 [--env|--env-only|--repo-secrets] [--repos a,b] [--allow-self-review] [--yes] [--ruleset-tags <repo>] [--dry-run]" >&2; exit 2; }

[[ "$ORG" =~ ^[A-Za-z0-9][A-Za-z0-9-]{0,38}$ ]] || usage "FS_ORG '$ORG' is not a GitHub organisation name"
[ "${ORG,,}" != freesynergy ] || usage "FS_ORG 'FreeSynergy' is the unrelated foreign organisation; our repositories are in FreeSynergy-Net"

ASKFLAG=0 whoid="" ans="" role="" tgt=() rinfo="" rid="" rtype="" MODE="" DRY=0 YES=0 PREVENT=true RULESET_REPO="" REPOS_ARG="" REVIEWER=${REVIEWER:-} DEV=0
REPOS=()
while [ $# -gt 0 ]; do
  case "$1" in
    --env | --env-only | --repo-secrets)
      [ -z "$MODE" ] || [ "$MODE" = "${1#--}" ] || usage "--env, --env-only and --repo-secrets exclude each other"
      MODE=${1#--} ;;
    --repos) [ $# -ge 2 ] || usage "--repos needs a list"; REPOS_ARG=$2; shift ;;
    --reviewer) [ $# -ge 2 ] || usage "--reviewer needs a login"; REVIEWER=$2; shift ;;
    --ruleset-tags) [ $# -ge 2 ] || usage "--ruleset-tags needs a repository"; RULESET_REPO=$2; shift ;;
    --allow-self-review) PREVENT=false ;;
    --ask-passphrase) ASKFLAG=1 ;;
    --yes) YES=1 ;;
    --dry-run) DRY=1 ;;
    -h | --help) sed -n "2,36p" "$0"; exit 0 ;;
    -*) usage "unknown option $1" ;;
    *) REPOS_ARG=${REPOS_ARG:+$REPOS_ARG,}$1 ;;
  esac
  shift
done

norm() { local n=${1#fs-}; [[ "$n" =~ ^[a-z0-9][a-z0-9-]*$ ]] || usage "bad repository name '$1'"; echo "fs-$n"; }

DO_SECRETS=1
if [ -n "$RULESET_REPO" ] && [ -z "$REPOS_ARG" ] && [ -z "$MODE" ]; then DO_SECRETS=0; fi
MODE=${MODE:-env}
# Passphrase source: typed (ask mode, default) unless PASSFILE names a file or the old default file exists; --ask-passphrase forces typing.
[ -n "$PASSFILE" ] || { [ ! -f "$HOME/.config/freesynergy/release-passphrase" ] || PASSFILE=$HOME/.config/freesynergy/release-passphrase; }
ASK=1; [ "$ASKFLAG" = 1 ] || [ -z "$PASSFILE" ] || ASK=0
KEYS=1; [ "$MODE" != env-only ] || KEYS=0   # KEYS=1: the signing key is exported and secrets are set/deleted (never with --env-only)
RULESET_REPO=${RULESET_REPO:+$(norm "$RULESET_REPO")}

if [ "$DO_SECRETS" = 1 ]; then
  if [ "$MODE" = env ] || [ "$MODE" = env-only ]; then
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
  role=$(gh api "orgs/$ORG/memberships/$who" --jq '"\(.role) \(.state)"' </dev/null 2>/dev/null) \
    || die "'$who' is not a member of the organisation '$ORG' (or gh may not read it); log in with an account that is admin of '$ORG' (gh auth switch)"
  [ "${role% *}" = admin ] || die "'$who' has the role '${role% *}' in '$ORG', but the role admin is required"
  [ "${role#* }" = active ] || die "the membership of '$who' in '$ORG' is '${role#* }' (an invitation must be accepted first); state active is required"
fi
tgt=("${REPOS[@]}"); [ -z "$RULESET_REPO" ] || tgt+=("$RULESET_REPO")
if [ "$REPOS_ARG" = all ]; then tgt=("(every non-archived fs-* repository)"); fi
echo "Ziel: Organisation $ORG, Repositories: ${tgt[*]}" >&2
if [ "$DRY" = 0 ] && [ "$YES" != 1 ]; then
  printf 'Write to the organisation %s as shown above? [y/N] ' "$ORG" >&2
  read -r ans || ans=n
  [ "$ans" = y ] || [ "$ans" = Y ] || die "not confirmed; nothing was changed"
fi

TMP=$(mktemp -d "${FS_SIGN_TMP:-/dev/shm}/fs-sign.XXXXXX") || die "no private temp dir"
chmod 700 "$TMP"
# shellcheck disable=SC2154  # f is the loop variable inside the single-quoted trap
trap 'unset PW; for f in sub.asc body out err names; do shred -u "$TMP/$f" 2>/dev/null || rm -f "$TMP/$f"; done; rmdir "$TMP" 2>/dev/null || true' EXIT
# shellcheck source=signing_env_lib.sh
. "$(dirname "${BASH_SOURCE[0]}")/signing_env_lib.sh"

REVIEWER_ID_JSON='"<id of the reviewer>"'
if [ "$DO_SECRETS" = 1 ] && [ "$MODE" != repo-secrets ] && [ "$DRY" = 0 ]; then
  rinfo=$(gh api "users/$REVIEWER" --jq '"\(.id) \(.type)"' </dev/null 2>"$TMP/err") || { explain "user $REVIEWER"; exit 1; }
  rid=${rinfo%% *} rtype=${rinfo#* }
  [[ "$rid" =~ ^[0-9]+$ ]] || die "no numeric id for $REVIEWER"
  [ "$rtype" = User ] || usage "REVIEWER '$REVIEWER' is of type '$rtype', not a User (an organisation cannot approve a run)"
  # compare by numeric id (logins are case-insensitive): the same account as the one that pushes the tags can never approve its own run
  if [ "$PREVENT" = true ] && [ "$rid" = "$whoid" ]; then
    usage "REVIEWER is the logged-in account '$who': with prevent_self_review the run that account starts could never be approved; use another account or --allow-self-review"
  fi
  REVIEWER_ID_JSON=$rid
elif [ "$DO_SECRETS" = 1 ] && [ "$MODE" != repo-secrets ]; then
  REVIEWER_ID_JSON=$(jq -nc --arg r "$REVIEWER" '"<id of \($r)>"')
fi

if [ "$DO_SECRETS" = 1 ] && [ "$DRY" = 0 ] && [ "$REPOS_ARG" = all ]; then
  mapfile -t REPOS < <(gh repo list "$ORG" --limit 200 --json name,isArchived </dev/null \
    -q '.[] | select(.isArchived|not) | .name' | grep '^fs-' | grep -v -- '-archive$')
fi

if [ "$DO_SECRETS" = 1 ] && [ "$KEYS" = 1 ] && [ "$DRY" = 0 ]; then
  command -v gpg >/dev/null || die "gpg is needed"
  if [ "$ASK" = 1 ]; then
    [ -t 0 ] || die "the passphrase is typed, but there is no terminal (stdin is not a terminal); run it in a terminal, or give PASSFILE (mode 600)"
    pw2=""
    read -rsp "Passphrase of the signing key (not shown): " PW || die "no passphrase read"; echo >&2
    read -rsp "Passphrase again (typo check): " pw2 || die "no passphrase read"; echo >&2
    [ -n "$PW" ] && [ "$PW" = "$pw2" ] || { unset pw2; die "the two entries differ or are empty; nothing was done"; }
    unset pw2
  else
    [ -f "$PASSFILE" ] || die "passphrase file $PASSFILE not found"
    [ "$(stat -c %a "$PASSFILE")" = 600 ] || die "$PASSFILE must have mode 600 (chmod 600 $PASSFILE)"
  fi
  gpg --list-secret-keys "$PRIMARY" >/dev/null 2>&1 || die "secret key $PRIMARY is not in this keyring"
  # gpgp <gpg args>: gpg with the passphrase from the file, or (ask mode) from a pipe on file descriptor 3 (printf is a builtin: nothing in the process list)
  gpgp() {
    if [ "$ASK" = 1 ]; then
      gpg --batch --pinentry-mode loopback --passphrase-fd 3 "$@" 3< <(printf '%s' "$PW")
    else
      gpg --batch --pinentry-mode loopback --passphrase-file "$PASSFILE" "$@"
    fi
  }
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
    gpgp --quick-add-key "$PRIMARY" ed25519 sign 1y
    SUB=$(newest_sign_subkey)
    [ -n "$SUB" ] || die "could not create a signing subkey"
  fi
  echo "signing subkey: $SUB"
  gpgp --armor --export-secret-subkeys "${SUB}!" >"$TMP/sub.asc"
  grep -q "BEGIN PGP PRIVATE KEY BLOCK" "$TMP/sub.asc" || die "export of the subkey failed"
fi

FAILED=0
if [ "$DO_SECRETS" = 1 ]; then
  for r in "${REPOS[@]}"; do
    if [ "$MODE" = env-only ]; then
      env_only_run "$r" || FAILED=$((FAILED + 1))
    elif [ "$MODE" = env ]; then
      env_run "$r" || FAILED=$((FAILED + 1))
    else
      GH_IN=$TMP/sub.asc gh_do "<redacted: signing subkey file>" secret set GPG_PRIVATE_KEY -R "$ORG/$r" || { explain "$r: GPG_PRIVATE_KEY"; FAILED=$((FAILED + 1)); continue; }
      pass_gh_do secret set GPG_PASSPHRASE -R "$ORG/$r" || { explain "$r: GPG_PASSPHRASE"; FAILED=$((FAILED + 1)); continue; }
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

if [ "$DO_SECRETS" = 1 ] && [ "$KEYS" = 1 ] && [ "$DRY" = 0 ]; then
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
