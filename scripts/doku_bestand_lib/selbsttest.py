"""Selbsttest von doku-bestand: eine Fixture-Doku in einem temporaeren Ordner, je Befund und je Fehlalarm-Regel ein Fall.

Jede Pruefung ist ein Paar: der Fall muss erkannt bzw. gruen sein, die Gegenprobe (ein echter Fehler) muss rot bleiben.
"""
import os
import shutil
import tempfile

from befunde import find_de_en, find_statuszeilen
from links import doc_md_files, pruefe_links, read


def build(tmp):
    """Fixture: docs/ mit verwaister Seite, Menue (.pages), decisions, Vorlagen, Anker, en-only, temp, Statuszeilen."""
    d = os.path.join(tmp, "klon")
    for sub in ("docs/de/ordner/tief", "docs/de/frei", "docs/de/decisions", "docs/de/temp", "docs/en", "conventions/templates"):
        os.makedirs(os.path.join(d, sub))

    def put(rel, text):
        with open(os.path.join(d, rel), "w", encoding="utf-8") as f:
            f.write(text)

    put("mkdocs.yml", "nav:\n  - Start: docs/de/start.md\n")
    put("README.md", "[start](docs/de/start.md)\n")
    put("docs/de/index.md", "# Index\n")
    put("docs/de/start.md",
        "# Start\n[tot](nicht-da.md) [ok](../en/gut.md) [erzeugt](decisions/0099-fehlt.md)\n"
        "[vorlage](../../conventions/templates/vorlage.md) [kaputt](../../conventions/fehlt.md)\n"
        "[anker-ok](anon.md#kis-ausschlieen) [anker-falsch](anon.md#kis-ausschließen)\n"
        "[ausserhalb-temp](nicht-da-temp.md)\n"
        "```\n(node(area.a);) [code](code-fehlt.md)\n```\n`[syntax](inline-fehlt.md)`\n"
        "[anker-kurz](#oben)\n")
    put("docs/de/verwaist-de.md", "# lose\n")
    put("conventions/templates/vorlage-quelle.md", "# Vorlage [platz](../../../apps/fehlt.md)\n")
    put("docs/de/ordner/.pages", "nav:\n  - tief\n")
    put("docs/de/ordner/tief/seite.md", "# Menue-Seite, ohne Link\n")
    put("docs/de/frei/losseite.md", "# frei, ohne Menue\n")
    put("docs/de/anon.md", "# Anon\n\n## KIs ausschließen\n\nText.\n")
    put("docs/de/temp/x.md", "# Temp [weg](nicht-da-temp.md)\n")
    put("docs/de/nur-de.md", "# Nur deutsch\n")
    put("docs/en/nur-en.md", "# Nur englisch, erlaubt\n")
    put("docs/en/gut.md", "# Good\n")
    put("docs/de/gut.md", "# Gut\n")
    put("docs/en/index.md", "# Index\n")
    put("docs/en/bak.md.bak", "copy\n")
    put("docs/de/decisions/0001-mit.md", "# ADR 1\n\nStatus: akzeptiert (2026-01-01)\n")
    put("docs/de/decisions/0002-ohne.md", "# ADR 2\n\nkeine Zeile\n")
    with open(os.path.join(tmp, "retired.txt"), "w", encoding="utf-8") as f:
        f.write("# Kopf\nfs-alt-name -> fs-neu  # umbenannt\n")
    put("docs/de/erlaubt.md", "fs-alt-name in der Historie\n")
    put("docs/de/treffer.md", "Hier steht fs-alt-name.\n")
    return d


