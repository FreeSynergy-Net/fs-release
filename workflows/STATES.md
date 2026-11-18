# States of the workflows / Zustände der Workflows

**EN.** Workflows are swimlanes: every lane in fs-tasks is a station of the workflow. Every file in `workflows/*.yaml` therefore carries the
same eight mandatory states plus its own steps (the work stations, "in progress"). The states stand in `document.metadata` of the file
(`freesynergy.net:states`) with the transition rules (`freesynergy.net:transitions`); the swimlanes are the eight states and the `do` steps.
Only the state names are fixed, no step may carry one of these names (and not `stationen`). The check is `scripts/check_workflows.py`
(rules in the constant `TRANSITIONS`, the one place), the tests are `crates/fs-release/tests/workflows_zustaende.rs`, `workflows_zustaende_luecken.rs` and `workflows_zustaende_regeln.rs`. No build logic lives here: building belongs to the
construction kit, not to fs-tasks.

**DE.** Workflows sind Swimlanes: jede Spalte in fs-tasks ist eine Station des Workflows. Jede Datei `workflows/*.yaml` trägt deshalb
dieselben acht Pflichtzustände plus ihre eigenen Schritte (die Arbeitsstationen, „in Arbeit“). Die Zustände stehen in `document.metadata`
(`freesynergy.net:states`) mit den Übergangsregeln (`freesynergy.net:transitions`).

| State / Zustand | Meaning / Bedeutung |
|---|---|
| `offen` | created, not started (start state). confirmed by the user: tasks start here / vom Nutzer bestätigt: am Anfang stehen die Aufgaben |
| `wartet-auf-entscheidung` | a person must decide (gate) / ein Mensch muss entscheiden |
| `auf-anfrage` | waits for an answer from outside (request sent) / wartet auf eine Antwort von außen |
| `in-prüfung` | a review or check is running / Prüfung oder Review läuft |
| `pausiert` | stopped on purpose, may continue / bewusst angehalten, kann weitergehen |
| `abgebrochen` | ended without result / ohne Ergebnis beendet |
| `erledigt` | goal reached / Ziel erreicht |
| `archiviert` | closed for good (end state) / endgültig abgelegt (Endzustand) |
| own steps (`do`) | the work stations ("in progress"; token `stationen` in the rules) / die Arbeitsstationen |

**Transitions / Übergänge** (`stationen` = any own step; each file must contain at least these rules):

| From / Von | To / Nach |
|---|---|
| `offen` | stationen, wartet-auf-entscheidung, auf-anfrage, pausiert, abgebrochen |
| stationen | stationen, wartet-auf-entscheidung, auf-anfrage, in-prüfung, pausiert, abgebrochen, erledigt |
| `wartet-auf-entscheidung`, `auf-anfrage` | stationen, in-prüfung, pausiert, abgebrochen, erledigt |
| `in-prüfung` | stationen, wartet-auf-entscheidung, auf-anfrage, pausiert, abgebrochen, erledigt |
| `pausiert` | offen, stationen, wartet-auf-entscheidung, auf-anfrage, in-prüfung, abgebrochen |
| `abgebrochen`, `erledigt` | offen (reopen), archiviert |
| `archiviert` | none / keine |

Rules: `pausiert` and `abgebrochen` are reachable from every live state (offen, stationen, the waiting and review states; `abgebrochen` also from `pausiert`).
`archiviert` is reached ONLY from `erledigt` or `abgebrochen` and has no way out. / `pausiert` und `abgebrochen` sind von jedem lebenden Zustand
erreichbar; `archiviert` nur von `erledigt`/`abgebrochen`, ohne Ausgang.
