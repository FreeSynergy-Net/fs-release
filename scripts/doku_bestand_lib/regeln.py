"""Fehlalarm-Regeln aus dem Opus-Urteil (16.11.2026): was KEIN Befund ist.

Jede Regel hat im Selbsttest (doku_bestand_lib/selbsttest.py) einen Fall, der gruen bleibt, und eine Gegenprobe, die rot bleibt.
"""
import os
import re
import unicodedata

GENERIERT = "decisions"  # docs/<sprache>/decisions/ erzeugt CI (gen_decisions.py) vor `mkdocs build --strict`
PLATZHALTER = "conventions/templates/"  # Vorlagen mit Platzhalter-Links, kein Ziel
TEMP_NAMEN = ("temp", "tmp", ".tmp")  # Quellen in temp/ sind kein Doku-Baum (eigener Befund `temp`)
ADR_NAME = re.compile(r"^\d{4}-.*\.md$")
STATUS = re.compile(r"^\W*status\s*:", re.IGNORECASE)


def ist_vorlage(rel):
    """Regel Vorlagen: eine Datei UNTER conventions/templates/ ist eine Vorlage; ihre Links zeigen auf Platzhalter."""
    return rel.replace(os.sep, "/").startswith(PLATZHALTER)


INLINE_CODE = re.compile(r"`[^`\n]*`")


def ohne_codeblock(text):
    """Regel Code: Zaunbloecke (```) und Code in Backticks sind kein Link (Overpass `(area.a)`, Syntax `[x](url)` als Beispiel)."""
    out, fence = [], False
    for line in text.splitlines():
        if line.lstrip().startswith("```"):
            fence = not fence
            out.append("")
        else:
            out.append("" if fence else INLINE_CODE.sub("", line))
    return "\n".join(out)


def in_temp(rel):
    """Eine Datei unter einem Ordner temp/tmp/.tmp ist keine Quelle des Doku-Baums (Regel temp-quelle)."""
    return any(part in TEMP_NAMEN for part in rel.split(os.sep)[:-1])


def menue_gelistet(rel, docs):
    """Regel awesome-pages: ein Ordner mit `.pages` baut sein Menue aus den Dateien; eine Seite darin gilt als verlinkt."""
    d = os.path.dirname(rel)
    while True:
        if os.path.exists(os.path.join(docs, d, ".pages")):
            return True
        if d == "":
            return False
        d = os.path.dirname(d)


def ist_generiert(ziel):
    """Regel decisions: docs/<sprache>/decisions/... fehlt lokal, weil CI die Datei erzeugt (kein toter Link)."""
    parts = ziel.split(os.sep)
    return len(parts) >= 4 and parts[0] == "docs" and parts[2] == GENERIERT


def ist_platzhalter(ziel):
    """Regel Vorlagen: Links auf conventions/templates/ sind Platzhalter, kein Ziel, das existieren muss."""
    return ziel.replace(os.sep, "/").startswith(PLATZHALTER)


def en_ohne_de_erlaubt(lang):
    """Regel ADR 0011: docs/de/ ist Uebersetzungsausgabe, fallback_to_default zeigt en. Nur de ohne en ist ein Befund."""
    return lang == "en"


def mkdocs_slug(text):
    """Anker wie Python-Markdown (Standard von mkdocs): NFKD, Nicht-ASCII fällt weg. `KIs ausschließen` -> `kis-ausschlieen`.

    Regel slug: ein Anker ist richtig, wenn er dem mkdocs-Slug einer Ueberschrift entspricht (nicht dem GitHub-Slug).
    """
    value = unicodedata.normalize("NFKD", text).encode("ascii", "ignore").decode("ascii")
    value = re.sub(r"[^\w\s-]", "", value).strip().lower()
    return re.sub(r"[-\s]+", "-", value)


def ist_adr(rel):
    """Eine ADR-Datei: ein Ordner `decisions` und ein Name mit vier Ziffern am Anfang."""
    return "decisions" in rel.split(os.sep) and bool(ADR_NAME.match(os.path.basename(rel)))


def statuszeile_fehlt(text):
    """Eine ADR ohne Zeile `Status: ...` in den ersten zehn Zeilen."""
    return not any(STATUS.match(line) for line in text.splitlines()[:10])
