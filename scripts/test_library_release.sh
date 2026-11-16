#!/usr/bin/env bash
# Self test of the library mode of release.yml with counter-checks on fixtures (no network, fake `gh`):
#   notes.py      ok case; wrong tag name, version != tag, missing / empty / blank-only section = rc 1 and no notes file
#   plan.py       empty binaries gives a library plan (no platform job); a binary release with a bad tag still fails
#   release.sh    draft first, then Pre-release; an existing release is refused; dry-run creates nothing
#   release.yml   platform jobs, finalize (SBOM, signatures, secrets) and publish are off in library mode; the job `library` has no secret and no
#                 sign / checksum / sbom step
# Mutations on copies (the version check removed, the `library` guard of finalize removed, a secret added to the library job) must turn the tests red.
#   bash scripts/test_library_release.sh
set -uo pipefail
here="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
repo="$(cd "$here/.." && pwd)"
tmp="$(mktemp -d)"
trap 'rm -rf "$tmp"' EXIT
fails=0
ok() { echo "ok   $1"; }
bad() { echo "FAIL $1"; fails=$((fails + 1)); }
python3 -c "import yaml" 2>/dev/null || { echo "skipped: no PyYAML (the workflow structure was NOT checked)"; exit 2; }

# fixture <dir> <cargo version> <changelog text>
fixture() {
  mkdir -p "$1"
  printf '[workspace]\nmembers = []\n[workspace.package]\nversion = "%s"\n' "$2" >"$1/Cargo.toml"
  printf '%s' "$3" >"$1/CHANGELOG.md"
}
notes() { python3 -I "$1/actions/library-release/notes.py" "$2" "$3" "$2/notes.out" 2>&1; }
# expect_notes <root of the actions> <name> <rc> <dir> <tag> [needle]
expect_notes() {
  local out rc
  rm -f "$4/notes.out"
  out="$(notes "$1" "$4" "$5")"; rc=$?
  if [ "$rc" -ne "$3" ]; then bad "$2: rc=$rc (want $3): $out"; return; fi
  if [ "$3" -ne 0 ] && [ -e "$4/notes.out" ]; then bad "$2: a notes file was written on a failure"; return; fi
  if [ -n "${6:-}" ] && ! grep -qF -- "$6" <<<"$out"; then bad "$2: needle '$6' missing in: $out"; return; fi
  ok "$2"
}

run_notes_cases() { # <actions root> <label>
  local r="$1" l="$2"
  fixture "$tmp/$l-ok" 0.1.2 $'# Changelog\n\n## 0.1.2\n\n- first\n- second\n\n## 0.1.1\n\n- old\n'
  expect_notes "$r" "$l: tag = version = section" 0 "$tmp/$l-ok" v0.1.2
  if [ "$(cat "$tmp/$l-ok/notes.out" 2>/dev/null)" = $'- first\n- second' ]; then ok "$l: the notes are exactly the section"; else bad "$l: notes differ: $(cat "$tmp/$l-ok/notes.out" 2>/dev/null)"; fi
  fixture "$tmp/$l-named" 0.1.2 $'## fs-components 0.1.2\n\n- x\n'
  expect_notes "$r" "$l: heading with a name" 0 "$tmp/$l-named" v0.1.2
  for t in 0.1.2 v0.1 vx.1.2 v01.1.2 v0.1.2.3 V0.1.2; do expect_notes "$r" "$l: wrong tag name '$t'" 1 "$tmp/$l-ok" "$t" "not a SemVer"; done
  expect_notes "$r" "$l: version != tag" 1 "$tmp/$l-ok" v0.1.3 "differs from the workspace version"
  fixture "$tmp/$l-vdiff" 0.1.1 $'## 0.1.2\n\n- x\n'
  expect_notes "$r" "$l: Cargo version differs from the tag" 1 "$tmp/$l-vdiff" v0.1.2 "differs from the workspace version"
  fixture "$tmp/$l-empty" 0.1.2 $'## 0.1.2\n\n## 0.1.1\n\n- old\n'
  expect_notes "$r" "$l: empty section" 1 "$tmp/$l-empty" v0.1.2 "no non-empty section"
  fixture "$tmp/$l-blank" 0.1.2 $'## 0.1.2\n   \n\t\n'
  expect_notes "$r" "$l: blank-only section" 1 "$tmp/$l-blank" v0.1.2 "no non-empty section"
  fixture "$tmp/$l-missing" 0.1.2 $'## 0.1.1\n\n- old\n'
  expect_notes "$r" "$l: missing section" 1 "$tmp/$l-missing" v0.1.2 "no non-empty section"
}
run_notes_cases "$repo" real

