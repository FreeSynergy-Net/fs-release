#!/usr/bin/env bash
# Pattern: none. Gate `test` of scripts/gates.sh: `cargo nextest run` (all test binaries in parallel, each test in its own process) with the ONE configuration configs/nextest.toml and the cargo-nextest
# pinned in scripts/nextest.pin. Doc tests are NOT run by nextest: gates.sh runs them as its own step `doctest` (`cargo test --doc`). "Not run is never green": no nextest / wrong version = exit 64 with
# its own line `test gate NOT RUN: ...`; a red test keeps the exit code of nextest. Arguments are passed on to `cargo nextest run` (package selection, -E filter, ...).
#   bash scripts/test-gate.sh [--profile <name>] <cargo nextest run arguments>
# In CI the version must be exactly the pin (actions/gates installs it); on a machine of its own the pin is the lowest version (a newer one is said, never silent).
set -uo pipefail
here="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
notrun() { echo "test gate NOT RUN: $*" >&2; exit 64; }
config="$here/../configs/nextest.toml"
# --doc <cargo test arguments>: the doc tests (nextest does not run them) with plain `cargo test --doc`; a selection without any library target has no doc tests (cargo says so with an error: that is green here)
if [ "${1:-}" = --doc ]; then
  shift
  out="$(cargo test --doc "$@" 2>&1)"; rc=$?
  printf "%s\n" "$out"
  if [ "$rc" -ne 0 ] && grep -q "^error: no library targets found" <<<"$out"; then echo "test gate: no library target in the selection, no doc tests"; exit 0; fi
  exit "$rc"
fi
# --profile <name> (first arguments; from the input `profile` of check.yml via FS_GATES_PROFILE): a named nextest profile of the CALLER repo (`.config/nextest.toml`, e.g. quick, release). The name is checked against a fixed alphabet
# before it is used (anything else = rc 2, never put on a command line). A repo without that profile does not fail: the default profile runs and the log says so (so repos without profiles stay green).
profile_args=()
if [ "${1:-}" = --profile ]; then
  [ $# -ge 2 ] || { echo "test gate: --profile needs a name" >&2; exit 2; }
  profile="$2"; shift 2
  [[ "$profile" =~ ^[a-z][a-z0-9_-]{0,31}$ ]] || { echo "test gate REFUSED: profile name must match [a-z][a-z0-9_-]{0,31} (got: ${profile:0:40})" >&2; exit 2; }
  if [ "$profile" = default ] || grep -qE "^[[:space:]]*\[profile\.${profile}(\.[A-Za-z0-9_-]+)?\][[:space:]]*(#.*)?\$" .config/nextest.toml 2>/dev/null; then
    profile_args=(--profile "$profile"); echo "test gate: profile $profile"
  else
    echo "test gate: profile $profile not defined here (no [profile.$profile] in .config/nextest.toml), running default"
  fi
fi
[ -f "$config" ] || notrun "configuration $config is missing"
version="$(sed -n 's/^NEXTEST_VERSION=//p' "$here/nextest.pin" 2>/dev/null | head -n 1)"
[[ "$version" =~ ^[0-9]+\.[0-9]+\.[0-9]+$ ]] || notrun "scripts/nextest.pin has no exact version"
got="$(cargo nextest --version 2>&1 | sed -n 's/^cargo-nextest \([0-9][0-9.]*\).*/\1/p' | head -n 1)"
[ -n "$got" ] || notrun "cargo-nextest is not installed (cargo binstall cargo-nextest@$version); doc tests and the rest of the gates are not a replacement"
if [ "$got" != "$version" ]; then
  if [ -n "${GITHUB_ACTIONS:-}" ] || [ "$(printf '%s\n%s\n' "$version" "$got" | sort -V | head -n 1)" != "$version" ]; then
    notrun "found cargo-nextest $got, the pin is $version (in CI exactly, elsewhere at least)"
  fi
  echo "test gate: cargo-nextest $got is newer than the pin $version (accepted outside CI)"
fi
echo "test gate: cargo-nextest $got, configuration $config"
cargo nextest run ${profile_args[@]+"${profile_args[@]}"} --tool-config-file "freesynergy:$config" --no-tests=pass --no-fail-fast "$@"