def selbsttest():
    tmp = tempfile.mkdtemp(prefix="doku-bestand-selbst-")
    try:
        d = build(tmp)
        from befunde import find_old_names, find_temp, read_retired
        mdfiles = doc_md_files(d)
        nav = read(os.path.join(d, "mkdocs.yml"))
        verwaist, dead, anker = pruefe_links(d, mdfiles, nav)
        de_en = find_de_en(d)
        names = read_retired(os.path.join(tmp, "retired.txt"))
        alte = find_old_names(d, mdfiles, names, ["docs/de/erlaubt.md"])
        statz = find_statuszeilen(d, mdfiles)
        temp = find_temp(d)
        ds = os.path.join
        checks = [
            (ds("docs", "de", "verwaist-de.md") in verwaist, "verwaist: lose Seite gefunden"),
            (ds("docs", "de", "frei", "losseite.md") in verwaist, "verwaist: Gegenprobe, Seite ohne Menue bleibt Befund"),
            (ds("docs", "de", "ordner", "tief", "seite.md") not in verwaist, "regel awesome-pages: Seite im .pages-Menue gilt als verlinkt"),
            (not any(x.endswith("index.md") for x in verwaist), "verwaist: index.md nie gemeldet"),
            (any(x["ziel"] == "nicht-da.md" for x in dead), "tote_links: fehlendes Ziel gefunden"),
            (not any(x["ziel"] == "../en/gut.md" for x in dead), "tote_links: vorhandenes Ziel nicht gemeldet"),
            (any(x["ziel"] == "../../conventions/fehlt.md" for x in dead), "regel vorlagen: Gegenprobe, Ziel ausserhalb templates bleibt rot"),
            (not any(x["ziel"] == "../../conventions/templates/vorlage.md" for x in dead), "regel vorlagen: conventions/templates/ ist Platzhalter"),
            (not any(x["ziel"].endswith("0099-fehlt.md") for x in dead), "regel decisions: CI-erzeugter Link ist kein toter Link"),
            (not any(x["ziel"] == "code-fehlt.md" for x in dead), "regel code: Link in Codeblock ist kein Befund"),
            (not any(x["ziel"] == "inline-fehlt.md" for x in dead), "regel code: Link in Backticks ist kein Befund"),
            (not any(x["datei"].startswith(ds("conventions", "templates")) for x in dead), "regel vorlagen: Quelle unter templates/ wird nicht geprueft"),
            (not any(x["datei"].startswith(ds("docs", "de", "temp")) for x in dead), "regel temp-quelle: Link aus temp/ wird nicht geprueft"),
            (any(x["datei"] == ds("docs", "de", "start.md") and x["ziel"] == "nicht-da-temp.md" for x in dead), "regel temp-quelle: Gegenprobe, gleicher Link aus docs/de/ bleibt rot"),
            (any(x["anker"] == "kis-ausschließen" for x in anker), "regel slug: Gegenprobe, Anker mit Nicht-ASCII bleibt rot"),
            (not any(x["anker"] == "kis-ausschlieen" for x in anker), "regel slug: mkdocs-Anker kis-ausschlieen ist richtig"),
            (not any(x["anker"] == "oben" for x in anker), "anker: Anker ohne Datei wird nicht geprueft"),
            (any(x.startswith(ds("docs", "de", "temp")) for x in temp), "temp: Ordner temp/ gefunden"),
            (any(x.endswith(".bak") for x in temp), "temp: .bak-Datei gefunden"),
            (ds("docs", "de", "nur-de.md") in de_en, "de_en_luecke: fehlendes en-Gegenstueck gefunden"),
            (not any("gut.md" in x for x in de_en), "de_en_luecke: vorhandenes Paar nicht gemeldet"),
            (not any("nur-en.md" in x for x in de_en), "regel ADR 0011: en ohne de ist erlaubt"),
            (any(h["name"] == "fs-alt-name" and h["datei"].endswith("treffer.md") for h in alte), "alte_namen: Treffer in treffer.md"),
            (not any(h["datei"].endswith("erlaubt.md") for h in alte), "alte_namen: --allow wirkt"),
            (ds("docs", "de", "decisions", "0002-ohne.md") in statz, "statuszeile: ADR ohne Statuszeile gefunden"),
            (ds("docs", "de", "decisions", "0001-mit.md") not in statz, "statuszeile: Gegenprobe, ADR mit Statuszeile gruen"),
        ]
        bad_checks = [msg for ok, msg in checks if not ok]
        print('{"selbsttest": "%s", "fehlend": %s, "pruefungen": %d}' % (
            "red" if bad_checks else "green", repr(bad_checks).replace("'", '"'), len(checks)))
        return 1 if bad_checks else 0
    finally:
        shutil.rmtree(tmp, ignore_errors=True)
