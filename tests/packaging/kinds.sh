#!/usr/bin/env bash
# Pattern: none (test). INST-4b: release kinds `module` (Linux RPM fs-module-<name> by `fs-blueprint package module`) and `data` (noarch RPM by `package rpm`).
# plan.py and package-linux/package.sh run for real; cargo and fs-blueprint are recording stubs (the real command is tested in crates/fs-blueprint).
TK="$T/kinds"; X=x86_64-unknown-linux-gnu
make_pkg "$TK/mod" fs-modules-planning 0.1.1 planning-service
: > "$TK/mod/LICENSE-MIT"
cat > "$T/bin/fs-blueprint" <<'STUB'
#!/usr/bin/env bash
echo "fs-blueprint $*" >> "$STUB_LOG"
out=""; ver=""; arch=noarch; name=""; bin=""
a=("$@")
for ((i = 0; i < ${#a[@]}; i++)); do
  case "${a[i]}" in --out) out=${a[i+1]} ;; --pkg-version) ver=${a[i+1]} ;; --arch) arch=${a[i+1]} ;; --name) name=${a[i+1]} ;; --binary) bin=${a[i+1]} ;; esac
done
if [[ "$2" == module ]]; then : > "$out/fs-module-$name-${ver/\~/-}-1.$arch.rpm"; else : > "$out/fs-tasks-${STUB_BPVER:-0.0.4}-1.noarch.rpm"; fi
STUB
chmod +x "$T/bin/fs-blueprint"

# --- plan.py
plan=$(make_plan "$TK/mod" BINARIES=planning-service KIND=module NAME=planning)
check_in "kinds: module plan has the kind" "$(jq -c .kind <<<"$plan")" '"module"'
check_in "kinds: module builds Linux RPM only" "$(jq -c '[.linux_formats,.has_windows,.has_macos,.has_arch]' <<<"$plan")" '["rpm",false,false,false]'
check_in "kinds: module keeps both Linux CPUs" "$(jq -c '[.linux_matrix[].arch]' <<<"$plan")" '["x86_64","aarch64"]'
for bad in "KIND=module BINARIES=planning-service NAME=fs-planning" "KIND=module BINARIES=planning-service" "KIND=module BINARIES=a,b NAME=planning" "KIND=module NAME=planning" "KIND=module BINARIES=planning-service NAME=Pla/ning"; do
  # shellcheck disable=SC2086
  if make_plan "$TK/mod" $bad >/dev/null 2>&1; then fail "kinds: plan must fail for $bad"; else pass "kinds: plan fails for $bad"; fi
done
mkdir -p "$T/none"; dplan=$(make_plan "$T/none" KIND=data NAME=fs-tasks)
check_in "kinds: data plan is noarch (one Linux job)" "$(jq -c '[(.linux_matrix|length), .has_windows, .has_macos, .has_arch, .linux_formats]' <<<"$dplan")" '[1,false,false,false,"rpm"]'
if make_plan "$T/none" KIND=data NAME=fs-tasks BINARIES=x >/dev/null 2>&1; then fail "kinds: data with binaries must fail"; else pass "kinds: data with binaries fails"; fi
if make_plan "$T/none" KIND=bogus NAME=x BINARIES=x >/dev/null 2>&1; then fail "kinds: unknown kind must fail"; else pass "kinds: unknown kind fails"; fi

