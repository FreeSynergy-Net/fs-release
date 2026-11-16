"""Thema-Lauf der Doku (inkrementell, nur lesen): welche Seiten zu dem gehoeren, was SEIT der Marke geaendert wurde.

Zuordnung (Format docs/topics.toml, eine Tabelle je Thema):
  [[thema]]
  name   = "release"                  # Pflicht, eindeutig
  crates = ["fs-release"]             # Crate-Namen (Ordner crates/<name>/ im Coderepo)
  namen  = ["fs-alt"]                 # alte/neue Namen: Zeilen `alt -> neu` in scripts/retired-names.txt, die seit der Marke dazukamen
  adr    = ["0080"]                   # ADR-Nummern aus Commit-Texten (`ADR 0080`) und Pfaden (decisions/0080-...)
  seiten = ["docs/de/release*.md"]    # Glob, relativ zum Klon (fnmatch, `*` darf / enthalten)
Zusaetzlich: Seiten mit Front-Matter `topics: [release, ...]` (eine Zeile) gehoeren zum Thema.
Zusaetzlich: jede Seite unter docs/, die einen geaenderten Crate- oder Namen als ganzes Wort nennt (grep), wird genannt.
Ein Thema trifft, wenn crates, namen oder adr sich mit den Aenderungen schneiden (Repo-Name nur fuer den grep).

Marke: Zustandsdatei (JSON) `{"marke": "<sha>", "bekannt": ["temp:...", ...]}`, Vorgabe $XDG_STATE_HOME/freesynergy/doku-aufraeumen/<repo>.marke (Standard ~/.local/state; ausserhalb jedes Checkouts: ueberlebt Wegwerf-Worktrees, haelt den Checkout sauber).
"""
import fnmatch
import json
import os
import re
import subprocess
import tomllib

from befunde import loesch_schluessel
from links import doc_md_files, read

ADR_TEXT = re.compile(r"\bADR\s*(\d{4})\b|\badr[-_ ](\d{4})\b", re.IGNORECASE)
ADR_PATH = re.compile(r"decisions/(\d{4})-")
RETIRED_ADD = re.compile(r"^\+\s*([\w-]+)\s*->")
SHA = re.compile(r"^[0-9a-f]{7,40}$")


class GitFehler(Exception):
    pass


def git(repo, *args):
    r = subprocess.run(["git", "-C", repo, *args], capture_output=True, text=True)
    if r.returncode != 0:
        raise GitFehler(r.stderr.strip() or f"git {args[0]} rc={r.returncode}")
    return r.stdout


def zustandsordner():
    """$XDG_STATE_HOME/freesynergy/doku-aufraeumen (Standard ~/.local/state); wird beim Schreiben angelegt."""
    basis = os.environ.get("XDG_STATE_HOME") or os.path.join(os.path.expanduser("~"), ".local", "state")
    return os.path.join(basis, "freesynergy", "doku-aufraeumen")


def standard_marke(repo):
    """Marke eines Coderepos: <zustandsordner>/<repo-name>.marke."""
    return os.path.join(zustandsordner(), os.path.basename(os.path.abspath(repo)) + ".marke")


def standard_bestand(repo):
    """Inventar des Laufs (--bekannt-aus): neben der Marke, nie im Checkout."""
    return os.path.join(zustandsordner(), os.path.basename(os.path.abspath(repo)) + ".bestand.json")


def aenderungen(repo, marke):
    """Was seit der Marke geaendert ist: Repo-Name (aus dem gemeinsamen .git, auch im Worktree), Crates, Namen, ADRs, Pfade."""
    paths = [p for p in git(repo, "diff", "--name-only", f"{marke}..HEAD").splitlines() if p]
    msgs = git(repo, "log", "--format=%B", f"{marke}..HEAD")
    retired = git(repo, "diff", f"{marke}..HEAD", "--", "scripts/retired-names.txt")
    crates = {p.split("/")[1] for p in paths if p.startswith("crates/") and p.count("/") >= 2}
    namen = {m.group(1) for line in retired.splitlines() for m in [RETIRED_ADD.match(line)] if m}
    adr = {a or b for a, b in ADR_TEXT.findall(msgs)}
    adr |= {m.group(1) for p in paths for m in [ADR_PATH.search(p)] if m}
    common = git(repo, "rev-parse", "--path-format=absolute", "--git-common-dir").strip()
    geaendert = {"repo": os.path.basename(os.path.dirname(common)),
                 "crates": sorted(crates), "namen": sorted(namen), "adr": sorted(adr)}
    return geaendert, paths


def lade_themen(path):
    with open(path, "rb") as f:
        data = tomllib.load(f)
    return data.get("thema", [])


