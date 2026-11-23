#!/usr/bin/env bash
# Pattern: none (test).
# Self test of the gate `licence` (scripts/licence-gate.sh, step `licence` of scripts/gates.sh). Part A (stub cargo-deny, no network): "not run is never green" (no binary, wrong version, no policy, no
# manifest, bad pin = rc 3 with the line `licence gate NOT RUN`), the exact cargo-deny call, the rc of cargo-deny is passed on, the step is in gates.sh. Part B (the REAL cargo-deny of
# scripts/cargo-deny.pin, downloaded and sha256-checked by the gate itself): a workspace with a path dependency under AGPL-3.0-only is RED, one under `MIT OR Apache-2.0` is green
# (tests/fixtures/licence-gate/). Mutants (the allow list gets AGPL; the step is removed from gates.sh; the call loses `licenses`) must each turn a check red.
#   bash scripts/test_licence_gate.sh        (FS_CARGO_DENY=<binary of the pinned version> skips the download)
set -uo pipefail
here="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
repo="$(cd "$here/.." && pwd)"
mkdir -p "$repo/.test-tmp"; tmp="$(TMPDIR="${TEST_TMPDIR:-$repo/.test-tmp}" mktemp -d)"  # never /tmp (RAM, rule [PLATTE])
trap '[ -n "${KEEP:-}" ] || { rm -rf "$tmp"; rmdir "$repo/.test-tmp" 2>/dev/null; }' EXIT
fails=0
ok() { echo "ok   $1"; }
bad() { echo "FAIL $1"; fails=$((fails + 1)); }
chk() { local name="$1"; shift; if "$@"; then ok "$name"; else bad "$name"; fi; }
version="$(sed -n 's/^CARGO_DENY_VERSION=//p' "$here/cargo-deny.pin")"
# a mechanism folder: scripts + policy (+ a repo to check)
mkmech() { rm -rf "$1"; mkdir -p "$1/configs"; cp -r "$here" "$1/scripts"; rm -rf "$1/scripts/__pycache__"; cp "$repo/configs/deny.toml" "$1/configs/"; }
fixture() { rm -rf "$2"; cp -r "$repo/tests/fixtures/licence-gate/$1" "$2"; }
# --- Part A: the stub ----------------------------------------------------------------------------------------------------------------------------
mkdir -p "$tmp/bin"
printf '#!/usr/bin/env bash\n[ "$1" = --version ] && { echo "cargo-deny %s"; exit 0; }\necho "$@" >"%s/args.txt"\nexit "${STUB_RC:-0}"\n' "$version" "$tmp" >"$tmp/bin/cargo-deny"
printf '#!/usr/bin/env bash\n[ "$1" = --version ] && { echo "cargo-deny 0.0.1"; exit 0; }\nexit 0\n' >"$tmp/bin/cargo-deny-old"
chmod +x "$tmp"/bin/*
mkmech "$tmp/m"; fixture ok "$tmp/ok"
g() { env "$@" bash "$tmp/m/scripts/licence-gate.sh" "$tmp/ok" >"$tmp/o.txt" 2>&1; }
notrun() { [ "$rc" -eq 3 ] && grep -q '^licence gate NOT RUN: ' "$tmp/o.txt"; }
g FS_CARGO_DENY="$tmp/bin/does-not-exist"; rc=$?; chk "no binary: rc 3, NOT RUN (never green)" notrun
g FS_CARGO_DENY="$tmp/bin/cargo-deny-old"; rc=$?; chk "wrong version: rc 3, NOT RUN" notrun
rm "$tmp/m/configs/deny.toml"; g FS_CARGO_DENY="$tmp/bin/cargo-deny"; rc=$?; chk "no policy file: rc 3, NOT RUN" notrun
cp "$repo/configs/deny.toml" "$tmp/m/configs/"; rm "$tmp/ok/Cargo.toml"; g FS_CARGO_DENY="$tmp/bin/cargo-deny"; rc=$?; chk "no Cargo.toml: rc 3, NOT RUN" notrun
fixture ok "$tmp/ok"; sed -i 's/^CARGO_DENY_SHA256=.*/CARGO_DENY_SHA256=zz/' "$tmp/m/scripts/cargo-deny.pin"; g FS_CARGO_DENY="$tmp/bin/cargo-deny"; rc=$?; chk "bad pin file: rc 3, NOT RUN" notrun
cp "$here/cargo-deny.pin" "$tmp/m/scripts/"
g FS_CARGO_DENY="$tmp/bin/cargo-deny"; rc=$?
chk "stub green: rc 0" [ "$rc" -eq 0 ]
chk "the call is exactly: --manifest-path <root>/Cargo.toml --workspace --locked --config <mech>/configs/deny.toml check licenses bans sources" \
  grep -qxF -- "--manifest-path $tmp/ok/Cargo.toml --workspace --locked --config $tmp/m/scripts/../configs/deny.toml check licenses bans sources" "$tmp/args.txt"
