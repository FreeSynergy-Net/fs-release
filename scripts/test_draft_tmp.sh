#!/usr/bin/env bash
# Self test of actions/draft-release/draft.sh: the file of the footer (mktemp) is removed when the script ends, on the draft path, on the
# refusal of a published release (exit 1) and when `gh` fails. A stub `gh` stands in; TMPDIR is a folder of the test.
# Mutant: the trap taken out must leave the file (the cases turn red).
#   bash scripts/test_draft_tmp.sh
set -uo pipefail
here="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
tmp="$(mktemp -d)"
trap 'rm -rf "$tmp"' EXIT
fails=0
ok() { echo "ok   $1"; }
bad() { echo "FAIL $1"; fails=$((fails + 1)); }
mkdir -p "$tmp/bin" "$tmp/dist" "$tmp/t"; echo f > "$tmp/dist/a.txt"
# stub gh: `release view` answers $STUB_VIEW (true/false; "none" = fails), `release create/upload` ends with $STUB_RC
cat > "$tmp/bin/gh" <<'S'
#!/usr/bin/env bash
case "$2" in
  view) [ "${STUB_VIEW:-none}" = none ] && exit 1; echo "$STUB_VIEW"; exit 0 ;;
  *) exit "${STUB_RC:-0}" ;;
esac
S
chmod +x "$tmp/bin/gh"
mkdir -p "$tmp/real/actions/draft-release"; cp "$here/../actions/draft-release/draft.sh" "$here/../actions/draft-release/footer.md" "$tmp/real/actions/draft-release/"
run() { # <draft.sh dir> -> rc; env: STUB_VIEW, STUB_RC
  PATH="$tmp/bin:$PATH" TMPDIR="$tmp/t" TAG=v1 REPO=o/r DIST="$tmp/dist" GH_TOKEN=x bash "$1/actions/draft-release/draft.sh" > "$tmp/out" 2>&1
}
left() { find "$tmp/t" -mindepth 1 | wc -l; }
STUB_VIEW=none STUB_RC=0 run "$tmp/real"; rc=$?
{ [ "$rc" -eq 0 ] && [ "$(left)" -eq 0 ]; } && ok "new draft: rc 0, no file left" || bad "new draft: rc $rc, $(left) left"
STUB_VIEW=true STUB_RC=0 run "$tmp/real"; rc=$?
{ [ "$rc" -eq 0 ] && [ "$(left)" -eq 0 ]; } && ok "existing draft replaced: no file left" || bad "existing draft: rc $rc, $(left) left"
STUB_VIEW=false run "$tmp/real"; rc=$?
{ [ "$rc" -eq 1 ] && [ "$(left)" -eq 0 ]; } && ok "published release refused (rc 1): no file left" || bad "published: rc $rc, $(left) left"
STUB_VIEW=none STUB_RC=3 run "$tmp/real"; rc=$?
{ [ "$rc" -ne 0 ] && [ "$(left)" -eq 0 ]; } && ok "gh fails (rc $rc): no file left" || bad "gh fails: rc $rc, $(left) left"
# mutant
mkdir -p "$tmp/m/actions/draft-release"; cp "$tmp/real/actions/draft-release/"* "$tmp/m/actions/draft-release/"
sed -i '/^trap /d' "$tmp/m/actions/draft-release/draft.sh"
STUB_VIEW=none STUB_RC=0 run "$tmp/m"
[ "$(left)" -ne 0 ] && ok "mutant (no trap): the file stays (test would be red)" || bad "mutant not caught"
[ "$fails" -eq 0 ] && echo "test_draft_tmp: all ok" || { echo "test_draft_tmp: $fails failed"; exit 1; }
