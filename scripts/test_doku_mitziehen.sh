#!/usr/bin/env bash
# Self test of scripts/doku-mitziehen with small git fixtures (a code repo and a docs clone, a fake mkdocs on PATH):
# green, code without docs, only one language, old name left over (and allowed), new name missing, broken link, mkdocs missing,
# round counter up to rc 3 and the reset on green, bad input. Counter-checks: every rule must really turn the gate red.
#   bash scripts/test_doku_mitziehen.sh
set -uo pipefail
here="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
tool=(python3 -I "$here/doku-mitziehen" --program fs-test)
tmp="$(mktemp -d)"
trap 'rm -rf "$tmp"' EXIT
fails=0
g() { git -c user.name=t -c user.email=t@example.org -c commit.gpgsign=false "$@"; }

mkdir -p "$tmp/bin"
printf '#!/bin/sh\nexit 0\n' >"$tmp/bin/mkdocs"
chmod +x "$tmp/bin/mkdocs"
mkdir -p "$tmp/empty"
ln -s "$(command -v git)" "$tmp/empty/git"
py="$(command -v python3)"

mkrepos() { # mkrepos <dir>: <dir>/code and <dir>/docs, one commit each; prints nothing
  mkdir -p "$1/code" "$1/docs/docs/de" "$1/docs/docs/en"
  (cd "$1/code" && g init -q . && echo a >a.rs && g add . && g commit -q -m base)
  (cd "$1/docs" && g init -q . && mkdir -p todo/checklisten &&
    printf '# fs-test\n- [x] **GRUNDSACHE** first (commit abc1234)\n- [ ] **GRUNDSACHE** second\n' >todo/checklisten/fs-test.md && echo "# Alt fs-old" >docs/de/a.md && echo "# Old fs-old" >docs/en/a.md &&
    echo "site_name: x" >mkdocs.yml && g add . && g commit -q -m base)
}
base() { g -C "$1" rev-parse HEAD; }

expect() { # expect <rc> <needle or -> <name> <dir> -- extra args...
  local want="$1" needle="$2" name="$3" dir="$4"
  shift 5
  local out rc
  out="$(PATH="$tmp/bin:$PATH" "${tool[@]}" --docs "$dir/docs" --docs-base "$DB" --code "$dir/code" --code-base "$CB" \
    --state "$dir/state" "$@" 2>&1)"
  rc=$?
  if [ "$rc" -ne "$want" ] || { [ "$needle" != "-" ] && ! grep -qF -- "$needle" <<<"$out"; }; then
    echo "FAIL $name: rc=$rc (want $want), needle '$needle'"
    printf "    %s\n" "${out//$'\n'/$'\n    '}"
    fails=$((fails + 1))
  else
    echo "ok   $name"
  fi
}

fresh() { # fresh <name>: new fixture dir in $d, bases in DB/CB
  d="$tmp/$1"
  mkrepos "$d"
  DB="$(base "$d/docs")"
  CB="$(base "$d/code")"
}
code_change() { (cd "$d/code" && echo b >>a.rs && g commit -q -am change); }
docs_both() { (cd "$d/docs" && echo "neu fs-new" >>docs/de/a.md && echo "new fs-new" >>docs/en/a.md &&
  echo '- [x] **GRUNDSACHE** third (commit def5678)' >>todo/checklisten/fs-test.md && g commit -q -am docs); }
docs_nolist() { (cd "$d/docs" && echo "neu fs-new" >>docs/de/a.md && echo "new fs-new" >>docs/en/a.md && g commit -q -am docs); }

fresh green; code_change; docs_both
expect 0 "green" "green: code and both languages of the docs changed" "$d" -- --new fs-new

fresh nolist; code_change; docs_nolist
expect 1 "checklist todo/checklisten/fs-test.md was not updated" "red: docs changed, but not the checklist of the program" "$d" -- --new fs-new

