#!/usr/bin/env bash
# Self test of scripts/fetch-upstream with a FAKE `gh` in the PATH (no network): only GET calls of the fixed pattern, a missing gh, an error or a 403 is rc 3 with the
# message `Upstream nicht erreichbar`, the answer is normalised. Mutants: a write flag in the call, errors swallowed.
#   bash scripts/test_fetch_upstream.sh
set -uo pipefail
here="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
tool="$here/fetch-upstream"
py="$(command -v python3)"
tmp="$(mktemp -d)"
trap 'rm -rf "$tmp"' EXIT
fails=0
chk() { local name="$1"; shift; if "$@"; then echo "ok   $name"; else echo "FAIL $name"; fails=$((fails + 1)); fi; }
differs() { ! cmp -s "$1" "$2"; }
mkdir -p "$tmp/bin"
cat >"$tmp/bin/gh" <<'GH'
#!/usr/bin/env bash
# fake gh: logs the arguments, answers from FAKE_MODE
echo "$*" >>"$FAKE_LOG"
case "$FAKE_MODE" in
  fail403) echo "gh: HTTP 403: API rate limit exceeded" >&2; exit 1 ;;
  fail500) echo "gh: HTTP 500" >&2; exit 1 ;;
esac
ep="${*: -1}"
case "$ep" in
  *-X*) echo '[]' ;;
  */issues/73/comments) echo '[{"id":1,"user":{"login":"Alice"},"created_at":"2026-11-01T10:00:00Z","body":"one"}]'; echo '[{"id":2,"user":{"login":"FreeSynergy"},"created_at":"2026-11-02T10:00:00Z","body":"ours"}]' ;;
  */pulls/73/comments) echo '[{"id":1,"user":{"login":"bob"},"created_at":"2026-11-03T10:00:00Z","body":"line"}]' ;;
  */pulls/73/reviews) echo '[{"id":1,"user":{"login":"carol"},"submitted_at":"2026-11-04T10:00:00Z","state":"CHANGES_REQUESTED","body":"please"}]' ;;
  */pulls/73) echo '{"state":"open","merged":false,"mergeable":true,"draft":false,"title":"x"}' ;;
  */issues/80/comments) echo '[]' ;;
  *) echo "unexpected $ep" >&2; exit 1 ;;
esac
GH
chmod +x "$tmp/bin/gh"
printf '{"status":"ok","leer":false,"nummern":[{"repo":"project-blinc/Blinc","nummer":73,"art":"pull"},{"repo":"project-blinc/Blinc","nummer":80,"art":"issue"}]}\n' >"$tmp/liste.json"
run() { # run <tool> <mode> -> rc, files in $tmp
  : >"$tmp/log"; rm -f "$tmp/out.json"
  FAKE_MODE="$2" FAKE_LOG="$tmp/log" PATH="$tmp/bin:$(dirname "$py"):/usr/bin:/bin" "$py" -I "$1" --liste "$tmp/liste.json" --out "$tmp/out.json" 2>"$tmp/err"
}
q() { "$py" -I -c 'import json,sys; d=json.load(open(sys.argv[1])); print(eval(sys.argv[2]))' "$tmp/out.json" "$1"; }
run "$tool" ok; rc=$?
chk "rc 0" [ "$rc" -eq 0 ]
chk "5 calls: 4 for the pull request, 1 for the issue" [ "$(wc -l <"$tmp/log")" -eq 5 ]
chk "every call is exactly: api --paginate repos/<repo>/<...> (GET, no flag)" [ "$(grep -cE '^api --paginate repos/project-blinc/Blinc/(issues/[0-9]+/comments|pulls/[0-9]+(/comments|/reviews)?)$' "$tmp/log")" -eq 5 ]
chk "paged answers are joined (two issue comments + line + review)" [ "$(q 'len(d["eintraege"][0]["kommentare"])')" = 4 ]
chk "review time is submitted_at, state is kept" [ "$(q '[(c["created_at"], c["state"]) for c in d["eintraege"][0]["kommentare"] if c["kind"]=="review"][0]')" = "('2026-11-04T10:00:00Z', 'CHANGES_REQUESTED')" ]
chk "pull state is kept, the issue has none" [ "$(q 'd["eintraege"][0]["pull"]["mergeable"], d["eintraege"][1]["pull"]')" = "(True, None)" ]
chk "status ok, no errors" [ "$(q 'd["status"], d["fehler"]')" = "('ok', [])" ]
for mode in fail403 fail500; do
  run "$tool" "$mode"; rc=$?
  chk "$mode: rc 3" [ "$rc" -eq 3 ]
  chk "$mode: message Upstream nicht erreichbar" grep -q "Upstream nicht erreichbar" "$tmp/err"
  chk "$mode: stops at the first failed call (no retry, no tight loop)" [ "$(wc -l <"$tmp/log")" -eq 1 ]
  chk "$mode: the file says error with the call named" [ "$(q 'd["status"], len(d["fehler"])')" = "('error', 1)" ]
