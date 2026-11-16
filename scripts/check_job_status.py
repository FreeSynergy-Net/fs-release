#!/usr/bin/env python3
"""Job status lint: a job BELOW a job whose `if` has `!cancelled()` / `always()` needs a status function of its own.

    python3 -I scripts/check_job_status.py <repo root | workflows folder | file.yml>

Pattern: none (checker; self test scripts/test_check_job_status.sh; run by lint.yml).
Why: without a status function GitHub adds an implicit `success()`, which is false when ANY job further up the `needs` chain was
skipped. `finalize` runs with `!cancelled()` although `container`/`mobile` were skipped; a `publish` that needs it with only
`needs.finalize.result == 'success'` is skipped silently (fs-packages v0.0.18). The fix is `!cancelled() && ...` (or `always()`,
`success()`, `failure()`) in the `if` of every job that has such an ancestor. Exit codes: 0 green, 1 violation (file, job, ancestor
named), 2 no PyYAML or unreadable file.
"""
import re, sys
from pathlib import Path

try:
    import yaml
except ImportError:
    print("ERROR no PyYAML: nothing was checked", file=sys.stderr)
    sys.exit(2)

LAX = re.compile(r"!\s*cancelled\(\)|\balways\(\)")
ANY = re.compile(r"!\s*cancelled\(\)|\balways\(\)|\bsuccess\(\)|\bfailure\(\)")

def needs_of(job: dict) -> list:
    n = job.get("needs", [])
    return [n] if isinstance(n, str) else list(n or [])

def check_file(f: Path) -> list:
    doc = yaml.safe_load(f.read_text(encoding="utf-8")) or {}
    jobs = doc.get("jobs") or {}
    if not isinstance(jobs, dict):
        return []
    lax = {n for n, j in jobs.items() if isinstance(j, dict) and LAX.search(str(j.get("if", "")))}
    def ancestors(n, seen=()):
        out = set()
        for p in needs_of(jobs[n]):
            if p in jobs and p not in seen:
                out |= {p} | ancestors(p, seen + (n,))
        return out
    errs = []
    for n, j in jobs.items():
        if not isinstance(j, dict) or ANY.search(str(j.get("if", ""))):
            continue
        bad = sorted(ancestors(n) & lax)
        if bad:
            errs.append(f"ERROR {f}: job `{n}` is below `{bad[0]}` (`!cancelled()`/`always()`) but its `if` has no status function: "
                        f"it is silently skipped when any job above was skipped; write `!cancelled() && ...` (or always()/success()/failure())")
    return errs

def main(argv: list) -> int:
    if len(argv) != 1:
        print(__doc__, file=sys.stderr)
        return 2
    p = Path(argv[0])
    files = [p] if p.is_file() else sorted((p / ".github/workflows" if (p / ".github/workflows").is_dir() else p).glob("*.y*ml"))
    errs = []
    try:
        for f in files:
            errs += check_file(f)
    except (OSError, yaml.YAMLError) as e:
        print(f"ERROR cannot read: {e}", file=sys.stderr)
        return 2
    print("\n".join(errs))
    return 1 if errs else 0

if __name__ == "__main__":
    sys.exit(main(sys.argv[1:]))
