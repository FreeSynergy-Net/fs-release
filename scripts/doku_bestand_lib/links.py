"""Tote Links, kaputte Anker und verwaiste Seiten der Doku (nur lesen). Regeln: doku_bestand_lib/regeln.py."""
import os
import re

from regeln import (in_temp, ist_generiert, ist_platzhalter, ist_vorlage, menue_gelistet, mkdocs_slug,
                    ohne_codeblock)

LINK = re.compile(r"(?<!\!)\[[^\]]*\]\(([^)\s#][^)\s]*?)(?:#([^)\s]*))?(?:\s+\"[^\"]*\")?\)")
HEAD = re.compile(r"^#{1,6}\s+(.*?)\s*#*\s*$")
SKIP_DIRS = {".git", ".venv", "site", "node_modules"}


def read(path):
    with open(path, encoding="utf-8", errors="replace") as f:
        return f.read()


def walk(root):
    """Relative Pfade aller Dateien unter root, ohne SKIP_DIRS."""
    out = []
    for dirpath, dirnames, filenames in os.walk(root):
        dirnames[:] = sorted(d for d in dirnames if d not in SKIP_DIRS)
        out.extend(os.path.relpath(os.path.join(dirpath, f), root) for f in sorted(filenames))
    return out


def doc_md_files(docs):
    """Markdown in docs/, conventions/ und README.md (Wurzel des Klons), relativ zum Klon."""
    files = []
    for rel in walk(docs):
        if not rel.endswith(".md"):
            continue
        top = rel.split(os.sep, 1)[0]
        if top in ("docs", "conventions") or rel == "README.md":
            files.append(rel)
    return files


def link_target(rel, target):
    """Der Klon-relative Pfad eines relativen Links, None bei externen Links und reinen Ankern."""
    if "://" in target or target.startswith(("mailto:", "/")):
        return None
    return os.path.normpath(os.path.join(os.path.dirname(rel), target))


def headings(text):
    """Die mkdocs-Anker aller Ueberschriften (Codeblöcke werden nicht ausgewertet)."""
    slugs, fence = set(), False
    for line in text.splitlines():
        if line.lstrip().startswith("```"):
            fence = not fence
        m = None if fence else HEAD.match(line)
        if m:
            slugs.add(mkdocs_slug(m.group(1)))
    return slugs


def menue_oder_nav(rel, docs, nav_text):
    """Eine Seite ist eingebunden: Textlink, Eintrag in mkdocs.yml oder Menue eines .pages-Ordners."""
    return rel.split(os.sep, 1)[-1] in nav_text or menue_gelistet(rel, docs)


def pruefe_links(docs, mdfiles, nav_text):
    """Gibt (verwaist, tote_links, anker_tot) zurueck; Regeln aus regeln.py sind schon angewandt."""
    incoming, dead, anker = {}, [], []
    for rel in mdfiles:
        if in_temp(rel) or ist_vorlage(rel):
            continue
        for m in LINK.finditer(ohne_codeblock(read(os.path.join(docs, rel)))):
            tgt = link_target(rel, m.group(1))
            if tgt is None:
                continue
            if not os.path.exists(os.path.join(docs, tgt)):
                if not (ist_generiert(tgt) or ist_platzhalter(tgt)):
                    dead.append({"datei": rel, "ziel": m.group(1)})
                continue
            incoming[tgt] = incoming.get(tgt, 0) + 1
            if m.group(2) and tgt.endswith(".md") and m.group(2) not in headings(read(os.path.join(docs, tgt))):
                anker.append({"datei": rel, "ziel": m.group(1), "anker": m.group(2)})
    orphans = []
    for rel in mdfiles:
        if not rel.startswith("docs" + os.sep) or os.path.basename(rel) == "index.md":
            continue
        if incoming.get(rel) or menue_oder_nav(rel, docs, nav_text):
            continue
        orphans.append(rel)
    return orphans, dead, anker