done
run "$tool" fail403
chk "403 is named in the file" [ "$(q '"403" in d["fehler"][0]')" = True ]
# no gh at all: a PATH with python3 only
mkdir -p "$tmp/pybin"; ln -s "$py" "$tmp/pybin/python3"
PATH="$tmp/pybin" python3 -I "$tool" --liste "$tmp/liste.json" --out "$tmp/out.json" 2>"$tmp/err"; rc=$?
chk "missing gh: rc 3" [ "$rc" -eq 3 ]
chk "missing gh: message" grep -q "Upstream nicht erreichbar" "$tmp/err"
# bad input
printf '{"nummern":[{"repo":"a b/c","nummer":1,"art":"pull"}]}' >"$tmp/bad.json"
"$py" -I "$tool" --liste "$tmp/bad.json" --out "$tmp/out2.json" 2>/dev/null; rc=$?
chk "a repo name with a space is refused before any call (rc 2)" [ "$rc" -eq 2 ]
printf '{"nummern":[{"repo":"a/b","nummer":"1;x","art":"pull"}]}' >"$tmp/bad.json"
"$py" -I "$tool" --liste "$tmp/bad.json" --out "$tmp/out2.json" 2>/dev/null; rc=$?
chk "a number that is no integer is refused (rc 2)" [ "$rc" -eq 2 ]
# the source has no write words: no method flag, no field, no input file, no issue/pr write command
chk "no write flag in the source" bash -c "! grep -nE -e '--method|--input|-f body|gh (issue|pr) ' '$tool' | grep -v '^[0-9]*:.*#' | grep -q ."
# mutant 1: a write flag in the call
sed 's/\["gh", "api", "--paginate", endpoint\]/["gh", "api", "-X", "POST", "--paginate", endpoint]/' "$tool" >"$tmp/m1"
chk "mutant 1 changed something" differs "$tmp/m1" "$tool"
run "$tmp/m1" ok
chk "mutant 1 (write flag in the call) is caught by the call-pattern check" [ "$(grep -cE '^api --paginate repos/' "$tmp/log")" -ne 5 ]
# mutant 2: errors are swallowed
sed 's/if r.returncode != 0:/if False:/' "$tool" >"$tmp/m2"
chk "mutant 2 changed something" differs "$tmp/m2" "$tool"
run "$tmp/m2" fail403; rc=$?
chk "mutant 2 (errors swallowed) is caught by the rc 3 case" [ "$rc" -ne 3 ]
# mutant 3: the endpoint pattern accepts everything; a forged endpoint (a flag smuggled into the argument) must be refused
forged() { FAKE_MODE=ok FAKE_LOG="$tmp/log" PATH="$tmp/bin:$(dirname "$py"):/usr/bin:/bin" "$py" -I - "$1" <<'PY'
import importlib.machinery, importlib.util, sys
loader = importlib.machinery.SourceFileLoader("fu", sys.argv[1])
spec = importlib.util.spec_from_loader("fu", loader)
m = importlib.util.module_from_spec(spec)
loader.exec_module(m)
try:
    m.gh_get("repos/a/b/issues/1/comments -X POST", 5)
except m.Unreachable:
    sys.exit(0)
sys.exit(1)
PY
}
sed 's/if not ENDPOINT.match(endpoint):/if False:/' "$tool" >"$tmp/m3"
chk "mutant 3 changed something" differs "$tmp/m3" "$tool"
chk "a forged endpoint is refused" forged "$tool"
refused_not() { ! forged "$1"; }
chk "mutant 3 (pattern check removed) is caught by the forged endpoint" refused_not "$tmp/m3"
if [ "$fails" -eq 0 ]; then echo "all ok"; else echo "$fails failed"; exit 1; fi
