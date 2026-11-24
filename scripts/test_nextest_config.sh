#!/usr/bin/env bash
# Pattern: none. Self test of configs/nextest.toml with the REAL cargo-nextest: the file must be accepted in a workspace that has none of the names it mentions (a binary(...) filter
# that matches nothing made nextest refuse the whole file in fs-components run 45326725055), and a test named like the convention lands in its group. Needs cargo-nextest; run in a build slot.
set -uo pipefail
here="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
t="$(mktemp -d "${TMPDIR:-/tmp}/nextest-config.XXXXXX")"; trap 'rm -rf "$t"' EXIT
mkdir -p "$t/src" "$t/tests"; printf '[package]\nname = "mini"\nversion = "0.1.0"\nedition = "2021"\n' > "$t/Cargo.toml"; : > "$t/src/lib.rs"
printf '#[test]\nfn plain() {}\n#[test]\nfn a_gpu_draw() {}\n' > "$t/tests/one.rs"
cd "$t" || exit 2
out="$(CARGO_TARGET_DIR="$t/target" cargo nextest show-config test-groups --tool-config-file "freesynergy:$here/../configs/nextest.toml" 2>&1)"; rc=$?
[ "$rc" -eq 0 ] || { echo "FAIL nextest refuses configs/nextest.toml (rc $rc)"; echo "$out"; exit 1; }
grep -q 'a_gpu_draw' <<<"$out" || { echo "FAIL a test named *_gpu is not in the gpu group"; echo "$out"; exit 1; }
echo "ok   configs/nextest.toml is accepted by cargo-nextest and the name convention groups a test"