def passende(themen, geaendert):
    """Die Namen der Themen, deren crates, namen oder adr sich mit den Aenderungen schneiden."""
    hit = set()
    for t in themen:
        if (set(t.get("crates", [])) & set(geaendert["crates"])
                or set(t.get("namen", [])) & set(geaendert["namen"])
                or {str(a) for a in t.get("adr", [])} & set(geaendert["adr"])):
            hit.add(t["name"])
    return hit


def front_themen(text):
    """`topics: [a, b]` in der Front-Matter (zwischen den ersten beiden `---`-Zeilen)."""
    if not text.startswith("---"):
        return set()
    end = text.find("\n---", 3)
    for line in (text[3:end] if end > 0 else "").splitlines():
        m = re.match(r"^topics:\s*\[(.*)\]\s*$", line)
        if m:
            return {x.strip().strip("\"'") for x in m.group(1).split(",") if x.strip()}
    return set()


def seiten_zu(docs, themen, hit, geaendert):
    """Seiten (klon-relativ, sortiert) fuer die Themen `hit` und per grep nach den geaenderten Namen."""
    mdfiles = [m for m in doc_md_files(docs) if m.startswith("docs" + os.sep)]
    out = set()
    globs = [g for t in themen if t["name"] in hit for g in t.get("seiten", [])]
    for rel in mdfiles:
        posix = rel.replace(os.sep, "/")
        text = read(os.path.join(docs, rel))
        if any(fnmatch.fnmatch(posix, g) for g in globs) or front_themen(text) & hit:
            out.add(posix)
    tokens = set(geaendert["crates"]) | set(geaendert["namen"]) | {geaendert["repo"]}
    for tok in tokens:
        pat = re.compile(r"(?<![\w-])" + re.escape(tok) + r"(?![\w-])")
        for rel in mdfiles:
            if pat.search(read(os.path.join(docs, rel))):
                out.add(rel.replace(os.sep, "/"))
    return sorted(out)


def lauf(docs, repo, state_path, themen_path):
    """Ergebnis-Dict fuer den Ablauf; wirft GitFehler / OSError bei fehlender Marke oder fehlender Tabelle."""
    if not os.path.isfile(state_path):
        raise ValueError(f"{state_path}: keine Marke (einmal doku-bestand als Vollpruefung, danach --schreibe-marke)")
    with open(state_path, encoding="utf-8") as f:
        marke = json.load(f).get("marke")
    if not marke or not SHA.match(marke):
        raise ValueError("keine Marke im Zustand: einmal doku-bestand (Vollpruefung) und danach --schreibe-marke")
    kopf = git(repo, "rev-parse", "HEAD").strip()
    geaendert, paths = aenderungen(repo, marke)
    if not paths and not geaendert["adr"] and not geaendert["namen"]:
        return {"status": "nichts_zu_tun", "marke": marke, "kopf": kopf, "geaendert": geaendert,
                "themen": [], "seiten": [], "ohne_seite": []}
    themen = lade_themen(themen_path)
    hit = passende(themen, geaendert)
    seiten = seiten_zu(docs, themen, hit, geaendert)
    abgedeckt = set()
    for t in themen:
        if t["name"] in hit:
            abgedeckt |= set(t.get("crates", [])) | set(t.get("namen", []))
    ohne = sorted((set(geaendert["crates"]) | set(geaendert["namen"])) - abgedeckt)
    return {"status": "ok", "marke": marke, "kopf": kopf, "geaendert": geaendert,
            "themen": sorted(hit), "seiten": seiten, "ohne_seite": ohne}


def schreibe_marke(state_path, sha, bestand_json=None):
    """Setzt die Marke; mit bestand_json werden die Schluessel dieses Inventars als bekannt eingetragen."""
    if not SHA.match(sha):
        raise ValueError(f"{sha!r}: kein Commit-SHA")
    alt = {}
    if os.path.isfile(state_path):
        with open(state_path, encoding="utf-8") as f:
            alt = json.load(f)
    bekannt = set(alt.get("bekannt", []))
    if bestand_json:
        with open(bestand_json, encoding="utf-8") as f:
            bekannt |= set(loesch_schluessel(json.load(f)["befunde"]))
    os.makedirs(os.path.dirname(os.path.abspath(state_path)), exist_ok=True)
    tmp = state_path + ".tmp"
    with open(tmp, "w", encoding="utf-8") as f:
        json.dump({"marke": sha, "bekannt": sorted(bekannt)}, f, ensure_ascii=False, indent=2)
        f.write("\n")
    os.replace(tmp, state_path)
