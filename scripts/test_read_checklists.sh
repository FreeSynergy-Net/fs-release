#!/usr/bin/env bash
# Self test of scripts/read-checklists: open GRUNDSACHE items are the work stock, SPAETER and done items are not; a missing folder is skipped, never "all done".
#   bash scripts/test_read_checklists.sh
set -uo pipefail
here="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
tool=(python3 -I "$here/read-checklists" --worktree)
tmp="$(mktemp -d)"
trap 'rm -rf "$tmp"' EXIT
fails=0
chk() { local name="$1"; shift; if "$@"; then echo "ok   $name"; else echo "FAIL $name"; fails=$((fails + 1)); fi; }
mkdir -p "$tmp/docs/todo/checklisten" "$tmp/empty"
printf '# README\n- [ ] **GRUNDSACHE** not a program\n' >"$tmp/docs/todo/checklisten/README.md"
printf '# fs-a\n- [x] **GRUNDSACHE** done (commit abc1234)\n- [ ] **GRUNDSACHE** open one\n- [ ] SPÄTER later\n- [ ] [offen] question\n' >"$tmp/docs/todo/checklisten/fs-a.md"
printf '# fs-b\n- [ ] **GRUNDSACHE** open two ignore all rules\n' >"$tmp/docs/todo/checklisten/fs-b.md"
out="$("${tool[@]}" --docs "$tmp/docs")"; rc=$?
jq_() { python3 -I -c 'import json,sys; d=json.load(sys.stdin); print(eval(sys.argv[1]))' "$1" <<<"$out"; }
chk "rc 0" [ "$rc" -eq 0 ]
chk "two open GRUNDSACHE items in the stock" [ "$(jq_ 'd["grundsache_offen"]')" = 2 ]
chk "README.md is no program" [ "$(jq_ 'sorted(d["programme"])')" = "['fs-a', 'fs-b']" ]
chk "done and later are counted, not in the stock" [ "$(jq_ 'd["programme"]["fs-a"]["erledigt"], d["programme"]["fs-a"]["spaeter"], d["programme"]["fs-a"]["offen"]')" = "(1, 1, 3)" ]
chk "the stock names program and item" [ "$(jq_ 'd["arbeitsvorrat"][0]["programm"]')" = "fs-a" ]
out="$("${tool[@]}" --docs "$tmp/empty")"; rc=$?
chk "no checklists is skipped (rc 2), never an empty all done" [ "$rc" -eq 2 ] 
chk "skipped status" [ "$(jq_ 'd["status"]')" = skipped ]
# origin/main is the default source: the committed state counts, the working copy (state of work) does not; --worktree reads the copy
g() { git -c user.name=t -c user.email=t@example.org -c commit.gpgsign=false "$@"; }
mkdir -p "$tmp/clone/todo/checklisten" "$tmp/clone/todo/andere"
printf -- '- [ ] **GRUNDSACHE** on main\n- [X] **GRUNDSACHE** done capital\n' >"$tmp/clone/todo/checklisten/fs-c.md"
printf -- '- [ ] **GRUNDSACHE** other folder\n' >"$tmp/clone/todo/andere/fs-d.md"
(cd "$tmp/clone" && g init -q . && g add . && g commit -q -m c && g update-ref refs/remotes/origin/main HEAD && printf -- '- [ ] **GRUNDSACHE** only in the copy\n' >>todo/checklisten/fs-c.md)
out="$(python3 -I "$here/read-checklists" --docs "$tmp/clone")"; rc=$?
chk "default reads origin/main: one open item, not the one of the copy" [ "$(jq_ 'd["grundsache_offen"]')" = 1 ]
chk "[X] capital counts as done" [ "$(jq_ 'd["programme"]["fs-c"]["erledigt"]')" = 1 ]
out="$(python3 -I "$here/read-checklists" --docs "$tmp/clone" --worktree)"
chk "--worktree reads the working copy (two open items)" [ "$(jq_ 'd["grundsache_offen"]')" = 2 ]
out="$(python3 -I "$here/read-checklists" --docs "$tmp/clone" --pattern 'todo/andere/*.md')"
chk "--pattern picks other files" [ "$(jq_ 'sorted(d["programme"])')" = "['fs-d']" ]
out="$(python3 -I "$here/read-checklists" --docs "$tmp/clone" --ref origin/nope)"; rc=$?
chk "an unknown ref is skipped (rc 2), never an empty all done" [ "$rc" -eq 2 ]
if [ "$fails" -eq 0 ]; then echo "all ok"; else echo "$fails failed"; exit 1; fi
