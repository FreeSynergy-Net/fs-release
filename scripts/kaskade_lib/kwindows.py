# Pattern: none (script of the pin cascade).
"""Windows state of a Glied repo (rule B4: every Glied gets the caller of `_windows-check.yml` before its tag). Read only: `git show` of
the workflow file, and the PUBLIC REST API (GET, no login, no `gh`; `FS_KASKADE_API`, default https://api.github.com) for the runs of one head."""
import json
import os
import re
import urllib.error
import urllib.request

import kgit

CALLER = ".github/workflows/windows.yml"
USES = re.compile(r"^\s*uses:\s*FreeSynergy/fs-release/\.github/workflows/_windows-check\.yml@(\S+)", re.M)


def caller(text, minimum):
    """(has the caller, its pinned ref, pin is a tag >= minimum). A `@main` or a lower tag is not enough."""
    m = USES.search(text or "")
    if not m:
        return False, None, False
    ref = m.group(1)
    k, lo = kgit.version_key(ref), kgit.version_key(minimum)
    return True, ref, bool(k and lo and k >= lo)


def runs_state(repo, sha):
    """'gruen' | 'rot' | 'laeuft' | 'keiner' | 'unbekannt: <why>' for the run of the workflow `windows` at exactly this head (older heads never count)."""
    api = os.environ.get("FS_KASKADE_API", "https://api.github.com").rstrip("/")
    u = f"{api}/repos/{kgit.ORG}/{repo}/actions/runs?head_sha={sha}&per_page=100"
    try:
        req = urllib.request.Request(u, headers={"Accept": "application/vnd.github+json", "User-Agent": "fs-release-pin-kaskade"})
        with urllib.request.urlopen(req, timeout=20) as r:  # noqa: S310 (https only by default; a test points it at localhost)
            data = json.load(r)
    except urllib.error.HTTPError as e:
        return f"unbekannt: HTTP {e.code}" + (" (rate limit: do not retry in a loop)" if e.code == 403 else "")
    except (OSError, ValueError) as e:
        return f"unbekannt: {e}"
    runs = [x for x in data.get("workflow_runs", []) if x.get("head_sha") == sha and (x.get("name") == "windows" or str(x.get("path", "")).endswith("windows.yml"))]
    if not runs:
        return "keiner"
    if any(x.get("status") != "completed" for x in runs):
        return "laeuft"
    if any(x.get("conclusion") not in ("success",) for x in runs):
        return "rot"
    return "gruen"
