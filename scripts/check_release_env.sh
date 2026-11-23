#!/usr/bin/env bash
# Report ONLY (changes nothing): is the GitHub environment `release-signing` set up as docs/usage.md ("Approval for the signing job") requires,
# in every repo that calls `release.yml` of fs-release? Read-only calls of `gh api` (GET); a write flag (-X, --method, -f, -F, --field, --raw-field,
# --input) is refused inside this script. No secret VALUES are read or printed, only the NAMES of secrets.
#
#   scripts/check_release_env.sh [--repos texts,packages,...] [--org FreeSynergy] [--env release-signing]
#                                [--tag v0.0.10 --sha a084f8b9...] [--strict] [--out report.txt]
#
# Per repo (one line in the table, TSV block below the readable table):
#   env        the environment exists (`gh api repos/<org>/<repo>/environments/<env>`; 404 = fehlt)
#   reviewers  number of reviewers in the protection rule `required_reviewers` (must be >= 1)
#   tag_rule   deployment branch policy with a TAG policy `v*` (deployment-branch-policies)
#   env_sec    GPG_PRIVATE_KEY and GPG_PASSPHRASE among the secrets of the environment (count of 2)
#   repo_sec   the same names among the repository secrets (both present next to complete env secrets = double holding, a hint, not an error)
#   release    the repo calls fs-release/.github/workflows/release.yml (info only; looks at .github/workflows/release*.yml; "unbekannt" without access)
#   result     OK = environment + reviewer + tag rule + both env secrets; WARN = environment without protection or secrets only in the repo; FEHLT = 404;
#              nicht nötig = 404 and the repo does not call release.yml (not counted; release = unbekannt stays FEHLT)
# Section fs-release: rulesets (only evaluated at status ok; 404/empty/no array = "Tag-Schutz unbekannt"; [] = "Tag-Schutz fehlt") and, with --tag, whether git/ref/tags/<tag> points at --sha (prefix compare).
# Exit code: 0 = report made (also with WARN/FEHLT: that is the finding); 1 = with --strict at least one repo is not OK (nicht nötig excluded) or the tag is a MISMATCH;
#            2 = call or gh error (401, 403, 5xx, network, no jq/gh): NEVER reported as "fehlt", only a 404 means "fehlt".
set -euo pipefail

org="FreeSynergy"; env_name="release-signing"; tag=""; sha=""; strict=0; out=""
repos="texts,packages,people,calendars,tasks,groups,maps,capnpc,interfaces,types,rpc,objects,jscalendar,identity,registry,forms,files,activities,broadcast,draw,messages,resources"
required=(GPG_PRIVATE_KEY GPG_PASSPHRASE)
gh_bin="${GH:-gh}"

usage() { sed -n '2,20p' "${BASH_SOURCE[0]}" | sed 's/^# \{0,1\}//'; }
die() { echo "check_release_env: $*" >&2; exit 2; }

while [ "$#" -gt 0 ]; do
  case "$1" in
    --repos) [ "$#" -ge 2 ] || die "--repos needs a value"; repos="$2"; shift 2 ;;
    --org) [ "$#" -ge 2 ] || die "--org needs a value"; org="$2"; shift 2 ;;
    --env) [ "$#" -ge 2 ] || die "--env needs a value"; env_name="$2"; shift 2 ;;
    --tag) [ "$#" -ge 2 ] || die "--tag needs a value"; tag="$2"; shift 2 ;;
    --sha) [ "$#" -ge 2 ] || die "--sha needs a value"; sha="$2"; shift 2 ;;
    --out) [ "$#" -ge 2 ] || die "--out needs a value"; out="$2"; shift 2 ;;
    --strict) strict=1; shift ;;
    -h|--help) usage; exit 0 ;;
    *) die "unknown argument: $1 (see --help)" ;;
  esac
done
[ -z "$sha" ] || [ -n "$tag" ] || die "--sha needs --tag"
for v in "$org" "$env_name" "$tag" "$sha" "${repos//,/ }"; do
  case "$v" in *[!A-Za-z0-9._\ -]*) die "unusual characters in an argument: refused" ;; esac
done
command -v jq >/dev/null 2>&1 || die "jq is missing"
command -v "$gh_bin" >/dev/null 2>&1 || die "gh is missing"

tmp="$(mktemp -d)"
trap 'rm -rf "$tmp"' EXIT

