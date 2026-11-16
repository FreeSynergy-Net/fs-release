"""Selbsttest von doku-themen: ein Coderepo mit zwei Commits und eine Doku-Fixture; jede Regel mit Fall und Gegenprobe."""
import json
import os
import shutil
import subprocess
import tempfile

from themen import lauf, schreibe_marke


def sh(cwd, *args):
    subprocess.run(["git", "-C", cwd, "-c", "user.name=t", "-c", "user.email=t@example.invalid",
                    "-c", "commit.gpgsign=false", *args], check=True, capture_output=True, text=True)


def put(root, rel, text):
    path = os.path.join(root, rel)
    os.makedirs(os.path.dirname(path), exist_ok=True)
    with open(path, "w", encoding="utf-8") as f:
        f.write(text)


def selbsttest():
    tmp = tempfile.mkdtemp(prefix="doku-themen-selbst-")
    try:
        code = os.path.join(tmp, "fs-release")
        docs = os.path.join(tmp, "docs")
        os.makedirs(code)
        sh(code, "init", "-q")
        put(code, "crates/fs-release/src/lib.rs", "// alt\n")
        put(code, "scripts/retired-names.txt", "# Kopf\n")
        sh(code, "add", "-A")
        sh(code, "commit", "-q", "-m", "start")
        marke_a = subprocess.run(["git", "-C", code, "rev-parse", "HEAD"], capture_output=True, text=True).stdout.strip()
        put(code, "crates/fs-release/src/lib.rs", "// neu\n")
        put(code, "scripts/retired-names.txt", "# Kopf\nfs-alt -> fs-neu  # umbenannt\n")
        put(code, "crates/fs-fremd/src/lib.rs", "// fremd\n")
        sh(code, "add", "-A")
        sh(code, "commit", "-q", "-m", "ADR 0080: Umbau\n\nAuch fs-release.")
        put(docs, "docs/topics.toml", (
            '[[thema]]\nname = "release"\ncrates = ["fs-release"]\nseiten = ["docs/de/release*.md"]\n\n'
            '[[thema]]\nname = "namen"\nnamen = ["fs-alt"]\nseiten = ["docs/de/namen.md"]\n\n'
            '[[thema]]\nname = "adr"\nadr = ["0080"]\nseiten = ["docs/de/adr80.md"]\n\n'
            '[[thema]]\nname = "fremd"\ncrates = ["fs-nichtda"]\nseiten = ["docs/de/fremd.md"]\n'))
        put(docs, "docs/de/release-ablauf.md", "# Ablauf\n")
        put(docs, "docs/de/namen.md", "# Namen\n")
        put(docs, "docs/de/adr80.md", "# ADR-Text\n")
        put(docs, "docs/de/fremd.md", "# nur fremd\n")
        put(docs, "docs/de/front.md", "---\ntopics: [release]\n---\n# Front\n")
        put(docs, "docs/de/grep.md", "# Grep\nDer Crate fs-fremd ist hier genannt.\n")
        put(docs, "docs/de/unbezogen.md", "# Nichts\n")
        state = os.path.join(tmp, "state", "doku-aufraeumen.state")
        toml = os.path.join(docs, "docs", "topics.toml")
        fehler = []
        try:
            lauf(docs, code, state, toml)
            fehler.append("ohne Marke: muss abbrechen")
        except ValueError:
            pass
        schreibe_marke(state, marke_a)
        with open(state, encoding="utf-8") as f:
            zustand = json.load(f)
        r = lauf(docs, code, state, toml)
        seiten = set(r["seiten"])
        checks = [
            (zustand["marke"] == marke_a, "marke: Zustand traegt den Commit"),
            (r["status"] == "ok", "status: Aenderungen seit der Marke ergeben ok"),
            ({"release", "namen", "adr"} <= set(r["themen"]), "themen: crates, namen und adr treffen ihre Themen"),
            ("docs/de/release-ablauf.md" in seiten, "seiten: Glob des Themas crates"),
            ("docs/de/namen.md" in seiten, "seiten: Thema namen (retired-names-Zeile)"),
            ("docs/de/adr80.md" in seiten, "seiten: Thema adr (Commit-Text ADR 0080)"),
            ("docs/de/front.md" in seiten, "seiten: Front-Matter topics: [release]"),
            ("docs/de/grep.md" in seiten, "seiten: grep nach geaendertem Crate-Namen"),
            ("docs/de/unbezogen.md" not in seiten, "seiten: Gegenprobe, unbezogene Seite bleibt draussen"),
            ("docs/de/fremd.md" not in seiten, "seiten: Gegenprobe, Thema ohne Treffer bleibt draussen"),
            ("fs-fremd" in r["ohne_seite"], "ohne_seite: geaenderter Crate ohne Thema steht drin"),
            ("fs-alt" not in r["ohne_seite"], "ohne_seite: abgedeckter Name steht nicht drin"),
        ]
        schreibe_marke(state, r["kopf"])
        r2 = lauf(docs, code, state, toml)
        checks += [
            (r2["status"] == "nichts_zu_tun" and r2["seiten"] == [], "nichts_zu_tun: Marke = HEAD, keine Seiten"),
        ]
        try:
            schreibe_marke(state, "nicht-sha")
            fehler.append("schreibe_marke: kein SHA muss abbrechen")
        except ValueError:
            pass
        checks.append((not fehler, "marke: Gegenprobe ohne Marke und mit falschem SHA"))
        bad = [m for ok, m in checks if not ok] + fehler
        print(json.dumps({"selbsttest": "red" if bad else "green", "fehlend": bad, "pruefungen": len(checks)},
                         ensure_ascii=False))
        return 1 if bad else 0
    finally:
        shutil.rmtree(tmp, ignore_errors=True)
