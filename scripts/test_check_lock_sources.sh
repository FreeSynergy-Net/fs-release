#!/usr/bin/env bash
# Self test of scripts/check_lock_sources.py and scripts/gates-lock.sh (fixtures: tests/fixtures/lock-sources/): one source is green, two sources of a
# library are red with a message that names both sources and who pulls them, V1+ (fs-types only from the registry `freesynergy` = index URL FreeSynergy-Net/fs-crates; every git source is red; registry sources are normalised, two registry versions are red with the fix `cargo update -p`) is red only with
# the switch, a mutant (library list emptied) lets the red lock through = the test would be red, the gate warns first and fails with `strict`.
#   bash scripts/test_check_lock_sources.sh
set -uo pipefail
here="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
tool="$here/check_lock_sources.py"; gate="$here/gates-lock.sh"
fx="$here/../tests/fixtures/lock-sources"
mkdir -p "$here/../.test-tmp"; tmp="$(TMPDIR="${TEST_TMPDIR:-$here/../.test-tmp}" mktemp -d)"  # never /tmp (RAM, rule [PLATTE])
trap 'rm -rf "$tmp"; rmdir "$here/../.test-tmp" 2>/dev/null' EXIT
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
has "V1+ message names the allowed index URL" 'allows only https://github.com/FreeSynergy-Net/fs-crates or'
FS_LOCK_ONLY_FROM=1 python3 -I "$tool" "$fx/red-v1plus.lock" >"$tmp/o" 2>&1 && bad "env switch FS_LOCK_ONLY_FROM=1 must be red" || ok "env switch FS_LOCK_ONLY_FROM=1 is red"
expect "registry fs-types is green with the V1+ switch" 0 --only-from "$fx/green-registry.lock"
expect "registry fs-types is green without the switch" 0 "$fx/green-registry.lock"
# K-5 PRODUCT RULE (ENTSCHEID-kaskade K-5, MUST): the SHIPPED scripts/lock-sources.toml must hold exactly ONE entry for fs-types (the index URL) after the wave. Until then the second entry
# (git FreeSynergy-Net/fs-types) is a documented EXCEPTION (lift and lock-gleich of the running cascade check against it). K-5 deletes the second entry AND sets K5_DONE=1 here; with K5_DONE=1 two entries are RED.
K5_DONE=0
shipped_entries() { grep '^fs-types' "$1" | grep -o '"[^"]*"' | wc -l; }
shipped_rule() { # <config> <k5 done 0|1>: prints ok/FAIL for the shipped state
  local cfg="$1" done="$2" n r
  n=$(shipped_entries "$cfg"); r=$(rc_of --only-from --config "$cfg" "$fx/green.lock")
  if [ "$n" = 1 ] && [ "$r" = 1 ]; then ok "shipped config: ONE only_from entry, git fs-types of FreeSynergy-Net/fs-types is red (K-5 state)"
  elif [ "$n" != 1 ] && [ "$done" = 0 ] && [ "$r" = 0 ]; then ok "shipped config: $n entries, git fs-types still green: DOCUMENTED EXCEPTION until K-5 (K-5 must delete the second entry and set K5_DONE=1)"
  else bad "shipped config: $n entries, git fs-types rc=$r, K5_DONE=$done (after K-5 exactly one entry and a red git fs-types are required)"; fi
}
shipped_rule "$here/lock-sources.toml" "$K5_DONE"
# the rule itself has teeth: a config with two entries is FAIL once K-5 is done, one entry is not (counter-proofs on copies, not on the shipped file)
cp "$here/lock-sources.toml" "$tmp/two.toml"; sed -i 's#^fs-types = .*#fs-types = ["https://github.com/FreeSynergy-Net/fs-crates", "https://github.com/FreeSynergy-Net/fs-types"]#; s#^only_from_enabled = .*#only_from_enabled = true#' "$tmp/two.toml"
fails_before=$fails; shipped_rule "$tmp/two.toml" 1 >"$tmp/o" 2>&1
if [ "$fails" -gt "$fails_before" ]; then fails=$fails_before; ok "counter-proof: two entries with K5_DONE=1 is RED"; else bad "counter-proof: two entries with K5_DONE=1 must be red"; fi
sed 's#^fs-types = .*#fs-types = "https://github.com/FreeSynergy-Net/fs-crates"#' "$tmp/two.toml" > "$tmp/one.toml"
fails_before=$fails; shipped_rule "$tmp/one.toml" 1 >"$tmp/o" 2>&1
[ "$fails" -eq "$fails_before" ] && ok "counter-proof: one entry with K5_DONE=1 is green" || bad "counter-proof: one entry with K5_DONE=1 must be green"
printf 'libraries = ["fs-types"]\nonly_from_enabled = true\n[only_from]\nfs-types = "https://github.com/FreeSynergy-Net/fs-crates"\n' > "$tmp/after-wave.toml"
expect "AFTER THE WAVE (config = index URL only, K-5): git fs-types of FreeSynergy-Net/fs-types is red" 1 --config "$tmp/after-wave.toml" "$fx/green.lock"
has "after the wave: the message names the index URL" 'allows only https://github.com/FreeSynergy-Net/fs-crates'
expect "after the wave: the registry fs-types is green" 0 --config "$tmp/after-wave.toml" "$fx/green-registry.lock"
expect "after the wave: the old org name and a foreign org stay red" 1 --config "$tmp/after-wave.toml" "$fx/red-foreign-org.lock"
expect "two registry versions of fs-types are red" 1 "$fx/red-registry-two.lock"
has "registry doubles: message names both sources" 'registry+https://github.com/FreeSynergy-Net/fs-crates'
has "registry doubles: message names a version" '0.0.6'
has "registry doubles: fix is cargo update -p fs-types" '`cargo update -p fs-types` (the dependents ask for a range'
expect "git doubles keep the old fix text" 1 "$fx/red-two-sources.lock"
has "git doubles: pin every dependent" 'pin every dependent to the same tag'
sed 's#registry+https://github.com/FreeSynergy-Net/fs-crates#registry+HTTPS://GitHub.com/FreeSynergy-Net/FS-Crates.git/#' "$fx/green-registry.lock" > "$tmp/case.lock"
expect "registry source is normalised (case, .git, slash) before the comparison" 0 --only-from "$tmp/case.lock"
sed 's#registry+https://github.com/FreeSynergy-Net/fs-crates#sparse+https://github.com/FreeSynergy-Net/fs-crates#' "$fx/green-registry.lock" > "$tmp/case.lock"
expect "a sparse+ source of the same index URL is normalised too" 0 --only-from "$tmp/case.lock"
sed 's#registry+https://github.com/FreeSynergy-Net/fs-crates#registry+https://github.com/FreeSynergy-Net/other-crates#' "$fx/green-registry.lock" > "$tmp/case.lock"
expect "a registry of another index URL is red with the V1+ switch" 1 --only-from "$tmp/case.lock"
expect "V1+: fs-types from the OLD org name FreeSynergy/ (redirect) is the same source, green" 0 --only-from "$fx/red-old-org.lock"
sed 's#github.com/FreeSynergy/#github.com/FreeSynergyX/#' "$fx/red-old-org.lock" > "$tmp/case.lock"
expect "V1+: a look-alike org FreeSynergyX/ is red" 1 --only-from "$tmp/case.lock"
expect "V1+: fs-types from a foreign org is red" 1 --only-from "$fx/red-foreign-org.lock"
expect "V1+: fs-types from the registry of FreeSynergy-Net/ is green" 0 --only-from "$fx/green-registry.lock"
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
# mutant: norm without the org alias -> the old org name is red again, so the alias test above is real
sed 's#^    return re.sub(r"^(https.*#    return u#' "$tool" > "$tmp/mut_tool.py"
python3 -I "$tmp/mut_tool.py" --config "$here/lock-sources.toml" --only-from "$fx/red-old-org.lock" >/dev/null 2>&1 && bad "mutant (no org alias) must be red on the old org name" || ok "mutant (no org alias) is red on the old org name, so the alias test is real"
# mutant: repo_of without the registry normalisation (the old code) -> the normalisation test above would be red
sed 's#for kind in ("git+", "registry+", "sparse+"):#for kind in ("git+",):#' "$tool" > "$tmp/mut_tool.py"
sed 's#registry+https://github.com/FreeSynergy-Net/fs-crates#registry+HTTPS://GitHub.com/FreeSynergy-Net/FS-Crates.git/#' "$fx/green-registry.lock" > "$tmp/case.lock"
python3 -I "$tmp/mut_tool.py" --config "$here/lock-sources.toml" --only-from "$tmp/case.lock" >/dev/null 2>&1 && bad "mutant (repo_of without registry) must be red on the normalisation case" || ok "mutant (repo_of without registry normalisation) is red on the case, so the normalisation test above is real"
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
# condition 1: a repo file cannot change the library list or [only_from]; condition 2: a workspace member (no source) is no violation
mkdir -p "$tmp/c1"; cp "$fx/red-two-sources.lock" "$tmp/c1/Cargo.lock"
printf 'strict = true\nlibraries = []\n[only_from]\nfs-types = "https://github.com/x/y"\n' > "$tmp/c1/.fs-lock-sources.toml"
expect "repo file with libraries = [] does not switch the check off" 1 "$tmp/c1"
bash "$gate" "$tmp/c1" >"$tmp/o" 2>&1 && bad "gate: repo file libraries = [] with strict must stay red" || ok "gate: repo file libraries = [] with strict stays red"
cp "$fx/red-v1plus.lock" "$tmp/c1/Cargo.lock"
printf 'only_from_enabled = true\n[only_from]\nfs-types = "https://github.com/FreeSynergy/fs-interfaces"\n' > "$tmp/c1/.fs-lock-sources.toml"
expect "repo file cannot replace [only_from] (still red)" 1 "$tmp/c1"
printf 'version = 4\n\n[[package]]\nname = "fs-types"\nversion = "0.0.1"\n' > "$tmp/c1/Cargo.lock"; rm "$tmp/c1/.fs-lock-sources.toml"
expect "workspace member fs-types (no source) is no violation with --only-from" 0 --only-from "$tmp/c1"
mkdir -p "$tmp/e"; : > "$tmp/e/Cargo.lock"; expect "an empty Cargo.lock is green" 0 --only-from "$tmp/e"
[ "$fails" -eq 0 ] || { echo "$fails failed"; exit 1; }
echo "all ok"