# plan.py
mkdir -p "$tmp/plan"
out="$(cd "$tmp/plan" && TAG=v0.1.2 BINARIES="" GITHUB_OUTPUT="$tmp/plan/out" GITHUB_REPOSITORY=O/fs-lib python3 -I "$repo/actions/plan/plan.py" 2>&1)"; rc=$?
if [ "$rc" -eq 0 ] && python3 - "$tmp/plan/out" <<'PY'
import json, sys
line = [l for l in open(sys.argv[1]) if l.startswith("plan=")][0][5:]
p = json.loads(line)
assert p["library"] is True and p["binaries"] == [] and not (p["has_linux"] or p["has_windows"] or p["has_macos"] or p["has_arch"]), p
PY
then ok "plan.py: empty binaries = library plan, no platform"; else bad "plan.py library: rc=$rc $out"; fi
(cd "$tmp/plan" && TAG=1.2 BINARIES="" GITHUB_OUTPUT="$tmp/plan/out2" python3 -I "$repo/actions/plan/plan.py" >/dev/null 2>&1); [ $? -eq 1 ] && ok "plan.py: library with a bad tag name is red" || bad "plan.py: bad tag in library mode was accepted"
(cd "$tmp/plan" && TAG=1.2 BINARIES="x" GITHUB_OUTPUT="$tmp/plan/out3" python3 -I "$repo/actions/plan/plan.py" >/dev/null 2>&1); [ $? -eq 1 ] && ok "plan.py: binary release with a bad tag is still red" || bad "plan.py: binary release lost its tag check"

# release.sh with a fake gh that records its calls
mkdir -p "$tmp/bin"
cat >"$tmp/bin/gh" <<'GH'
#!/usr/bin/env bash
echo "$*" >>"$GH_LOG"
if [ "$1 $2" = "release view" ]; then [ -n "${GH_EXISTS:-}" ] && exit 0; exit 1; fi
exit 0
GH
chmod +x "$tmp/bin/gh"
printf '%s\n' '- first' >"$tmp/notes.md"
PATH="$tmp/bin:$PATH" GH_LOG="$tmp/gh.log" TAG=v0.1.2 REPO=O/fs-lib GH_TOKEN=x NOTES="$tmp/notes.md" bash "$repo/actions/library-release/release.sh" >"$tmp/rel.out" 2>&1; rc=$?
if [ "$rc" -eq 0 ] && grep -q '^release create v0.1.2 --draft --prerelease --verify-tag .*--notes-file' "$tmp/gh.log" && grep -q '^release edit v0.1.2 --draft=false --prerelease' "$tmp/gh.log"; then ok "release.sh: draft, then Pre-release"; else bad "release.sh: rc=$rc log: $(cat "$tmp/gh.log")"; fi
if grep -q -- '--generate-notes' "$tmp/gh.log"; then bad "release.sh: generated notes instead of the CHANGELOG"; else ok "release.sh: notes come from the file, not generated"; fi
: >"$tmp/gh.log"
GH_EXISTS=1 PATH="$tmp/bin:$PATH" GH_LOG="$tmp/gh.log" TAG=v0.1.2 REPO=O/fs-lib GH_TOKEN=x NOTES="$tmp/notes.md" bash "$repo/actions/library-release/release.sh" >"$tmp/rel.out" 2>&1; rc=$?
if [ "$rc" -eq 1 ] && ! grep -q 'release create\|release edit' "$tmp/gh.log"; then ok "release.sh: an existing release is refused and untouched"; else bad "release.sh existing: rc=$rc log: $(cat "$tmp/gh.log")"; fi
: >"$tmp/gh.log"
DRY_RUN=true PATH="$tmp/bin:$PATH" GH_LOG="$tmp/gh.log" TAG=v0.1.2 REPO=O/fs-lib GH_TOKEN=x NOTES="$tmp/notes.md" bash "$repo/actions/library-release/release.sh" >"$tmp/rel.out" 2>&1; rc=$?
if [ "$rc" -eq 0 ] && ! grep -q 'release create\|release edit' "$tmp/gh.log"; then ok "release.sh: dry-run creates nothing"; else bad "release.sh dry-run: rc=$rc log: $(cat "$tmp/gh.log")"; fi

