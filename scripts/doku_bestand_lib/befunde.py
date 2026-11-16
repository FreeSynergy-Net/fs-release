"""Die uebrigen Befundarten von doku-bestand (nur lesen): temp, de/en, alte Namen, Statuszeilen, alte Branches."""
import datetime
import fnmatch
import os
import re
import subprocess

from links import SKIP_DIRS, read, walk
from regeln import en_ohne_de_erlaubt, ist_adr, statuszeile_fehlt

TEMP_DIRS = {"temp", "tmp", ".tmp"}
TEMP_SUFFIX = (".tmp", ".bak", "~")
MAX_AGE_DAYS = 3


def find_temp(docs):
    hits = []
    for dirpath, dirnames, filenames in os.walk(docs):
        dirnames[:] = [d for d in dirnames if d not in SKIP_DIRS]
        for d in dirnames:
            if d in TEMP_DIRS:
                hits.append(os.path.relpath(os.path.join(dirpath, d), docs) + "/")
        for f in filenames:
            if f.endswith(TEMP_SUFFIX):
                hits.append(os.path.relpath(os.path.join(dirpath, f), docs))
    return sorted(hits)


def find_de_en(docs):
    """Nur de ohne en ist ein Befund; en ohne de ist erlaubt (Regel ADR 0011)."""
    out = []
    for lang in ("de", "en"):
        base = os.path.join(docs, "docs", lang)
        if not os.path.isdir(base) or en_ohne_de_erlaubt(lang):
            continue
        for rel in walk(base):
            if rel.endswith(".md") and not os.path.exists(os.path.join(docs, "docs", "en", rel)):
                out.append(os.path.join("docs", lang, rel))
    return sorted(out)


def find_statuszeilen(docs, mdfiles):
    return sorted(rel for rel in mdfiles if ist_adr(rel) and statuszeile_fehlt(read(os.path.join(docs, rel))))


def read_retired(path):
    names = []
    for line in read(path).splitlines():
        line = line.split("#", 1)[0].strip()
        if "->" in line:
            names.append(line.split("->", 1)[0].strip())
    return [n for n in names if n]


def find_old_names(docs, mdfiles, names, allow):
    hits = []
    for name in names:
        pat = re.compile(r"(?<![\w-])" + re.escape(name) + r"(?![\w-])")
        for rel in mdfiles:
            if any(fnmatch.fnmatch(rel, g) for g in allow):
                continue
            for n, line in enumerate(read(os.path.join(docs, rel)).splitlines(), 1):
                if pat.search(line):
                    hits.append({"name": name, "datei": rel, "zeile": n})
    return hits


def git(repo, *args):
    return subprocess.run(["git", "-C", repo, *args], capture_output=True, text=True)


def find_old_branches(docs, now):
    """None, wenn origin/main fehlt (der Befund ist dann nicht gelaufen, nie gruen)."""
    if git(docs, "rev-parse", "--verify", "-q", "origin/main").returncode != 0:
        return None
    r = git(docs, "for-each-ref", "--format=%(committerdate:unix) %(refname:short)", "refs/heads", "refs/remotes")
    if r.returncode != 0:
        return None
    out = []
    for line in r.stdout.splitlines():
        ts, ref = line.split(" ", 1)
        if ref in ("origin/main", "origin/HEAD", "main"):
            continue
        age = (now - int(ts)) / 86400
        if age <= MAX_AGE_DAYS:
            continue
        if git(docs, "merge-base", "--is-ancestor", ref, "origin/main").returncode == 0:
            continue
        out.append({"branch": ref, "alter_tage": int(age)})
    return out


def loesch_schluessel(befunde):
    """Befunde, die eine Entscheidung (Opus) brauchen: Ordner temp, verwaiste Seiten, alte Branches.

    Ein Schluessel ist bekannt, sobald die Entscheidung gefallen ist (Zustandsdatei `bekannt`); nur NEUE Schluessel
    loesen wieder einen Opus-Lauf aus.
    """
    keys = ["temp:" + t for t in befunde["temp"]]
    keys += ["verwaist:" + v for v in befunde["verwaist"]]
    keys += ["branch:" + b["branch"] for b in befunde["branches_alt"]]
    return sorted(keys)


def now_unix(text=None):
    if text:
        return int(datetime.datetime.fromisoformat(text).replace(tzinfo=datetime.timezone.utc).timestamp())
    return int(datetime.datetime.now(datetime.timezone.utc).timestamp())