g FS_CARGO_DENY="$tmp/bin/cargo-deny" STUB_RC=1; rc=$?; chk "a finding of cargo-deny (rc 1) is passed on, not turned green" [ "$rc" -eq 1 ]
chk "gates.sh runs the step licence (own step, severity error)" grep -q '^  step licence error bash "\$here/licence-gate.sh" "\$root"$' "$here/gates.sh"
chk "a repo deny.toml is not read: the call names only the shared policy" bash -c "grep -c -- '--config' '$tmp/args.txt' | grep -qx 1"
mut_a="$tmp/mut-step"; mkdir -p "$mut_a"; grep -v 'step licence error' "$here/gates.sh" >"$mut_a/gates.sh"
if grep -q '^  step licence error bash "\$here/licence-gate.sh" "\$root"$' "$mut_a/gates.sh"; then bad "mutant: step removed from gates.sh survived"; else ok "mutant: step removed from gates.sh is seen by the check above"; fi
# --- Part B: the real cargo-deny -----------------------------------------------------------------------------------------------------------------
real() { # <mech> <fixture name>: the gate on a fixture copy; output in $tmp/r.txt
  fixture "$2" "$tmp/fx"; env ${FS_CARGO_DENY:+FS_CARGO_DENY="$FS_CARGO_DENY"} bash "$1/scripts/licence-gate.sh" "$tmp/fx" >"$tmp/r.txt" 2>&1
}
agpl_red() { real "$1" agpl; local r=$?; agpl_rc=$r; [ "$r" -eq 1 ] && grep -q 'AGPL-3.0-only' "$tmp/r.txt"; }
ok_green() { real "$1" ok; [ $? -eq 0 ]; }
mkmech "$tmp/real"
chk "REAL cargo-deny $version: dependency under AGPL-3.0-only is RED (rc 1, the licence is named)" agpl_red "$tmp/real"
cp "$tmp/r.txt" "$tmp/agpl.txt"; agpl_keep=$agpl_rc
chk "REAL cargo-deny $version: dependency under MIT OR Apache-2.0 is green (rc 0)" ok_green "$tmp/real"
cp "$tmp/r.txt" "$tmp/ok.txt"
# mutant: AGPL on the allow list -> the check agpl_red must turn red (the test sees the loosened policy)
mkmech "$tmp/mut-allow"; sed -i 's/^    "MIT",$/    "MIT",\n    "AGPL-3.0-only",/' "$tmp/mut-allow/configs/deny.toml"
if agpl_red "$tmp/mut-allow"; then bad "mutant: allow list with AGPL survived"; else ok "mutant: allow list with AGPL is caught (the AGPL fixture is no longer red)"; fi
# mutant: the call without `licenses` -> the AGPL fixture is no longer red
mkmech "$tmp/mut-call"; sed -i 's/check licenses bans sources/check bans sources/' "$tmp/mut-call/scripts/licence-gate.sh"
if agpl_red "$tmp/mut-call"; then bad "mutant: call without licenses survived"; else ok "mutant: call without licenses is caught"; fi
echo "---"; echo "AGPL fixture, output of the gate (rc $agpl_keep):"; head -n 60 "$tmp/agpl.txt"
if [ "$fails" -ne 0 ]; then echo "test_licence_gate: $fails FAILED"; exit 1; fi
echo "test_licence_gate: all ok"