# structure of release.yml in library mode
structure() { # <release.yml> <label>: prints FAIL lines
  python3 - "$1" <<'PY'
import sys, yaml
w = yaml.safe_load(open(sys.argv[1]))
jobs = w["jobs"]
on = w[True] if True in w else w["on"]
bad = []
def cond(j): return str(jobs[j].get("if", ""))
lib = "fromJSON(needs.plan.outputs.plan).library"
for j in ("linux", "windows", "macos"):
    # off through the has_* flags of the plan, which the library plan sets to false
    if f"has_{ {'linux':'linux','windows':'windows','macos':'macos'}[j] }" not in cond(j):
        bad.append(f"job {j} is not guarded by its has_* flag")
for j in ("flatpak", "container", "mobile", "finalize"):
    if f"!{lib}" not in cond(j):
        bad.append(f"job {j} is not off in library mode")
if "publish" in jobs and "finalize.result == 'success'" not in cond("publish"):
    bad.append("publish does not depend on finalize")
L = jobs.get("library")
if not L:
    bad.append("no job library")
else:
    if lib not in str(L.get("if", "")): bad.append("job library does not run only in library mode")
    text = yaml.dump(L)
    for word in ("secrets", "GPG", "sign-files", "sbom", "checksums", "draft-release", "attest", "id-token", "package"):
        if word in text: bad.append(f"job library contains '{word}'")
    if L.get("permissions") != {"contents": "write"}: bad.append("job library permissions are not exactly contents: write")
if on["workflow_call"]["inputs"]["binaries"].get("required"): bad.append("binaries is still required")
for b in bad: print("FAIL", b)
PY
}
out="$(structure "$repo/.github/workflows/release.yml" real)"
if [ -z "$out" ]; then ok "release.yml: library mode switches off packages, signing, publish; the library job has no secret"; else bad "release.yml structure: $out"; fi

# mutations on copies: every one must turn its check red
mutate() { # <name> <file in the copy> <sed expression> <test kind>
  local d="$tmp/mut-$1"
  mkdir -p "$d" && cp -r "$repo/actions" "$d/" && mkdir -p "$d/.github/workflows" && cp "$repo/.github/workflows/release.yml" "$d/.github/workflows/"
  sed -i "$3" "$d/$2"
  if cmp -s "$d/$2" "$repo/$2"; then bad "mutation $1 changed nothing"; return; fi
  case "$4" in
    notes) (run_notes_cases "$d" "mut-$1") >"$tmp/mut.out" 2>&1; grep -q '^FAIL' "$tmp/mut.out" && ok "mutation $1 turns the notes cases red" || bad "mutation $1 stayed green" ;;
    structure) [ -n "$(structure "$d/$2")" ] && ok "mutation $1 turns the structure check red" || bad "mutation $1 stayed green" ;;
  esac
}
mutate version actions/library-release/notes.py 's/    if have != version:/    if False:/' notes
mutate section actions/library-release/notes.py 's/    if notes is None or not notes.strip():/    if notes is None:/' notes
mutate semver actions/library-release/notes.py 's/    if not tag.startswith("v") or not SEMVER.match(tag\[1:\]):/    if False:/' notes
mutate finalize .github/workflows/release.yml "s/ \&\& !fromJSON(needs.plan.outputs.plan).library \&\& !contains/ \&\& !contains/" structure
mutate secret .github/workflows/release.yml 's/          token: \${{ github.token }}\n*$/&/; /name: \${{ inputs.name }}/a\          private-key: ${{ secrets.GPG_PRIVATE_KEY }}' structure
mutate permissions .github/workflows/release.yml '/^  library:/,/^  publish:/ s/      contents: write/      contents: write\n      id-token: write/' structure

if [ "$fails" -eq 0 ]; then echo "all ok"; else echo "$fails failed"; exit 1; fi