# --- package.sh
run_kind() { # run_kind DIR PLAN ARCH -> log + dist listing (rc in $T/krc: it runs in a $(...) subshell)
  local d=$1 plan=$2 arch=${3:-x86_64} triple=${4:-x86_64-unknown-linux-gnu}
  rm -rf "$d/dist" "$d/target" "${T:?}/cargo-target"; mkdir -p "$T/stage"; : > "$T/log"; KRC=0
  (cd "$d" && PATH="$T/bin:$PATH" PLAN="$plan" ARCH=$arch TRIPLE=$triple RUNNER_TEMP="$T" GITHUB_WORKSPACE="$d" STUB_LOG="$T/log" \
    bash "$ROOT/actions/package-linux/package.sh" >/dev/null 2>"$T/err") || KRC=$?
  echo "$KRC" > "$T/krc"
  { sed "s#$d#@R@#g; s#$T/cargo-target#@R@/target#g" "$T/log"; echo "--- dist"; ls "$d/dist" 2>/dev/null || true; }
}
out=$(STUB_ELF=3e run_kind "$TK/mod" "$plan"); KRC=$(cat "$T/krc")
[[ $KRC -eq 0 ]] && pass "kinds: module packaging succeeds" || fail "kinds: module packaging rc=$KRC"
check_has "kinds: module builds the service" "$out" "cargo build --release --locked --target $X --bin planning-service"
check_has "kinds: module RPM from the build output (target/<triple>/release), not from the repo" "$out" "fs-blueprint package module . --name planning --pkg-version 0.1.1 --binary @R@/target/$X/release/planning-service --arch x86_64 --out @R@/dist"
check_has "kinds: module RPM is the only artefact" "$out" "fs-module-planning-0.1.1-1.x86_64.rpm"
check_lacks "kinds: module makes no tar.gz" "$out" ".tar.gz"
check_lacks "kinds: module makes no deb" "$out" "cargo deb"
# the built service is checked: wrong CPU, no ELF and a link are refused BEFORE fs-blueprint runs
out=$(STUB_ELF=b7 run_kind "$TK/mod" "$plan"); KRC=$(cat "$T/krc"); [[ $KRC -ne 0 ]] && ! grep -q "^fs-blueprint" <<<"$out" && pass "kinds: ELF of another CPU is refused" || fail "kinds: wrong e_machine must fail (rc=$KRC)"
out=$(run_kind "$TK/mod" "$plan"); KRC=$(cat "$T/krc"); [[ $KRC -ne 0 ]] && ! grep -q "^fs-blueprint" <<<"$out" && pass "kinds: a non-ELF service is refused" || fail "kinds: non-ELF must fail (rc=$KRC)"
out=$(STUB_ELF=b7 run_kind "$TK/mod" "$(make_plan "$TK/mod" BINARIES=planning-service KIND=module NAME=planning)" aarch64 aarch64-unknown-linux-gnu); KRC=$(cat "$T/krc")
[[ $KRC -eq 0 ]] && check_has "kinds: aarch64 module RPM" "$out" "fs-module-planning-0.1.1-1.aarch64.rpm" || fail "kinds: aarch64 module rc=$KRC"
# a link as service (K7): cargo stub leaves a link where the build output should be
mkdir -p "$TK/link-bin"; { printf '\x7fELF'; head -c 14 /dev/zero; printf '\x3e\0'; } > "$TK/real-elf"  # a VALID ELF behind the link: only the link check can refuse it
printf '#!/usr/bin/env bash\nif [[ "$1" == metadata ]]; then exec %s "$@"; fi\nif [[ "$1" == build ]]; then while [[ $# -gt 0 ]]; do case "$1" in --bin) b=$2 ;; esac; shift; done; mkdir -p "target/x86_64-unknown-linux-gnu/release"; ln -s %s "target/x86_64-unknown-linux-gnu/release/$b"; fi\n' "$REAL_CARGO" "$TK/real-elf" > "$TK/link-bin/cargo"
chmod +x "$TK/link-bin/cargo"
rm -rf "$TK/mod/dist" "$TK/mod/target"; : > "$T/log"; rc=0
(cd "$TK/mod" && unset CARGO_TARGET_DIR && PATH="$TK/link-bin:$T/bin:$PATH" PLAN="$plan" ARCH=x86_64 TRIPLE=$X RUNNER_TEMP="$T" GITHUB_WORKSPACE="$TK/mod" STUB_LOG="$T/log" \
  bash "$ROOT/actions/package-linux/package.sh" >/dev/null 2>"$T/err") || rc=$?
[[ $rc -ne 0 ]] && ! grep -q "^fs-blueprint" "$T/log" && pass "kinds: a link as built service is refused" || fail "kinds: link must be refused (rc=$rc)"

# data: one RPM, no cargo at all
mkdir -p "$T/none"; : > "$T/none/scaffold.toml"
out=$(STUB_BPVER=0.1.1 run_kind "$T/none" "$dplan"); KRC=$(cat "$T/krc")
[[ $KRC -eq 0 ]] && pass "kinds: data packaging succeeds" || fail "kinds: data packaging rc=$KRC"
check_has "kinds: data uses package rpm" "$out" "fs-blueprint package rpm scaffold.toml --out @R@/dist"
check_lacks "kinds: data never calls cargo" "$out" "cargo "
check_has "kinds: data RPM in dist" "$out" "fs-tasks-0.1.1-1.noarch.rpm"
out=$(STUB_BPVER=9.9.9 run_kind "$T/none" "$dplan"); KRC=$(cat "$T/krc"); [[ $KRC -ne 0 ]] && pass "kinds: data RPM of another version than the tag fails" || fail "kinds: version mismatch must fail"

# K-T1: the ELF magic is checked on its own: a file that is NOT an ELF but has the e_machine bytes 3e 00 at byte 18 must be refused
out=$(STUB_MAGIC=XYZ STUB_ELF=3e run_kind "$TK/mod" "$plan"); KRC=$(cat "$T/krc")
[[ $KRC -ne 0 ]] && ! grep -q "^fs-blueprint" <<<"$out" && pass "kinds: non-ELF with the x86_64 e_machine bytes is refused (ELF magic)" || fail "kinds: non-ELF with 3e00 must fail (rc=$KRC)"

# K-W1 / MUSS-1: what the plan says out loud (decided in the plan job, before the signing approval)
plan_out() { (cd "$1" && shift && env TAG=v0.1.1 LINUX_TARGETS=x86_64 WINDOWS_TARGETS=x86_64 MACOS=true "$@" GITHUB_OUTPUT="$T/out" python3 -I "$ROOT/actions/plan/plan.py" 2>&1); }
check_in "kinds: module says it has no Windows build yet (WIN-b)" "$(plan_out "$TK/mod" BINARIES=planning-service KIND=module NAME=planning)" "::warning::kind module: no Windows package yet"
check_in "kinds: data says it has no SBOM, before the approval" "$(plan_out "$T/none" KIND=data NAME=fs-tasks)" "::notice::kind data: no SBOM"
# the finalize job: the SBOM step is skipped for data, and a release without *.spdx.json says so in its notes
check_in "kinds: finalize skips the SBOM action for data" "$(grep -B1 'actions/sbom@' "$ROOT/.github/workflows/_release-finalize.yml")" "fromJSON(inputs.plan).kind != 'data'"
check_in "kinds: draft notes say 'SBOM: none' without an spdx file" "$(grep -c 'SBOM: none for this release' "$ROOT/actions/draft-release/draft.sh")" "1"
