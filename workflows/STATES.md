# States of a run / Zustände eines Laufs

**EN.** A workflow file describes the stations (the `do` steps). A **run** (Lauf) is one pass of an object through a workflow. The run owns its
states, ONCE: the rule set `workflows/run/rules.yaml` is the default (the tables below say the same; a test compares both pair by pair).
No workflow file repeats the states (ADR 0097 E4, ADR 0103 E2/E3). A workflow owns only the rules for its own stations.
`scripts/check_run_rules.py` checks the structure of the rule set (derived from its data), `scripts/check_workflows.py` the files; the tests are
`crates/fs-release/tests/workflows_lauf*.rs`. No build logic lives here.

**DE.** Eine Workflow-Datei beschreibt die Stationen (die `do`-Schritte). Ein **Lauf** ist ein Durchgang eines Objekts durch einen Workflow.
Der Lauf besitzt seine Zustände EINMAL: das Regelwerk `workflows/run/rules.yaml` ist die Voreinstellung (die Tabellen unten sagen dasselbe, ein
Test vergleicht beides Paar für Paar). Keine Workflow-Datei wiederholt die Zustände; ein Workflow besitzt nur die Regeln seiner Stationen.

| State / Zustand | Meaning / Bedeutung |
|---|---|
| `offen` | created, not started (start state) / angelegt, nicht begonnen (Startzustand) |
| `läuft` | the run is at one of the stations of its workflow / der Lauf steht an einer Station |
| `wartet` | waits, ALWAYS with a reason (below) / wartet, IMMER mit Grund (unten) |
| `pausiert` | stopped on purpose, continues at the same station / bewusst angehalten, geht an derselben Station weiter |
| `fehlgeschlagen` | a station failed after its attempts (status command CRITICAL) / eine Station ist nach den Versuchen gescheitert |
| `abgebrochen` | ended without result / ohne Ergebnis beendet |
| `erledigt` | goal reached / Ziel erreicht |
| `archiviert` | closed for good (end state, no way out) / endgültig abgelegt (Endzustand, ohne Ausgang) |

**Wait reasons / Wartegründe** (`reasons` of `wartet`; one column per reason is a display setting, not part of this rule set): `auf-entscheidung`
(a person must decide), `auf-anfrage` (an answer from outside), `in-prüfung` (a review or check is running). A `wartet` without a reason, or with a
reason outside this list, is red (`python3 scripts/check_run_rules.py workflows/run/rules.yaml wartet [reason]`). The word `stationen` is gone
(it is `läuft`). / `wartet` ohne Grund oder mit einem Grund außerhalb der Liste ist rot. Das Wort `stationen` entfällt (es heißt `läuft`).

**Time box / Zeitbox** (`timebox` of the rule set, CLAUDE.md 4a): an order of a run takes at most `max-minutes` (60) and shows visible progress or a clear
blocker every `visible-every-minutes` (60); over the limit the run does `on-exceed` (`stop`, `report`, `workflow-pflege`). One block for the whole run, no
step in every flow; a flow with its own `timeout` longer than `max-minutes` needs `freesynergy.net:timebox-exempt: <reason>` (checked by
`scripts/check_workflows.py`). / Ein Auftrag eines Laufs dauert höchstens `max-minutes` und zeigt alle `visible-every-minutes` Sichtbares oder einen Blocker; darüber
tut der Lauf `on-exceed`. Ein Block für den ganzen Lauf; ein Fluss mit längerem eigenem `timeout` braucht `freesynergy.net:timebox-exempt: <Grund>`.

**Transitions / Übergänge** (the default rules of the run; the workflow adds its station rules):

| From / Von | To / Nach |
|---|---|
| `offen` | läuft, wartet, pausiert, abgebrochen |
| `läuft` | läuft, wartet, pausiert, fehlgeschlagen, abgebrochen, erledigt |
| `wartet` | läuft, wartet, pausiert, abgebrochen, erledigt |
| `pausiert` | offen, läuft, wartet, abgebrochen |
| `fehlgeschlagen` | läuft (repeat), pausiert, abgebrochen, archiviert |
| `abgebrochen`, `erledigt` | offen (reopen), archiviert |
| `archiviert` | none / keine |

Rules (ADR 0103 E2): `pausiert` and `abgebrochen` are reachable from every live state, INCLUDING `fehlgeschlagen`. `archiviert` only from `erledigt`,
`abgebrochen`, `fehlgeschlagen`, and it has no way out. A run reopens from `erledigt` / `abgebrochen` to `offen`. A change of the reason is
`wartet -> wartet`. Open (user decision, ADR 0103 Offen 5, not decided here): whether `fehlgeschlagen -> abgebrochen` is a human act only. /
`pausiert` und `abgebrochen` von jedem lebenden Zustand einschließlich `fehlgeschlagen`; `archiviert` nur von `erledigt`/`abgebrochen`/`fehlgeschlagen`,
ohne Ausgang. Offen: ob `fehlgeschlagen -> abgebrochen` nur ein Mensch darf.

**Rule set name / Name des Regelwerks:** `freesynergy.net:run-rules: freesynergy` in `document.metadata` names the default rule set; any other value is
red (own rule sets follow with fs-flow).

**Station rules / Stationsregeln** (optional, `document.metadata` of a workflow file; PROPOSAL, the form is open until fs-flow): `freesynergy.net:station-rules`
maps a STATION (a step of this file; an unknown name is red) to `allow` (roles that may enter), `from` (optional: the stations it may come from),
`confirm` (optional: true = needs a confirmation) and `on-enter`: a list of actions with a value, e.g. `- set-wait-reason: auf-anfrage` (one of the
reasons), `- assign-role: reviewer`, `- create-decision: <title>`, `- run-status-command: <step>`. A file must not carry `freesynergy.net:states` or
`freesynergy.net:transitions`. The progress mapping to the object (JSCalendar `progress`) is a matter of fs-flow, not of this standard.

**Status command / Status-Befehl** (a step with `freesynergy.net:status-command`, result per Monitoring Plugins Guidelines: exit 0 OK, 1 WARNING, 2 CRITICAL,
3 UNKNOWN): the step needs the Serverless Workflow field `timeout` (`after`, 1 to 3600 s; booleans and negative parts are refused),
`freesynergy.net:origin: local` (only runs of local origin), no shell, no shell syntax (also no line break), no INTERPRETER (perl, python*, ruby, node,
awk, find, xargs, sudo ... run any code, on the command line or on the allow list), and the command is a script under `scripts/` or a program in the
list `freesynergy.net:allow`.
Every entry of `freesynergy.net:allow` is a decision of trust: the interpreter list cannot be complete (it is compared without upper/lower case), and
a program that is not on it may still run foreign code. / Jeder Eintrag von `freesynergy.net:allow` ist eine Vertrauensentscheidung: die Liste der
Interpreter kann nie vollständig sein (Vergleich ohne Groß-/Kleinschreibung), ein Programm, das nicht darauf steht, kann trotzdem fremden Code ausführen.
