#!/usr/bin/env bash
# Pattern: Strategy-by-presence - every signing step runs only when GPG_PRIVATE_KEY is set.
# 1. sign .deb (dpkg-sig) and .rpm (rpmsign), 2. export the public key,
# 3. write SHA256SUMS.txt (by the caller step), 4. detached ASCII-armoured .asc for every file.
# Key ring and passphrase live in a private temp dir/file that a trap removes at the end of THIS call (the action calls
# it once per mode, each call imports the key again). Test: tests/packaging/sign.sh.
# Usage: sign.sh packages|detached DIR
set -euo pipefail
mode=$1 dir=$2

# The key may also come as a FILE (GPG_PRIVATE_KEY_FILE, optional GPG_PASSPHRASE_FILE): then no environment variable of any process holds it (a child could read
# /proc/<pid>/environ of its parent); only the one `gpg --import` reads it, from stdin. The site flow (scripts/site-signieren.sh) uses this form.
if [[ -z "${GPG_PRIVATE_KEY:-}" && -n "${GPG_PRIVATE_KEY_FILE:-}" ]]; then
  [[ -s "$GPG_PRIVATE_KEY_FILE" ]] || { echo "::error::GPG_PRIVATE_KEY_FILE is empty or missing"; exit 1; }
  key_from_file=1
fi
if [[ -z "${GPG_PRIVATE_KEY:-}" && -z "${key_from_file:-}" ]]; then
  echo "::warning::GPG_PRIVATE_KEY is not set - artefacts are NOT signed (see docs/signing.md)"
  exit 0
fi

# Own, private files only (0700 dir, 0600 passphrase file), removed on EVERY exit (also on error): later steps of the
# job must not find the key ring or the passphrase in RUNNER_TEMP. Only these two paths are touched, no globs.
umask 077
tmp_base="${RUNNER_TEMP:-/tmp}"
GNUPGHOME="$(mktemp -d "$tmp_base/fs-release-gnupg.XXXXXX")"
export GNUPGHOME
pass_file="$(mktemp "$tmp_base/fs-release-pass.XXXXXX")"
cleanup() {
  gpgconf --kill gpg-agent >/dev/null 2>&1 || true
  [[ -n "${pass_file:-}" && -f "$pass_file" ]] && { shred -u "$pass_file" 2>/dev/null || rm -f "$pass_file"; }
  [[ -n "${GNUPGHOME:-}" && -d "$GNUPGHOME" ]] && rm -rf "$GNUPGHOME"
  return 0
}
trap cleanup EXIT
chmod 700 "$GNUPGHOME"; chmod 600 "$pass_file"
if [[ -n "${GPG_PASSPHRASE_FILE:-}" && -z "${GPG_PASSPHRASE:-}" ]]; then cat -- "$GPG_PASSPHRASE_FILE" > "$pass_file"; else printf '%s' "${GPG_PASSPHRASE:-}" > "$pass_file"; fi
if [[ -n "${key_from_file:-}" ]]; then gpg --batch --import < "$GPG_PRIVATE_KEY_FILE"; else printf '%s' "$GPG_PRIVATE_KEY" | gpg --batch --import; fi
fpr=$(gpg --batch --list-secret-keys --with-colons | awk -F: '/^fpr:/ {print $10; exit}')
[[ -n "$fpr" ]] || { echo "::error::no secret key found in GPG_PRIVATE_KEY"; exit 1; }
gpg_opts=(--batch --yes --pinentry-mode loopback --passphrase-file "$pass_file" --local-user "$fpr")
echo "Signing key fingerprint: $fpr"

case "$mode" in
  packages)
    shopt -s nullglob
    debs=("$dir"/*.deb) rpms=("$dir"/*.rpm)
    if ((${#debs[@]})); then
      sudo apt-get install -y --no-install-recommends dpkg-sig >/dev/null || echo "::warning::dpkg-sig unavailable - .deb stays unsigned (detached .asc still written)"
      if command -v dpkg-sig >/dev/null; then
        dpkg-sig -g "--batch --pinentry-mode loopback --passphrase-file $pass_file" -k "$fpr" --sign builder "${debs[@]}"
      fi
    fi
    if ((${#rpms[@]})); then
      sudo apt-get install -y --no-install-recommends rpm >/dev/null
      for r in "${rpms[@]}"; do
        rpmsign --define "_gpg_name $fpr" \
          --define "_gpg_sign_cmd_extra_args --pinentry-mode loopback --passphrase-file $pass_file" \
          --addsign "$r"
      done
    fi
    gpg --batch --armor --export "$fpr" > "$dir/FreeSynergy-release-key.pgp"
    ;;
  detached)
    find "$dir" -maxdepth 1 -type f ! -name '*.asc' -print0 | while IFS= read -r -d '' f; do
      gpg "${gpg_opts[@]}" --armor --detach-sign --output "$f.asc" "$f"
    done
    ;;
  *) echo "usage: sign.sh packages|detached DIR" >&2; exit 2 ;;
esac