fresh noproof; code_change; docs_both
(cd "$d/docs" && echo '- [x] **GRUNDSACHE** fourth without proof' >>todo/checklisten/fs-test.md && g commit -q -am p)
expect 1 "ticked without a proof" "red: ticked without a proof" "$d" --

# C3: the proof must be a real one - a remark in parentheses, a short hex word or a capital [X] without a proof is RED
fresh remark; code_change; docs_both
(cd "$d/docs" && echo '- [x] **GRUNDSACHE** fifth (z. B. fs-text "gespeichert")' >>todo/checklisten/fs-test.md && g commit -q -am p)
expect 1 "is ticked without a proof" "red: a remark in parentheses is no proof" "$d" --

fresh shorthex; code_change; docs_both
(cd "$d/docs" && echo '- [x] **GRUNDSACHE** sixth (abc12)' >>todo/checklisten/fs-test.md && g commit -q -am p)
expect 1 "is ticked without a proof" "red: 5 hex characters are no proof (K3, own case)" "$d" --

fresh hexword; code_change; docs_both
(cd "$d/docs" && echo '- [x] **GRUNDSACHE** seventh (defaced)' >>todo/checklisten/fs-test.md && g commit -q -am p)
expect 1 "is ticked without a proof" "red: a hex-only word without a digit is no proof (K3, own case)" "$d" --

fresh hex6; code_change; docs_both
(cd "$d/docs" && echo '- [x] **GRUNDSACHE** seventh b (abc1234)' >>todo/checklisten/fs-test.md && echo '- [x] **GRUNDSACHE** six (abc123)' >>todo/checklisten/fs-test.md && g commit -q -am p)
expect 1 ":6 is ticked without a proof" "red: exactly 6 hex characters are no proof, 7 are (line 5 ok, line 6 red)" "$d" --

fresh bigx; code_change; docs_both
(cd "$d/docs" && echo '- [X] **GRUNDSACHE** eighth' >>todo/checklisten/fs-test.md && g commit -q -am p)
expect 1 "is ticked without a proof" "red: capital [X] without a proof is checked like [x]" "$d" --

fresh goodproofs; code_change; docs_both
(cd "$d/docs" && printf '%s\n' '- [x] **GRUNDSACHE** tag (tag v0.0.7)' '- [X] **GRUNDSACHE** picture (Bild bilder/start.png)' '- [x] **GRUNDSACHE** hash (commit 0123abcdef0123abcdef0123abcdef0123abcdef)' >>todo/checklisten/fs-test.md && g commit -q -am p)
expect 0 "green" "green: tag, picture path and long commit hash are proofs" "$d" --

fresh dropped; code_change; docs_both
(cd "$d/docs" && sed -i '/first (commit abc1234)/d' todo/checklisten/fs-test.md && g commit -q -am p)
expect 1 "a ticked item was removed" "red: a ticked item was removed" "$d" --

fresh nofile; code_change; docs_both
(cd "$d/docs" && g rm -q todo/checklisten/fs-test.md && g commit -q -m p)
expect 1 "its checklist todo/checklisten/fs-test.md does not exist" "red: no checklist file" "$d" --

fresh nodocs; code_change
expect 1 "documentation did not" "red: code changed, docs not" "$d" --

fresh onelang; code_change
(cd "$d/docs" && echo x >>docs/de/a.md && g commit -q -am de)
expect 1 "docs/en/a.md did not" "red: only the German page changed" "$d" --

fresh oldname; code_change; docs_both
expect 1 "old name 'fs-old' still in docs/de/a.md:1" "red: old name left over" "$d" -- --old fs-old
expect 0 "green" "green: old name only in an allowed history file" "$d" -- --old fs-old --allow 'docs/*/a.md'

fresh newname; code_change; docs_both
expect 1 "new name 'fs-zzz' appears nowhere" "red: new name missing" "$d" -- --new fs-zzz

fresh link; code_change; docs_both
(cd "$d/docs" && echo "see [x](nowhere.md#a)" >>docs/de/a.md && echo "see [x](a.md)" >>docs/en/a.md && g commit -q -am l)
expect 1 "broken link 'nowhere.md' in docs/de/a.md" "red: broken link" "$d" --

