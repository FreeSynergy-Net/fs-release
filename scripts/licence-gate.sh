#!/usr/bin/env bash
# Pattern: none. Gate `licence` of scripts/gates.sh: `cargo-deny check licenses bans sources` with the ONE policy configs/deny.toml (no deny.toml per repo is read) and the cargo-deny
# pinned in scripts/cargo-deny.pin (version + sha256 of the release asset). "Not run is never green": no binary / wrong version / no config / no manifest = exit 64 with its own line
# `licence gate NOT RUN: ...`; a finding of cargo-deny keeps its exit code (1). Advisories are not part of the gate (moving database; see actions/licence-check).
#   bash scripts/licence-gate.sh <repo root>
# FS_CARGO_DENY=<path>: use that binary (still must report the pinned version). Otherwise the pinned asset is downloaded (linux x86_64), its sha256 is checked EVERY run and it is unpacked fresh.
set -uo pipefail
here="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
notrun() { echo "licence gate NOT RUN: $*" >&2; exit 64; }
[ $# -eq 1 ] && [ -d "$1" ] || notrun "usage: licence-gate.sh <repo root>"
root="$(cd "$1" && pwd)"
config="$here/../configs/deny.toml"
[ -f "$config" ] || notrun "policy $config is missing"
[ -f "$root/Cargo.toml" ] || notrun "no $root/Cargo.toml"
pin() { sed -n "s/^$1=//p" "$here/cargo-deny.pin" 2>/dev/null | head -n 1; }
version="$(pin CARGO_DENY_VERSION)"; sha="$(pin CARGO_DENY_SHA256)"
[[ "$version" =~ ^[0-9]+\.[0-9]+\.[0-9]+$ && "$sha" =~ ^[0-9a-f]{64}$ ]] || notrun "scripts/cargo-deny.pin has no exact version and sha256"
bin="${FS_CARGO_DENY:-}"
if [ -z "$bin" ]; then
  [ "$(uname -s)-$(uname -m)" = Linux-x86_64 ] || notrun "no pinned binary for $(uname -s)-$(uname -m): set FS_CARGO_DENY to a cargo-deny $version"
  asset="cargo-deny-$version-x86_64-unknown-linux-musl"
  cache="${FS_TOOLS_DIR:-${RUNNER_TEMP:-${XDG_CACHE_HOME:-$HOME/.cache}}/fs-release-tools}"
  mkdir -p "$cache" || notrun "cannot create $cache"
  if ! echo "$sha  $cache/$asset.tar.gz" | sha256sum -c --status - 2>/dev/null; then
    rm -f "$cache/$asset.tar.gz"
    timeout 120 curl -fsSL --retry 3 -o "$cache/$asset.tar.gz" "https://github.com/EmbarkStudios/cargo-deny/releases/download/$version/$asset.tar.gz" || notrun "download of cargo-deny $version failed"
    echo "$sha  $cache/$asset.tar.gz" | sha256sum -c --status - || { rm -f "$cache/$asset.tar.gz"; notrun "sha256 of the cargo-deny download differs from scripts/cargo-deny.pin"; }
  fi
  rm -rf "${cache:?}/${asset:?}"; tar -xzf "$cache/$asset.tar.gz" -C "$cache" || notrun "cannot unpack cargo-deny"
  bin="$cache/$asset/cargo-deny"
fi
[ -x "$bin" ] || notrun "$bin is not an executable file"
got="$("$bin" --version 2>&1)" || notrun "$bin --version failed"
[ "$got" = "cargo-deny $version" ] || notrun "found '$got', the pin is cargo-deny $version"
echo "licence gate: $got, policy $config, workspace $root"
"$bin" --manifest-path "$root/Cargo.toml" --workspace --locked --config "$config" check licenses bans sources