# Only reading calls: a write flag is refused here (the self test also fails when the mock gh sees one).
ghapi_guard() {
  local a
  for a in "$@"; do
    case "$a" in -X|-X?*|--method|--method=*|-f|-F|--field|--raw-field|--input) echo "check_release_env: internal error, write flag $a refused" >&2; exit 2 ;; esac
  done
}

# api <gh api arguments>: sets API_ST = ok | 404 | err and API_OUT (stdout) / API_ERR (first part of the error text)
api() {
  ghapi_guard "$@"
  API_OUT=""; API_ERR=""
  if API_OUT="$("$gh_bin" api "$@" 2>"$tmp/err")"; then API_ST=ok
  elif grep -q 'HTTP 404' "$tmp/err"; then API_ST=404
  else API_ST=err; API_ERR="$(head -c 200 "$tmp/err" | tr '\n' ' ')"; fi
}
# must <path>: like api, but everything except ok and 404 ends the run with rc 2 (401/403/5xx are not "fehlt")
api_strict() {
  api "$1"
  [ "$API_ST" != err ] || die "gh api $1 failed ($API_ERR): not reported as 'fehlt', run stopped"
}

# names_of <json> <jq path of the array>: only the names
names_of() { jq -r "$2" <<<"$1" 2>/dev/null || true; }
# count_required <names>: sets $cnt = how many of the required secrets are in the list and $missing = the missing names (no subshell)
count_required() {
  local n; cnt=0; missing=""
  for n in "${required[@]}"; do
    if grep -qx "$n" <<<"$1"; then cnt=$((cnt + 1)); else missing="$missing $n"; fi
  done
}

rows="$tmp/rows"; : >"$rows"; notes="$tmp/notes"; : >"$notes"; not_ok=0
for r in ${repos//,/ }; do
  case "$r" in fs-*) repo="$r" ;; *) repo="fs-$r" ;; esac
  base="repos/$org/$repo"
  env_s="fehlt"; rev="-"; trule="-"; esec="-"; rsec="-"; call="unbekannt"; res="FEHLT"; note=""

  # (4) repository secrets (names only)
  api_strict "$base/actions/secrets"
  rnames=""; [ "$API_ST" != ok ] || rnames="$(names_of "$API_OUT" '.secrets[]?.name')"
  count_required "$rnames"; rcount="$cnt"; rsec="$rcount/${#required[@]}"

  # (5) does the repo call release.yml of fs-release (info; no access = unbekannt)
  api "$base/contents/.github/workflows"
  if [ "$API_ST" = 404 ]; then call="nein"
  elif [ "$API_ST" = ok ]; then
    call="nein"
    for f in $(names_of "$API_OUT" '.[]? | select(.type=="file") | .name | select(test("^release.*[.]ya?ml$"))'); do
      api -H 'Accept: application/vnd.github.raw+json' "$base/contents/.github/workflows/$f"
      if [ "$API_ST" = ok ] && grep -Eq '(FreeSynergy-Net|FreeSynergy)/fs-release/[.]github/workflows/release[.]yml' <<<"$API_OUT"; then call="ja"; break; fi
      [ "$API_ST" = ok ] || { call="unbekannt"; break; }
    done
  fi

  # (1) the environment
  api_strict "$base/environments/$env_name"
  if [ "$API_ST" = 404 ]; then
    if [ "$call" = nein ]; then res="nicht nötig"; note="no release.yml call, environment not needed (404)"
    else note="environment missing (404): GitHub creates it unprotected at the first run"; fi
  else
    env_s="ja"; envjson="$API_OUT"
    # (2) protection: required_reviewers with >= 1 reviewer; deployment branch policy with the tag policy v*
    rev="$(jq -r '[.protection_rules[]? | select(.type=="required_reviewers") | .reviewers[]?] | length' <<<"$envjson" 2>/dev/null || echo 0)"
    case "$rev" in ""|*[!0-9]*) rev=0 ;; esac  # broken or empty JSON: never counts as reviewer
    trule="nein"
    if [ "$(jq -r '.deployment_branch_policy.custom_branch_policies // false' <<<"$envjson" 2>/dev/null)" = true ]; then
      api_strict "$base/environments/$env_name/deployment-branch-policies"
      if [ "$API_ST" = ok ] && [ "$(jq -r '[.branch_policies[]? | select(.name=="v*" and .type=="tag")] | length' <<<"$API_OUT" 2>/dev/null || echo 0)" -ge 1 ]; then trule="ja"; fi
    fi
    # (3) environment secrets (names only)
    api_strict "$base/environments/$env_name/secrets"
    enames=""; [ "$API_ST" != ok ] || enames="$(names_of "$API_OUT" '.secrets[]?.name')"
    count_required "$enames"; ecount="$cnt"; emissing="$missing"; esec="$ecount/${#required[@]}"
    if [ "$rev" -ge 1 ] && [ "$trule" = ja ] && [ "$ecount" -eq "${#required[@]}" ]; then res="OK"; else res="WARN"; fi
    [ "$rev" -ge 1 ] || note="$note no required reviewer;"
    [ "$trule" = ja ] || note="$note no tag rule v*;"
    [ -z "$emissing" ] || note="$note environment secrets missing:$emissing;"
    if [ "$ecount" -lt "${#required[@]}" ] && [ "$rcount" -gt 0 ]; then note="$note secrets only in the repository (arrive unprotected);"; fi
    if [ "$ecount" -eq "${#required[@]}" ] && [ "$rcount" -gt 0 ]; then note="$note hint: same secrets also in the repository (double holding, delete the repository ones);"; fi
  fi
  [ "$res" = OK ] || [ "$res" = "nicht nötig" ] || not_ok=$((not_ok + 1))
  printf '%s\t%s\t%s\t%s\t%s\t%s\t%s\t%s\t%s\n' "$repo" "$env_s" "$rev" "$trule" "$esec" "$rsec" "$call" "$res" "${note# }" >>"$rows"
