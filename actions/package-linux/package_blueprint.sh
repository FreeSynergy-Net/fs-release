#!/usr/bin/env bash
# Pattern: Strategy - the Linux packager for kind module and data: ONE RPM made by `fs-blueprint package module|rpm` (INST-4b), no tar.gz/deb/AppImage.
# Sourced by package.sh after lib.sh, with $PLAN, $ARCH, $TRIPLE and `fs-blueprint` on PATH (the workflow installs it from its own commit).
set -euo pipefail

# plain_file FILE -> success only for a regular file that is no link (K7/K8 of the review of INST-4a r2: the path comes from OUR build output, never from the module repo).
plain_file() { [[ -f "$1" && ! -L "$1" ]]; }

# elf_machine_ok FILE ARCH -> the built service is an ELF of the CPU it is packaged for (e_machine 0x3e x86_64, 0xb7 aarch64; little endian).
elf_machine_ok() {
  local want m
  case "$2" in x86_64) want="3e00" ;; aarch64) want="b700" ;; *) return 1 ;; esac
  [[ "$(head -c 4 "$1" | od -An -tx1 | tr -d ' \n')" == "7f454c46" ]] || return 1
  m=$(dd if="$1" bs=1 skip=18 count=2 2>/dev/null | od -An -tx1 | tr -d ' \n')
  [[ "$m" == "$want" ]]
}

blueprint_package() {
  local kind name version v bin dist out
  kind=$(plan_get kind); name=$(plan_get name); version=$(plan_get version)
  v=${version/-/\~}   # RPM forbids '-' in versions (same rule as rpm_package)
  dist=$(dist_dir)
  command -v fs-blueprint >/dev/null || die "fs-blueprint is not installed (the workflow builds it from its own commit)"
  case "$kind" in
    data)
      group "package rpm (data, noarch)"
      fs-blueprint package rpm "$(plan_get blueprint)" --out "$dist"
      endgroup ;;
    module)
      local b; b=$(plan_get binaries)
      group "build $TRIPLE"
      group_list; cargo_build "$TRIPLE"; strip "$(bin_path "$TRIPLE" "$b")"
      endgroup
      bin=$(bin_path "$TRIPLE" "$b")
      plain_file "$bin" || die "the built service $bin is not a plain file (link or missing)"
      elf_machine_ok "$bin" "$ARCH" || die "the built service $bin is no ELF for $ARCH"
      group "package module"
      fs-blueprint package module . --name "$name" --pkg-version "$v" --binary "$bin" --arch "$ARCH" --out "$dist"
      endgroup ;;
    *) die "package_blueprint: kind $kind" ;;
  esac
  out=$(find "$dist" -maxdepth 1 -name '*.rpm' | wc -l)
  [[ "$out" -ge 1 ]] || die "no RPM in $dist after fs-blueprint"
  # the version of a data RPM is the blueprint's: the tag must name the same one, else a wrong tag would publish an RPM of another version
  [[ -n "$(find "$dist" -maxdepth 1 -name "*-$v-*.rpm" -print -quit)" ]] || die "no RPM of version $v in $dist (the tag and the version of the blueprint differ)"
  dist_normalise "$dist"
  ls -l "$dist"
}