fresh nomk; code_change; docs_both
out="$(PATH="$tmp/empty" "$py" -I "$here/doku-mitziehen" --program fs-test --docs "$d/docs" --docs-base "$DB" --code "$d/code" \
  --code-base "$CB" --state "$d/state" 2>&1)"
rc=$?
if [ "$rc" -eq 2 ] && grep -qF "was NOT run" <<<"$out"; then echo "ok   mkdocs missing is skipped, never green"; else
  echo "FAIL mkdocs missing: rc=$rc"; echo "$out"; fails=$((fails + 1)); fi

fresh newword; code_change; docs_both
expect 1 "new name 'fs-ne' appears nowhere" "red: new name only as the start of a longer name (fs-new)" "$d" -- --new fs-ne

fresh mkred; code_change; docs_both
mkdir -p "$tmp/binred"
# shellcheck disable=SC2016
printf '#!/bin/sh\n[ "$1" = "--version" ] && exit 0\necho "ERROR broken nav" >&2\nexit 1\n' >"$tmp/binred/mkdocs"
chmod +x "$tmp/binred/mkdocs"
out="$(PATH="$tmp/binred:$PATH" "${tool[@]}" --docs "$d/docs" --docs-base "$DB" --code "$d/code" --code-base "$CB" --state "$d/state" 2>&1)"
rc=$?
if [ "$rc" -eq 1 ] && grep -qF "mkdocs build --strict failed" <<<"$out"; then echo "ok   red: mkdocs build fails"; else
  echo "FAIL mkdocs build red: rc=$rc"; echo "$out"; fails=$((fails + 1)); fi

# a git worktree: `.git` is a file there; the default state file must still work (rc 0, JSON with status green)
fresh wt; code_change; docs_both
g -C "$d/docs" worktree add -q "$d/wt" -b wtbranch
out="$(PATH="$tmp/bin:$PATH" "${tool[@]}" --docs "$d/wt" --docs-base "$DB" --code "$d/code" --code-base "$CB" --json "$d/wt.json" 2>&1)"
rc=$?
if [ "$rc" -eq 0 ] && grep -qF '"status": "green"' "$d/wt.json"; then echo "ok   green in a git worktree without --state"; else
  echo "FAIL worktree: rc=$rc"; echo "$out"; fails=$((fails + 1)); fi
rm -f "$(g -C "$d/wt" rev-parse --absolute-git-dir)/doku-mitziehen-runde"

# the JSON of a skipped run is never green (the flow keys on status == green)
fresh skipjson; code_change; docs_both
PATH="$tmp/empty" "$py" -I "$here/doku-mitziehen" --program fs-test --docs "$d/docs" --docs-base "$DB" --code "$d/code" --code-base "$CB" \
  --state "$d/state" --json "$d/skip.json" >/dev/null 2>&1
if grep -qF '"status": "skipped"' "$d/skip.json"; then echo "ok   skipped run writes status skipped"; else
  echo "FAIL skipped json"; fails=$((fails + 1)); fi
"${tool[@]}" --docs "$d/docs" --docs-base nothere --json "$d/bad.json" >/dev/null 2>&1
if grep -qF '"status": "skipped"' "$d/bad.json"; then echo "ok   bad input writes status skipped"; else
  echo "FAIL bad json"; fails=$((fails + 1)); fi

fresh rounds; code_change
expect 1 "runde=1" "round 1 red" "$d" --
expect 1 "runde=2" "round 2 red" "$d" --
expect 3 "runde=3" "round 3 red escalates" "$d" --
docs_both
expect 0 "runde=0" "green resets the counter" "$d" --

fresh bad
expect 2 "bad input" "bad base hash" "$d" -- --docs-base nothere
out="$("${tool[@]}" --docs "$d/docs" --docs-base "$DB" --code "$d/code" 2>&1)"; rc=$?
if [ "$rc" -eq 2 ]; then echo "ok   --code without --code-base"; else echo "FAIL code-base pairing rc=$rc"; fails=$((fails + 1)); fi

