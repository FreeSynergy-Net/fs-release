# Pattern: none (script of the pin cascade).
"""Input and output of the pin-kaskade scripts: JSON in (a literal, a file, or `-` = stdin), JSON out on stdout, rc 0 green / 1 red / 2 bad input."""
import json
import re
import sys

TAG = re.compile(r"^v[0-9]+\.[0-9]+\.[0-9]+$")
ARTEN = ("bibliothek", "programm")


def load_json(arg):
    """A JSON text, the path of a JSON file, or `-` (stdin). Raises ValueError on anything unreadable (the caller exits 2, nothing is guessed)."""
    try:
        if arg == "-":
            return json.load(sys.stdin)
        if arg.lstrip().startswith(("{", "[")):
            return json.loads(arg)
        with open(arg, encoding="utf-8") as f:
            return json.load(f)
    except (OSError, json.JSONDecodeError) as e:
        raise ValueError(f"cannot read JSON from {arg[:60]!r}: {e}") from e


def emit(obj):
    print(json.dumps(obj, ensure_ascii=False, sort_keys=True))


def skipped(msg, **more):
    emit({"status": "skipped", "red": True, "fehler": [msg], **more})
    return 2


def check_glieder(glieder):
    """Fehlertexte of a `glieder` list: every Glied needs repo, art (bibliothek|programm), tag (vX.Y.Z), allow; no repo twice."""
    err, seen = [], set()
    if not isinstance(glieder, list) or not glieder:
        return ["glieder: a non-empty list is required"]
    for g in glieder:
        if not isinstance(g, dict):
            err.append(f"glieder: not an object: {g!r}")
            continue
        r = g.get("repo")
        if not isinstance(r, str) or not re.match(r"^[A-Za-z0-9][A-Za-z0-9_.-]*$", r):
            err.append(f"glieder: bad repo {r!r}")
        if r in seen:
            err.append(f"glieder: {r} twice")
        seen.add(r)
        if g.get("art") not in ARTEN:
            err.append(f"glieder {r}: art must be bibliothek|programm")
        if not isinstance(g.get("tag"), str) or not TAG.match(g["tag"]):
            err.append(f"glieder {r}: tag must be vX.Y.Z")
        if "allow" not in g:
            err.append(f"glieder {r}: allow is missing")
    return err
