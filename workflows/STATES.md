# States of a run / Zustände eines Laufs

**EN.** A workflow file describes the stations (the `do` steps). A **run** (Lauf) is one pass of an object through a workflow. The run owns its
states, ONCE: the rule set `workflows/run/rules.yaml` is the default (the table below says the same; a test compares both pair by pair).
No workflow file repeats the states (ADR 0097 E4: no states wired into every file). A workflow owns only the rules for its own stations.
`scripts/check_run_rules.py` checks the rule set, `scripts/check_workflows.py` the files; the tests are in `crates/fs-release/tests/workflows_lauf*.rs`.
No build logic lives here.

**DE.** Eine Workflow-Datei beschreibt die Stationen (die `do`-Schritte). Ein **Lauf** ist ein Durchgang eines Objekts durch einen Workflow.
Der Lauf besitzt seine Zustände EINMAL: das Regelwerk `workflows/run/rules.yaml` ist die Voreinstellung (die Tabellen unten sagen dasselbe, ein
Test vergleicht beides Paar für Paar). Keine Workflow-Datei wiederholt die Zustände; ein Workflow besitzt nur die Regeln seiner Stationen.

| State / Zustand | Meaning / Bedeutung |
|---|---|
| `offen` | created, not started (start state) / angelegt, nicht begonnen (Startzustand) |
| `wartet-auf-entscheidung` | waits (reason `entscheidung`): a person must decide / wartet: ein Mensch muss entscheiden |
| `auf-anfrage` | waits (reason `anfrage`): an answer from outside / wartet: Antwort von außen |
| `in-prüfung` | waits (reason `prüfung`): a review or check is running / wartet: Prüfung oder Review läuft |
| `pausiert` | stopped on purpose, may continue at the same station / bewusst angehalten, geht an derselben Station weiter |
| `fehlgeschlagen` | a station failed after its attempts (status command CRITICAL) / eine Station ist nach den Versuchen gescheitert |
| `abgebrochen` | ended without result / ohne Ergebnis beendet |
| `erledigt` | goal reached / Ziel erreicht |
| `archiviert` | closed for good (end state) / endgültig abgelegt (Endzustand) |

`stationen` is NOT a state: it is the word for "the run is on one of the stations of its workflow". The three waiting states are one waiting
state with a reason (`wait-reasons` in the rule set); the words stay for the display. / `stationen` ist KEIN Zustand, sondern das Wort für
„der Lauf steht an einer Station“. Die drei Wartezustände sind ein Warten mit Grund; die Wörter bleiben für die Anzeige.

**Transitions / Übergänge** (the default rules of the run; the workflow adds its station rules):

| From / Von | To / Nach |
|---|---|
| `offen` | stationen, wartet-auf-entscheidung, auf-anfrage, pausiert, abgebrochen |
| stationen | stationen, wartet-auf-entscheidung, auf-anfrage, in-prüfung, pausiert, fehlgeschlagen, abgebrochen, erledigt |
| `wartet-auf-entscheidung`, `auf-anfrage` | stationen, in-prüfung, pausiert, abgebrochen, erledigt |
| `in-prüfung` | stationen, wartet-auf-entscheidung, auf-anfrage, pausiert, abgebrochen, erledigt |
| `pausiert` | offen, stationen, wartet-auf-entscheidung, auf-anfrage, in-prüfung, abgebrochen |
| `fehlgeschlagen` | stationen (repeat), abgebrochen, archiviert |
| `abgebrochen`, `erledigt` | offen (reopen), archiviert |
| `archiviert` | none / keine |

Rules: `abgebrochen` is reachable from every live state (also `fehlgeschlagen` and `pausiert`), `pausiert` from every live state except
`fehlgeschlagen`. `archiviert` only from `erledigt`, `abgebrochen`, `fehlgeschlagen`, and it has no way out. A run reopens from `erledigt` /
`abgebrochen` to `offen`. Open question (not decided here): whether `fehlgeschlagen -> abgebrochen` is a human act only. /
`abgebrochen` von jedem lebenden Zustand, `pausiert` von jedem außer `fehlgeschlagen`; `archiviert` nur von `erledigt`/`abgebrochen`/`fehlgeschlagen`, ohne
Ausgang. Offene Frage: ob `fehlgeschlagen -> abgebrochen` nur ein Mensch darf.

**Station rules / Stationsregeln** (optional, in `document.metadata` of a workflow file): `freesynergy.net:station-rules` maps a STATION (a step of
this file; an unknown name is red) to `allow` (roles that may enter) and `on-enter` (a list of `assign-role`, `run-status-command`,
`create-decision`, `set-wait-reason`). A file must not carry `freesynergy.net:states` or `freesynergy.net:transitions`.

**Status command / Status-Befehl** (a step with `freesynergy.net:status-command`, result per Monitoring Plugins Guidelines: exit 0 OK, 1 WARNING, 2 CRITICAL,
3 UNKNOWN): the step needs the Serverless Workflow field `timeout` (`after`, 1 to 3600 s), `freesynergy.net:origin: local` (only runs of local
origin), no shell and no shell syntax, and the command is a script under `scripts/` or a program in the list `freesynergy.net:allow`.