# the default state file (in the git folder of docs) is never written through a symlink
fresh sym; code_change; docs_both
sf="$(g -C "$d/docs" rev-parse --absolute-git-dir)/doku-mitziehen-runde"
: >"$tmp/victim"; ln -s "$tmp/victim" "$sf"
PATH="$tmp/bin:$PATH" "${tool[@]}" --docs "$d/docs" --docs-base "$DB" --code "$d/code" --code-base "$CB" --json "$d/sym.json" >/dev/null 2>&1; rc=$?
if [ "$rc" -eq 2 ] && [ ! -s "$tmp/victim" ] && grep -qF '"status": "skipped"' "$d/sym.json"; then echo "ok   a symlinked state file is refused (skipped, never green)"; else
  echo "FAIL symlink state: rc=$rc"; fails=$((fails + 1)); fi

# C4: a glob in a picture path is a proof (`*-de-*.png`)
fresh glob; code_change; docs_both
# shellcheck disable=SC2016
(cd "$d/docs" && echo '- [x] **GRUNDSACHE** pictures (`.scratch/wiki-0013-bilder/*-de-*.png` und `*-en-*.png`)' >>todo/checklisten/fs-test.md && g commit -q -am p)
expect 0 "green" "green: a picture path with * is a proof (C4)" "$d" --
fresh noglob; code_change; docs_both
(cd "$d/docs" && echo '- [x] **GRUNDSACHE** nothing (*-de-*)' >>todo/checklisten/fs-test.md && g commit -q -am p)
expect 1 "is ticked without a proof" "red: a bare glob without a file ending is no proof (C4 counter)" "$d" --

# C5: an OLD ticked line without a proof (before the base) never blocks; a NEW or CHANGED one does
fresh oldline
(cd "$d/docs" && echo '- [x] **GRUNDSACHE** old item (README)' >>todo/checklisten/fs-test.md && g commit -q -am old)
DB="$(base "$d/docs")"
code_change; docs_both
expect 0 "green" "green: an old ticked line without a proof does not block (C5)" "$d" --
(cd "$d/docs" && sed -i 's/old item (README)/old item reworded (README)/' todo/checklisten/fs-test.md && g commit -q -am rw)
expect 1 "is ticked without a proof" "red: the same old line CHANGED without a proof is checked (C5 counter)" "$d" --

# R8': the head of the AGENT's branch counts, not the checkout of the repo
fresh headb
(cd "$d/code" && g checkout -q -b unreviewed/x && echo b >>a.rs && g commit -q -am change && g checkout -q -)
docs_both
expect 0 "green" "green: no code change in the checkout, no --head (the old blind spot)" "$d" --
expect 1 "documentation did not" "red: --head sees the commit on the branch while main is checked out (R8')" "$d" -- --head unreviewed/x --docs-head "$DB"
expect 0 "green" "green: --head sees the code and the docs head has the docs" "$d" -- --head unreviewed/x
expect 2 "bad input" "bad --head ref" "$d" -- --head nope/none
out="$("${tool[@]}" --docs "$d/docs" --docs-base "$DB" --head HEAD 2>&1)"; rc=$?
if [ "$rc" -eq 2 ]; then echo "ok   --head without --code"; else echo "FAIL head pairing rc=$rc"; fails=$((fails + 1)); fi

# --docs-head: the docs are read at that commit, not in the working tree
fresh dhead; code_change; docs_both
DH="$(base "$d/docs")"
(cd "$d/docs" && echo "see [x](nowhere.md)" >>docs/de/a.md && echo "see [x](nowhere.md)" >>docs/en/a.md)
expect 0 "green" "green: --docs-head ignores the dirty working tree (the old state of the branch head is read)" "$d" -- --docs-head "$DH"
expect 1 "broken link" "red: without --docs-head the working tree is read" "$d" --

if [ "$fails" -eq 0 ]; then echo "all ok"; else echo "$fails failed"; exit 1; fi
