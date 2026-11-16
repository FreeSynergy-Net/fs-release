#!/usr/bin/env bash
# Self test of scripts/check_lock_sources.py and scripts/gates-lock.sh (fixtures: tests/fixtures/lock-sources/): one source is green, two sources of a
# library are red with a message that names both sources and who pulls them, V1+ (fs-types only from FreeSynergy/fs-types) is red only with
# the switch, a mutant (library list emptied) lets the red lock through = the test would be red, the gate warns first and fails with `strict`.
#   bash scripts/test_check_lock_sources.sh
set -uo pipefail
here="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
tool="$here/check_lock_sources.py"; gate="$here/gates-lock.sh"
fx="$here/../tests/fixtures/lock-sources"
tmp="$(mktemp -d)"
trap 'rm -rf "$tmp"' EXIT
fails=0
ok() { echo "ok   $1"; }
bad() { echo "FAIL $1"; fails=$((fails + 1)); }
rc_of() { python3 -I "$tool" "$@" >"$tmp/o" 2>&1; echo $?; }
expect() { # <label> <rc wanted> <args...>
  local l="$1" w="$2" r; shift 2; r=$(rc_of "$@")
  if [ "$r" = "$w" ]; then ok "$l"; else bad "$l: rc=$r wanted $w: $(cat "$tmp/o")"; fi
}
has() { if grep -qF -- "$2" "$tmp/o"; then ok "$1"; else bad "$1: missing '$2' in: $(cat "$tmp/o")"; fi; }

expect "green: one source per library" 0 "$fx/green.lock"
expect "red: two sources of fs-types" 1 "$fx/red-two-sources.lock"
has "red message names the library" 'ERROR'; has "red message names the first source" 'v0.0.26'
has "red message names the second source" 'v0.0.27'
has "red message names the packages that pull them" 'pulled by: demo-app 0.1.0 (workspace member)'
has "red message names a second puller" 'demo-people 0.1.0'
expect "V1+ lock without the switch is green" 0 "$fx/red-v1plus.lock"
expect "V1+ lock with the switch (--only-from) is red" 1 --only-from "$fx/red-v1plus.lock"
has "V1+ message names the allowed repo" 'allows only https://github.com/FreeSynergy/fs-types'
FS_LOCK_ONLY_FROM=1 python3 -I "$tool" "$fx/red-v1plus.lock" >"$tmp/o" 2>&1 && bad "env switch FS_LOCK_ONLY_FROM=1 must be red" || ok "env switch FS_LOCK_ONLY_FROM=1 is red"
expect "green stays green with the V1+ switch" 0 --only-from "$fx/green.lock"
expect "stdin" 1 - < "$fx/red-two-sources.lock"
expect "missing file is rc 2" 2 "$tmp/none.lock"
# repo root: every Cargo.lock outside target/ is found
mkdir -p "$tmp/r/sub" "$tmp/r/target/x"
cp "$fx/green.lock" "$tmp/r/Cargo.lock"; cp "$fx/red-two-sources.lock" "$tmp/r/sub/Cargo.lock"; cp "$fx/red-two-sources.lock" "$tmp/r/target/x/Cargo.lock"
expect "repo root: a nested lock is checked" 1 "$tmp/r"
rm "$tmp/r/sub/Cargo.lock"
expect "repo root: a lock under target/ is ignored" 0 "$tmp/r"
mkdir -p "$tmp/empty"; expect "repo root without a lock is green" 0 "$tmp/empty"
# mutant: library list emptied -> the red lock passes, i.e. the real checks above would be red without the library list
printf 'libraries = []\nonly_from_enabled = false\n' > "$tmp/mut.toml"
r=$(rc_of --config "$tmp/mut.toml" "$fx/red-two-sources.lock")
if [ "$r" = 0 ]; then ok "mutant (check switched off) passes the red lock, so the red test above is real"; else bad "mutant must pass the red lock, rc=$r"; fi
# the gate: warning first, error later
mkdir -p "$tmp/g"; cp "$fx/red-two-sources.lock" "$tmp/g/Cargo.lock"
bash "$gate" "$tmp/g" >"$tmp/o" 2>&1 && ok "gate: violation is only a warning while strict = false" || bad "gate: must stay green without strict: $(cat "$tmp/o")"
has "gate: the warning is printed" 'WARNING'
FS_LOCK_STRICT=1 bash "$gate" "$tmp/g" >"$tmp/o" 2>&1 && bad "gate: FS_LOCK_STRICT=1 must be red" || ok "gate: FS_LOCK_STRICT=1 is red"
printf 'strict = true\n' > "$tmp/g/.fs-lock-sources.toml"
bash "$gate" "$tmp/g" >"$tmp/o" 2>&1 && bad "gate: repo file strict = true must be red" || ok "gate: repo file strict = true is red"
cp "$fx/green.lock" "$tmp/g/Cargo.lock"
bash "$gate" "$tmp/g" >"$tmp/o" 2>&1 && ok "gate: green lock is green with strict" || bad "gate: green lock must pass strict: $(cat "$tmp/o")"
printf 'not toml [[' > "$tmp/g/Cargo.lock"
bash "$gate" "$tmp/g" >"$tmp/o" 2>&1 && bad "gate: unreadable lock must be red" || ok "gate: unreadable lock is red"
[ "$fails" -eq 0 ] || { echo "$fails failed"; exit 1; }
echo "all ok"