done

# fs-release itself: rulesets and the tag
api_strict "repos/$org/fs-release/rulesets"
rs="Tag-Schutz unbekannt (rulesets: no usable answer)"
if [ "$API_ST" = ok ] && jq -e 'type=="array"' <<<"$API_OUT" >/dev/null 2>&1; then
  if [ "$(jq -r 'length' <<<"$API_OUT")" -eq 0 ]; then rs="Tag-Schutz fehlt (no rulesets)"
  else rs="rulesets: $(jq -r 'length' <<<"$API_OUT") (target tag: $(jq -r '[.[] | select(.target=="tag")] | length' <<<"$API_OUT"))"; fi
fi
mismatch=0
tagline="tag: not asked (--tag)"
if [ -n "$tag" ]; then
  api_strict "repos/$org/fs-release/git/ref/tags/$tag"
  if [ "$API_ST" = 404 ]; then tagline="tag $tag: missing (404)"
  else
    got="$(jq -r '.object.sha // ""' <<<"$API_OUT")"; typ="$(jq -r '.object.type // ""' <<<"$API_OUT")"
    if [ "$typ" = tag ]; then api_strict "repos/$org/fs-release/git/tags/$got"; got="$(jq -r '.object.sha // ""' <<<"$API_OUT")"; fi
    if [ -z "$sha" ]; then tagline="tag $tag points at ${got:0:12} (no --sha given)"
    elif [ -n "$got" ] && [ "${got#"$sha"}" != "$got" ]; then tagline="tag $tag points at the expected commit ${got:0:12}: OK"
    else tagline="tag $tag points at ${got:0:12}, expected $sha: MISMATCH"; mismatch=1; fi
  fi
fi

report() {
  hdr=$'repo\tenv\treviewers\ttag_rule\tenv_sec\trepo_sec\trelease\tresult\tnote'
  echo "release environment check: org=$org env=$env_name (report only, nothing was changed)"
  echo
  if command -v column >/dev/null 2>&1; then { echo "$hdr"; cat "$rows"; } | column -t -s $'\t'; else { echo "$hdr"; cat "$rows"; }; fi
  echo
  echo "fs-release: $rs"
  echo "fs-release: $tagline"
  echo "summary: $(grep -c . "$rows") repos, not OK: $not_ok (nicht nötig does not count)"
  echo
  echo "# TSV"
  echo "$hdr"
  cat "$rows"
}
if [ -n "$out" ]; then report | tee "$out"; else report; fi
[ "$strict" -eq 0 ] || { [ "$not_ok" -eq 0 ] && [ "$mismatch" -eq 0 ]; } || exit 1
exit 0
