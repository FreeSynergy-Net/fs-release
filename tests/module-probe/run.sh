#!/usr/bin/env bash
# Pattern: none (test caller). The release path of kind `module` with the REAL plan.py, package.sh and fs-blueprint (no stubs; tests/packaging/kinds.sh has the stubs):
# the step "package module" runs and the RPM fs-module-probe is read back with rpm. Needs fs-blueprint, cargo, jq, rpm on PATH. Arch = the CPU of this machine.
set -euo pipefail
root=$(cd "$(dirname "$0")/../.." && pwd); here=$root/tests/module-probe
arch=$(uname -m); triple=$arch-unknown-linux-gnu
cd "$here"; rm -rf dist target; out=$(mktemp)
env TAG=v0.1.1 NAME=probe KIND=module BINARIES=probe-service LINUX_TARGETS="$arch" WINDOWS_TARGETS="" MACOS=false GITHUB_OUTPUT="$out" python3 -I "$root/actions/plan/plan.py" >/dev/null
PLAN=$(sed -n 's/^plan=//p' "$out")
[[ "$(jq -r .kind <<<"$PLAN")" == module ]]
PLAN=$PLAN ARCH=$arch TRIPLE=$triple RUNNER_TEMP=${RUNNER_TEMP:-$(mktemp -d)} GITHUB_WORKSPACE="$here" bash "$root/actions/package-linux/package.sh"
rpm_file=$(find dist -name 'fs-module-probe-*.rpm')
[[ -n "$rpm_file" && "$(echo "$rpm_file" | wc -l)" == 1 ]]
[[ "$(rpm -qp --qf '%{NAME} %{VERSION} %{ARCH}' "$rpm_file")" == "fs-module-probe 0.1.1 $arch" ]]
rpm -qlp "$rpm_file" | grep -q '^/usr/libexec/freesynergy/modules/'
echo "module-probe: ok ($rpm_file)"
