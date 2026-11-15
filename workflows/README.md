# workflows/

**DE.** Hier stehen unsere Abläufe als sichtbare Workflows: Serverless-Workflow-1.0-YAML (`dsl: 1.0.x`) im FS-Profil
(Entwurf `ENTWURF-workflow-standard.md`, Fassung 3). Jeder Schritt trägt `freesynergy.net:actor` (`human` = Person, `ai` =
Application, `machine` = Service; dazu `freesynergy.net:actor-type`) und `freesynergy.net:gate` (Freigabe-Haltepunkt, z. B.
`opus-ja`, `human`, oder `none`) und `freesynergy.net:kind` (`objective` = wiederholen, bis das Ziel erreicht ist, z. B.
Tests grün; `subjective` = Zustand ändern, egal wie das Ergebnis ist, z. B. Bilder zeigen, Urteil einholen). Die Dateien
sind 1:1 in fs-workflows importierbar: Serverless Workflow 1.0 plus FS-Profil, keine Sonderformen. Alle tragen
`document.version` = Crate-Version = Tag (0.0.6). Agenten pushen, taggen und mergen nie. Heute sind sie Beschreibung und
werden nur geprüft (`scripts/check_workflows.py`, Test `crates/fs-release/tests/workflows.rs`; Schrittnamen von `lift` und
`flow` müssen mit dem Code übereinstimmen). In einem zweiten Schritt liest `fs-release flow` diese Dateien und führt den
Ablauf daraus aus. Keine Geheimnisse in diese Dateien.

**Regel: alle Arbeitsschritte laufen über einen Workflow.** Gibt es für einen Schritt keinen, ist der erste Schritt, den
Workflow zu schreiben oder zu erweitern (`workflow-pflege`). Ein Agentenauftrag verweist auf den Flow so:
`Workflow <name>, Schritt <schritt>, Parameter <schlüssel=wert, ...>`, z. B. `Workflow bugfix, Schritt reproduce, Parameter
finding="..."`. Der Agent arbeitet nur die genannten Schritte ab und meldet im Bericht den Flow und die Schritte. Der Auftrag
bleibt kompakt: `Workflow <name>, Schritt <id>, Parameter repo, branch, head, delta, Auftragsinhalt`; Gates, Bash-Regeln,
Berichtsform und Prüfliste stehen im `summary` der Schritte und in `rules.md` (`[GATES]`, `[BASH]`, `[COMMIT]`, `[BERICHT]`,
`[REVIEW]`), nicht im Auftrag (`docs/prompt-template.md`, Messung `scripts/measure-prompts.py`). Ausgaben früherer Schritte
gehen über `export.as` und werden mit `$context.<name>` gelesen; ein Rücksprung ist ein `switch` mit `then`. **`rules.md`**
liegt unter `/srv/FreeSynergy/.scratch/api-runner/rules.md`; diese Datei gilt, im Repo gibt es keine Kopie. Die Regeln
`[EFFIZIENZ]` (ein Lauf = ein Gate-Zyklus, Gates einmal je Kopf, Reviews nur Delta, Mechanik als Skript, Abbruch-Schutz) stehen
als Schritt-Regeln in `agent-run` und `review`. Vor jeder Entscheidung sucht `scripts/adr-search` die vorhandenen ADRs
(`entscheidung`, `user-decision`).

**Die Flows.**
- Release und Bau: `release-flow` (lift, gates, Review, Tag, Push, CI, ff main, Release, bump, sign), `lift`, `catalog-bump`,
  `cleanup-targets`, `agent-run`, `bugfix`, `review`.
- Frontend und Bilder: `frontend-round` (mit dem objektiven Schritt `menue-vergleich`, s. u.), `frontend-umbau` (Bestand auf die Schichten nach ADR 0074), `dossier`, `icon-round`,
  `picture-acceptance`.
- Neues Programm: `programm-neu` (17 Schritte in P0 bis P6, s. u.), `programm-grundgeruest` (Fließband für die Grundsachen jedes Programms, s. u.).
- Fremdes: `upstream-anfrage` (ein git-bug-Issue an einen Upstream), `upstream-update` (neuer Upstream-Release in Spiegel und Katalog, Sicherheitsfixes zuerst; der Spiegel-Bau erzeugt nur einen Entwurf, die Veröffentlichung ist ein eigener Schritt mit Freigabe; fs-kanidm hat bisher kein Release und keinen Katalog-Eintrag, der erste Lauf legt beides an), `wrapper-einbindung` (fremde Crate per Git-Pin
  unverändert, eigene Wrapper-Crate, Lizenz-Gate in der CI).
- Server: `server-einrichtung` (Weg A: unsere Schicht auf ein vorhandenes System mit SSH; Bestand, Trockenlauf, Gate, Schicht,
  Absicherung, Dienste, Prüfung; der Mensch führt `fs-server-run` lokal aus, ohne KI; B kexec, D Plattenumbau sind eigene Workflows),
  `anbieter-neuinstallation` (Weg C: Hetzner-Rescue und `installimage` mit Konfig aus Variablen durch `scripts/installimage-config`; Gate "Daten werden gelöscht" mit Hostname-Wiederholung; der Mensch führt Rescue und Install aus; danach Übergabe an `server-einrichtung`; Montagsablauf in `docs/anbieter-neuinstallation.md`).
- Umbenennen und Doku: `umbenennen` (Repo, Crates `fs-<x>-*`, Katalog mit `replaces`, Doku, Pins; der alte Name wird nie wieder vergeben, ADR 0080; Trockenlauf ohne KI, GitHub-Umbenennung nur nach dem Freigabe-Gate `strategie-ok`), `doku-mitziehen` (Pflichtschritt, s. u.).
- Koordinator: `koordinator-runde`, `testlauf`, `coordination-round`, `cost-reconcile`, `report-to-strategy`, `slot-check`, `wip-continue`, `download-readonly`,
  `publish-doc`, `workflow-pflege`.
- Strategie: `entscheidung` (erst ADR-Suche, dann Auswertung, Nutzerfrage nur bei Abweichung oder Lücke), `adr-write`,
  `user-decision`, `handoff-update`, `restart-handoff`.

**Koordinator-Runde, Grundgerüst, Testlauf (Bündel 0.0.7).** `koordinator-runde`: Start mit `handoffs-lesen` (fs-documentation `todo/handoff-*.md`), `posteingang-lesen`, `rot-pruefen`, `straenge` (Stränge und Prioritäten in eine Arbeitsliste); `agenten-starten` startet je Arbeitsschritt einen Agenten im Hintergrund (Sonnet baut und testet, Opus prüft, Haiku sucht und schreibt nie Code), ihre Meldungen gehen an den Koordinator (`report_to: coordinator`). Grund (verbindliche Anweisung der Strategie vom 15.11. abends): der Koordinator läuft in einer EIGENEN Sitzung und baut, testet und wartet selbst nicht; die Regel „Agenten im Vordergrund“ galt nur für Koordinatoren, die als Unteragent innerhalb der Strategie-Sitzung liefen (dort landeten Hintergrundmeldungen bei der Strategie). Der Koordinator endet erst, wenn ALLE Agenten gemeldet haben: `einsammeln` wartet ereignisgesteuert (`wait: all`, ohne `sleep`/`pgrep`, nur Bericht und Commit-Liste), `einsammeln-weiche` schickt eine unvollständige Runde in die `eskalation`; `auswerten`; `gates-pruefen` (über `testlauf`), rot geht über `korrektur` (ein Agent über `agent-run`, nie `bugfix`, das im `release-flow` endet) zurück, ab der dritten roten Runde `eskalation`; `doku` (`doku-mitziehen`, `doku-weiter` stoppt bei Eskalation), `pruefung` (je Branch `resolve-head` NACH Gates und Doku, Opus-JA für genau diesen Kopf, NEIN geht über `korrektur` zurück); jedes Ergebnis ist nur bei `red == false` bzw. `status == green` grün, ein fehlendes Ergebnis nie; `eskalation` läuft noch durch `sichern`, `push-unreviewed`, `handoff` und `bericht`; `regel-check` (Skript, ROT wenn der Koordinator selbst gebaut oder getestet, in Schleifen gewartet oder große Logs gelesen hat); Ende: `sichern` (WIP-Commit), `push-unreviewed` (Mensch, nur `unreviewed/*`, nie main, nie Tag), `handoff` (`handoff-update`), genau ein `bericht` (`report-to-strategy`: nur Endstände, Rot, Freigabe-Gates). `programm-grundgeruest` (ADR 0087 Nr. 7: starten, einfache Dinge, beenden): Stufe 1 parallel (`dossier-doku` und `recherche` durch Haiku, nur lesen und Notizen schreiben) mit Skript-Prüfung; Stufe 2 `plan` (Opus, FESTER Trait), `plan-check`, `trait-sperre` (SHA-256 in eine schreibgeschützte Datei in `lock_dir` außerhalb jedes Agenten-Worktrees und im Ablauf), danach Gate `strategie-ok` (`listen` mit Subject = dieser Hash); Stufe 3 parallel (Sonnet: `kern` mit Trait, typisierter Speicherung nach ADR 0086 und CLI; `frontend` mit zuerst Baustein-Zuordnung und Testdaten gegen den Trait), `trait-unveraendert` (Vergleich mit `expected` aus dem Ablauf UND der Sperrdatei; geänderter Trait = zurück zu `plan`, neue Sperre und neues `strategie-ok`), `gates` mit `gates-again` (rote Tests gehen zurück), `speicher-check` (keine JSON-/Blob-Spalte außer `original`), `lauf-check`; Stufe 4 `doku` (mit `doku-weiter`), `kopf` (`resolve-head`), `review` (Opus, für genau diesen Kopf, NEIN zurück zu `stufe-3`), `bilder` (Bilder-Gate); `output` `status`/`eskaliert`. Verhältnis zu `programm-neu` (Entscheidung nach ADR 0087 Nr. 7): ersetzt nichts; `programm-neu` bleibt der einzige Einstieg für NEUE Programme, `programm-grundgeruest` ist das Band für die Grundsachen jedes Programms (auch bestehender) und wird von `programm-neu` in einem späteren Bündel als Bauteil für die Schritte nach der Schnittstelle aufgerufen (in 0.0.7 nicht verdrahtet). `testlauf`: `geaendert` (Skript: `-p` und `--test` der geänderten Teile), `slot` (`build-slot.sh`, höchstens 3), `gezielt`, rot über `fix` zurück, der Workspace-Volllauf nur bei `scope: end` als End-Gate (`voll`, im Hintergrund, nur Zusammenfassung gelesen); `voll-ende` wartet auf das Ereignis `net.freesynergy.test.done` mit dem Kopf als Subject, `zeit` (Zeitmessung); `output` `status`/`red`/`runde`/`eskaliert`/`head`, nur `red == false` ist grün. Nicht belegt: alle neuen `@fs`-Namen, ein echter SW-1.0-Lauf.

**Menü-Vergleich (R2, Nutzerentscheidung 14.11.).** Menüs, Dialoge und Abläufe eines Frontends müssen denen des Vorbilds
gleich oder ähnlich sein (Erweiterungen später), nicht nur die Bilder. Der Schritt `menue-vergleich` in `frontend-round` (nach
`gates`, vor `acceptance`; rot springt zu `build`) lässt `scripts/menue-vergleich` (Python, ohne Netz, ohne KI) den Menübaum
und die Dialogliste des Programms (Dump `fs-menu-dump/1`) mit `## Menü-Inventar` / Dialogtabelle des Dossiers vergleichen.
Abweichungen (`fehlt`, `zusaetzlich`, `anders-benannt`, `kuerzel`) sind nur mit Begründung in der Ausnahmeliste des Frontends
erlaubt; ohne Begründung ist das Ergebnis rot. Gilt für alle Frontends der Welle 1. Format: `docs/menue-vergleich.md`,
Selbsttest `scripts/test_menue_vergleich.sh`.

**Stil-Tor der Theme-Brücke (Nutzerregeln zu ADR 0074, Plan `/srv/FreeSynergy/.scratch/plan-theme-bruecke.md`).** In `frontend-umbau` laufen vor `convert` (dem Ersetzen gemalter Nachbildungen durch Blinc-Bausteine) `bruecke`, `alle-stile-erweitern`, `stil-vollstaendigkeit` (Skript: Theme-Vertrag x alle Stile, fehlender Eintrag = rot, kein Release) und `zwei-stile` (Skript: jeder Baustein in zwei Stilen gezeichnet, unverändertes Bild = rot, Ausnahmen nur für Inhalt). Reihenfolge: Vertrag, Vollständigkeit, zwei-stile. Jeder Stil gestaltet jeden Baustein mit allen Teilen und Zuständen; ein neuer Baustein, Teil oder Zustand = neue Vertragsversion und der feste Schritt `alle-stile-erweitern`. In `frontend-round` sichert `stil-sichern` den Stil des nachgebauten Vorbilds (englischer Name, Anlehnung und Lizenz benannt, nichts kopiert), danach `stil-vollstaendigkeit`. Die Skripte sind noch nicht gebaut.

**Baustein-Zuordnung (Nutzerentscheidung 14.11., für alle Frontends).** Verbindliche Reihenfolge: (1) Baustein-Analyse - jeder
Bereich des Vorbilds wird einer Baustein-Familie und einer Baustein-Id zugeordnet („was ist das?“: Tabelle, Baum, Liste, Karte,
Leiste, Dialog …), vorhandene Bausteine zuerst, ein neuer nur mit Begründung, warum kein vorhandener (mit Modul) passt; (2) Aussehen -
Stil-Analyse, Tokens und Stilregeln je Baustein (ADR 0074); (3) Zusammensetzen - die Anordnung im Frontend. Beispiel: die Datei-Tabelle
in fs-files ist dieselbe Tabelle wie später in fs-data, nur mit ausgeblendeten Köpfen und ohne Rechen-Modul (Formeln = Modul, Muster
von ADR 0084). Der objektive Prüfschritt `baustein-zuordnung` (in `dossier`, vor dem Veröffentlichen; rot springt zu `research`) lässt
`scripts/baustein-zuordnung` die Tabelle `## Baustein-Zuordnung` in `bausteine.md` prüfen und zählt die roten Runden selbst (Zustandsdatei im Dossier-Ordner, rc 3 ab der dritten) und meldet den Anteil je Quelle
(Familie, Blinc-Widget, neu; Reihenfolge Familie, Blinc, neu mit Begründung). Format: `docs/baustein-zuordnung.md`, Selbsttest `scripts/test_baustein_zuordnung.sh`.

**Neues Programm (`programm-neu`, Nutzerwunsch 14.11.).** Ein Ablauf von der Prüffrage bis zur Abnahme, 17 Schritte in sieben Phasen:
P0 (1 „wessen Daten?“, ADR 0076, Ausgang Programm / Facette / Modul / Suite; 2 Hub und Katalog, ADR 0068), P1 (3 Standards und ihre
Testsammlungen, ADR 0014; 4 Kern-Crate, Wrapper statt Fork; 5 Formate festlegen (Zeiten UTC plus Zonen-Id, Personen als Verweis, ADR 0085 Entwurf) und Umzugsweg), P2 (6 Datenmodell
auf Standards, Speichern ist nicht Exportieren, ADR 0046; 7 Themen-Schnittstelle, ADR 0045/0066; 8 Manifest, ADR 0068), P3 (9 Engine
mit Vertragstests; 9a Importer/Exporter als Grenz-Adapter nach ADR 0085 (Entwurf); 10 Admin-Frontend aus der Typbeschreibung), P4 (11 Vorbild-Dossier; 12 `frontend-round` mit Baustein-Zuordnung,
Stil, Zusammensetzen, `menue-vergleich`; 13 Sprache MF2 de+en und Barrierefreiheit), P5 (14 Hub-Anmeldung und `fs-server`-Adapter
nach ADR 0082; 15 Sicherheit, ADR 0054/0056/0067; 16 Pakete, Katalog ADR 0077, Doku; danach `release-flow`, danach der Serverlauf), P6 (17 Abnahme durch den
Nutzer, Befunde laufen durch `bugfix`). Jeder Agentenschritt (`objective`) hat danach einen objektiven Skript-Prüfschritt; rot
springt per `switch`/`then` zurück, höchstens zweimal: die dritte rote Runde (`runde` im Ergebnis des Prüfskripts, bei Grün innerhalb eines Kreises nicht zurückgesetzt) endet in `eskalation` beim Menschen. `frontend-round` läuft hier mit `release: false` (ein einziges Release, erst nach Sprache, Sicherheit und Paketen). Jeder Schritt nennt seine ADRs in `freesynergy.net:adr` und seine Phase in
`freesynergy.net:phase`. Ausgang Facette, Modul oder Suite verlässt den Ablauf an einem Stub (`facette-abzweig`, `modul-abzweig`,
`suite-abzweig`); **`facette-anlegen` und ein Modul-Ablauf gibt es noch nicht** und sind als TODO geführt, nicht erfunden. Auf echten
Servern arbeitet kein Agent: Schritt 14 läuft nur lokal gegen Testinstanzen, den echten Lauf macht der Mensch (`server-lauf`). Selbsttest:
`crates/fs-release/tests/workflows_programm_neu.rs` (Gegenproben: Schritt ohne `kind`, ohne ADR-Verweis, ohne Prüfschritt, Sprung ins Leere; Strukturregeln: kein toter Schritt, Rücksprung nur auf Agent oder Mensch, Eskalation nach drei roten Runden, Reihenfolge Sicherheit, Pakete, Release, Serverlauf, Eingaben der Unterabläufe vollständig).

**Regel: die Doku zieht mit (`doku-mitziehen`).** Jede Änderung an Code, Namen, Schnittstellen oder Verhalten zieht im selben Lauf die Doku in fs-documentation mit (de und en zusammen: Seiten, Links, ADR-Statuszeilen, Beispiele). Der Unterablauf `doku-mitziehen` sucht per grep die Fundstellen (`suchen`), der Autor der Änderung schreibt sie nach (`schreiben`), `check-docs` und das Skript `scripts/doku-mitziehen` (Python, ohne Netz, ohne KI) prüfen: Code geändert, Doku nicht = **rot**; nur eine der beiden Sprachen geändert, alter Name übrig, neuer Name fehlt, kaputter relativer Link oder `mkdocs build --strict` rot = rot; ist mkdocs nicht lauffähig, ist das Ergebnis `skipped` und nie grün. Rote Runden werden gezählt, die dritte endet beim Menschen. Der Unterablauf veröffentlicht nicht: das Veröffentlichen in den Doku-Hauptzweig (`publish-doc`, Mensch/Koordinator) ist der Schritt `doku-veroeffentlichen` des aufrufenden Ablaufs, erst nach Review und Freigabe (und nach dem Merge des Codes). Ergebnis des Unterablaufs: `status` (green | red | skipped) und `eskaliert`; nur `green` geht weiter (`doku-weiter` im Aufrufer), `skipped` ist nie grün. Pflichtschritt `doku` steht in `release-flow` (zwischen `lift` und `gates`; damit auch für `lift`, `frontend-round`, `frontend-umbau` und `programm-neu`, das keinen eigenen `doku` mehr hat), `upstream-update` (vor `head`) und `umbenennen`. Selbsttest `scripts/test_doku_mitziehen.sh`.

**Umbenennen (`umbenennen`, ADR 0080).** Ablauf: `trockenlauf` (`scripts/umbenennen dry-run`: Crates, CI-Dateien, Pins, Katalogeintrag, Doku-Treffer; rot bei gesperrtem oder vorhandenem Namen) mit Gate `strategie-ok` für den Plan, `umbau` (Agent, Branch-Worktree), `sperre` (alter Name in `scripts/retired-names.txt`, wird nie wieder vergeben), `doku` (`doku-mitziehen`), `nachher` (`scripts/umbenennen after`: kein alter Name mehr, `replaces` im Katalog), `gates`, `review` (Opus-JA für genau diesen Kopf-Hash), Gate `freigabe` (`strategie-ok`), erst danach die GitHub-Umbenennung durch Mensch/Koordinator, `weiterleitung` (nur lesen), `release` über `release-flow`. Kein Agent benennt ein Repo um. Nicht belegt: Weiterleitung der Release-Dateien nach der Umbenennung (ADR 0080 offen). Selbsttest `scripts/test_umbenennen.sh`.

**EN.** Our processes as visible workflows: Serverless Workflow 1.0 YAML (`dsl: 1.0.x`) in the FS profile (draft
`ENTWURF-workflow-standard.md`, version 3). Every step carries `freesynergy.net:actor` (`human` = Person, `ai` =
Application, `machine` = Service; plus `freesynergy.net:actor-type`) and `freesynergy.net:gate` (approval stop, e.g.
`opus-ja`, `human`, or `none`) and `freesynergy.net:kind` (`objective` = repeat until the goal is reached, e.g. tests
green; `subjective` = change the state whatever the result, e.g. show images, ask for a verdict). The files import 1:1 into
fs-workflows: Serverless Workflow 1.0 plus the FS profile, no special forms. All carry `document.version` = crate version
= tag (0.0.6). Agents never push, tag or merge. For now they are a description and are only checked
(`scripts/check_workflows.py`, test `crates/fs-release/tests/workflows.rs`; the step names of `lift` and `flow` must match
the code). In a second step `fs-release flow` will read these files and run the process from them. No secrets in these
files.

**Rule: every work step runs through a workflow.** If a step has none, the first step is to write or extend it
(`workflow-pflege`). An agent order points to a flow like this: `Workflow <name>, step <step>, parameters <key=value, ...>`,
e.g. `Workflow bugfix, step reproduce, parameters finding="..."`. The agent works only the named steps and names flow and
steps in its report. The list of flows is above (same names). The order stays compact: `Workflow <name>, step <id>, parameters
repo, branch, head, delta, content`; gates, bash rules, report form and check list are in the `summary` of the steps and in
`rules.md` (`[GATES]`, `[BASH]`, `[COMMIT]`, `[BERICHT]`, `[REVIEW]`), not in the order (`docs/prompt-template.md`, measured by
`scripts/measure-prompts.py`). Outputs of earlier steps go through `export.as` and are read as `$context.<name>`; a jump back
is a `switch` with `then`. **`rules.md`** is the file `/srv/FreeSynergy/.scratch/api-runner/rules.md`; that one applies, there
is no copy in the repo. The `[EFFIZIENZ]` rules (one run = one gate cycle, gates once per head, reviews read only the delta,
mechanics as a script, abort protection) are step rules in `agent-run` and `review`. Before any decision `scripts/adr-search`
looks up the existing ADRs (`entscheidung`, `user-decision`).

## Events / Ereignisse

**DE.** Workflows und CI-Läufe wecken einander über Ereignisse nach CloudEvents (Pflichtfelder `id`, `source`, `specversion`,
`type`). Hülle: `type` = `net.freesynergy.<gegenstand>.<ereignis>`; `source` = Repo-URL des Senders; `subject` = Tag oder Name;
`time` RFC 3339; `data` klein (Namen, Zahlen, Tags), nie Geheimnisse, nie Befehle. Ereignisse sind Weckrufe, die Entscheidung steht
im Repo-Zustand, nicht im Ereignis. **Grundregel: keine Freigabe-Umgehung. Automatisch laufen nur Folgen bereits freigegebener
Schritte** (Gate `none` oder schon erfüllt); kein Ereignis löst Tag, Push, Veröffentlichung, ADR-Push oder Löschen aus. Der
Zeitplan von `bump.yml` (alle 15 Minuten) bleibt als Sicherheitsnetz; es gibt keinen Token zwischen Repos (Pull, Q2b in DESIGN.md).

**EN.** Workflows and CI runs wake each other by events in the CloudEvents form (required `id`, `source`, `specversion`, `type`).
Envelope: `type` = `net.freesynergy.<subject>.<event>`; `source` = repo URL of the sender; `subject` = tag or name; `time`
RFC 3339; `data` small (names, numbers, tags), never secrets, never commands. Events are wake-up calls; the decision is in the
repo state, not in the event. **Basic rule: no bypass of an approval. Only follow-ups of steps that are already approved run
automatically** (gate `none` or already met); no event triggers a tag, push, publication, ADR push or deletion. The 15-minute
schedule of `bump.yml` stays as the safety net; there is no token between repos (pull, Q2b in DESIGN.md).

| type | sender | receiver | data | approval rule |
|---|---|---|---|---|
| `net.freesynergy.release.published` | last job of `release.yml` in the program repo, after `publish` was green (a draft sends nothing); `release-flow` step `announce` | `catalog-bump` (`schedule.on`), `release-flow` step `release` (`listen`) | `repo`, `tag`, `prerelease` (yes/no) | automatic: follow-up of the approved push (the opus-ja and picture OK lay before the tag) |
| `net.freesynergy.catalog.signed` | `sign.yml` of fs-catalog after the push | `release-flow` step `sign` (`listen`) | `commit` | automatic |
| `net.freesynergy.catalog.tested` | check like a user (a task in fs-catalog: in the same `sign.yml` run, because a push with the run token starts no other workflow) | `release-flow` step `sign` | `ok` | automatic; `ok` false is red |
| `net.freesynergy.catalog.changed` | candidate: `sign.yml` after a green check | candidate: rebuild of fs-packages (pull by schedule, later) | `commit` | automatic only if nothing new becomes public; the first publication of a package repo is a gate |
| `net.freesynergy.text.published` | candidate: text/docs repo on a push to main | candidate: docs build | `repo`, `commit` | build into the preview automatic; going public only with an approved source |
| `net.freesynergy.run.failed` | any workflow when red (`_notify-red.yml`, issue `rot: <workflow> <repo>`, label `rot`; closes at the next green) | the coordinator, by one query: `gh search issues --owner FreeSynergy --label rot --state open` | `workflow`, `repo`, `run_url`, `step` | always allowed (a notice only); there is no status board |
| `net.freesynergy.approval` | the Opus approval file | `release-flow` step `review` (`listen`) | head | stays a gate |
| `net.freesynergy.test.done` | the end of the background full workspace run of `testlauf` (the wrapper around `build-slot.sh cargo test --workspace`) | `testlauf` step `voll-ende` (`listen`, subject = the head) | `red`, `runde`, `seconds`, `summary` (only the summary line) | not a gate, only the wait; a missing or foreign event is never green |
| `net.freesynergy.ci.green` | CI of the pushed branch | `release-flow` step `ci` (`listen`) | head | not a gate, only the wait |

**Mapping to Serverless Workflow 1.0 / what is proven.** Send = `emit.event.with` (`source`, `type`, `subject`, `data`; `id` is made by the
runtime) - in the spec (dsl-reference, `emit`), also in v1.0.0. Start by event = top-level `schedule.on` (`one`/`any` with `with.type`) -
in the spec. Wait in the flow = `listen.to` (`one`/`any`/`all`) - in the spec. NOT proven: `schedule.read` in 1.0.x (not set, the default
`data` is used), and whether `every` and `on` may stand together (so the 15-minute net is written in the comment of `catalog-bump.yaml`,
not as a second trigger). The spec defines no transport: how an event really arrives is the implementation. Today: `release.published`
reaches fs-catalog as the schedule of `bump.yml` (pull), `run.failed` as an issue; `listen` steps only describe the wait (the flow stops,
the continuation is a new run). The check (`scripts/check_workflows.py`) requires that every type is `net.freesynergy.*` and is in
this table, and that `coordination-round` starts with `rot-pruefen`.

**Rule: the coordinator acts only on red or at approval gates.** Silence on green. Follow-ups after an approval (bump, sign, catalog
check, package repos, docs build) are never started by hand; they run by event/CI. Red = `bugfix` or `agent-run` for exactly that run.

**Gates that never run automatically / Gates, die nie automatisch laufen:** publication (tag, Pre-release, release), the tag push
itself, the Opus approval (`opus-ja`) for exactly one head, fast-forward of main (`human`), ADR push, deleting (repos, tags, releases,
files in foreign repos), the Immutable-Releases switch, making a new package repo public for the first time, creating or rotating
secrets and tokens, any deviation from the rules. DE: Veröffentlichung, Tag-Push, Opus-JA für genau einen Kopf, ff main, ADR-Push,
Löschen, Immutable-Schalter, erstes Veröffentlichen eines Paketrepos, Geheimnisse anlegen/drehen, jede Regelabweichung.

The `@fs` catalog (`call: <command>:<version>@fs`) is the fixed list of commands of the draft; it is not defined as a
`use.catalogs` entry yet, so these files are valid YAML in the profile's shape but not runnable by a stock SW runtime.

**Rule: the docs move along (`doku-mitziehen`).** Every change of code, names, interfaces or behaviour pulls the documentation in fs-documentation along in the same run (de and en together: pages, links, ADR status lines, examples). The sub-workflow `doku-mitziehen` finds the places by grep (`suchen`), the author of the change updates them (`schreiben`), `check-docs` and the script `scripts/doku-mitziehen` (Python, no network, no AI) check: code changed, docs not = **red**; only one of the two languages changed, an old name left, a new name missing, a broken relative link or a red `mkdocs build --strict` = red; when mkdocs cannot run the result is `skipped`, never green. Red rounds are counted, the third ends with the person. The sub-workflow does not publish: pushing the docs main branch (`publish-doc`, human/coordinator) is the step `doku-veroeffentlichen` of the calling flow, only after review and approval (and after the merge of the code). Result of the sub-workflow: `status` (green | red | skipped) and `eskaliert`; only `green` goes on (`doku-weiter` in the caller), `skipped` is never green. The mandatory step `doku` is in `release-flow` (between `lift` and `gates`, so also for `lift`, `frontend-round`, `frontend-umbau` and `programm-neu`, which no longer has an own `doku`), `upstream-update` (before `head`) and `umbenennen`. Self test `scripts/test_doku_mitziehen.sh`.

**Rename (`umbenennen`, ADR 0080).** Order: `trockenlauf` (`scripts/umbenennen dry-run`: crates, CI files, pins, catalog entry, docs hits; red for a retired or existing name) with the gate `strategie-ok` for the plan, `umbau` (agent, branch worktree), `sperre` (old name into `scripts/retired-names.txt`, never given out again), `doku` (`doku-mitziehen`), `nachher` (`scripts/umbenennen after`: no old name left, `replaces` in the catalog), `gates`, `review` (Opus YES for exactly this head hash), gate `freigabe` (`strategie-ok`), only then the GitHub rename by the human/coordinator, `weiterleitung` (read only), `release` through `release-flow`. No agent renames a repo. Not proven: redirect of release assets after the rename (ADR 0080 open). Self test `scripts/test_umbenennen.sh`.

**Coordinator round, basics belt, test run (bundle 0.0.7).** `koordinator-runde`: it starts with `handoffs-lesen` (fs-documentation `todo/handoff-*.md`), `posteingang-lesen`, `rot-pruefen`, `straenge` (strands and priorities into one work list); `agenten-starten` starts one agent per work step in the background (Sonnet builds and tests, Opus reviews, Haiku searches and never writes code), their messages go to the coordinator (`report_to: coordinator`). Reason (binding instruction of the strategy, 15 Nov evening): the coordinator runs in its OWN session and builds, tests and waits in loops itself never; the rule "agents in the foreground" applied only to coordinators that ran as a sub-agent inside the strategy session (there background messages landed at the strategy). The coordinator ends only when ALL agents have reported: `einsammeln` waits event driven (`wait: all`, no `sleep`/`pgrep`, only the report and the commit list), `einsammeln-weiche` sends an incomplete round to `eskalation`; `auswerten`; `gates-pruefen` (through `testlauf`), red goes back through `korrektur` (one agent through `agent-run`, never `bugfix`, which ends in `release-flow`), from the third red round `eskalation`; `doku` (`doku-mitziehen`, `doku-weiter` stops on an escalation), `pruefung` (per branch `resolve-head` AFTER gates and docs, Opus YES for exactly this head, a NO goes back through `korrektur`); every result is green only with `red == false` or `status == green`, a missing result never; `eskalation` still goes through `sichern`, `push-unreviewed`, `handoff` and `bericht`; `regel-check` (script, RED when the coordinator built or tested itself, waited in loops or read big logs); end: `sichern` (WIP commit), `push-unreviewed` (the person, only `unreviewed/*`, never main, never a tag), `handoff` (`handoff-update`), exactly one `bericht` (`report-to-strategy`: only final states, red, approval gates). `programm-grundgeruest` (ADR 0087 item 7: start, simple things, end): stage 1 in parallel (`dossier-doku` and `recherche` by Haiku, read and write notes only) with a script check; stage 2 `plan` (Opus, FIXED trait), `plan-check`, `trait-sperre` (SHA-256 into a read-only file in `lock_dir`, outside every agent worktree, and into the flow), then the gate `strategie-ok` (`listen` with the subject = this hash); stage 3 in parallel (Sonnet: `kern` with the trait, typed storage after ADR 0086 and CLI; `frontend` with the block assignment first and test data against the trait), `trait-unveraendert` (compares with `expected` from the flow AND the lock file; a changed trait goes back to `plan`, a new lock and a new `strategie-ok`), `gates` with `gates-again` (red tests go back), `speicher-check` (no JSON or blob column except `original`), `lauf-check`; stage 4 `doku` (with `doku-weiter`), `kopf` (`resolve-head`), `review` (Opus, for exactly this head, a NO goes back to `stufe-3`), `bilder` (picture gate); `output` `status`/`eskaliert`. Relation to `programm-neu` (decision after ADR 0087 item 7): it replaces nothing; `programm-neu` stays the only entry for NEW programs, `programm-grundgeruest` is the belt for the basics of every program (existing ones too) and will be called by `programm-neu` as its component for the steps after the interface in a later bundle (not wired in 0.0.7). `testlauf`: `geaendert` (script: `-p` and `--test` of the changed parts), `slot` (`build-slot.sh`, at most 3), `gezielt`, red goes back through `fix`, the full workspace run only with `scope: end` as an end gate (`voll`, in the background, only the summary is read); `voll-ende` waits for the event `net.freesynergy.test.done` with the head as subject, `zeit` (timing); `output` `status`/`red`/`runde`/`eskaliert`/`head`, only `red == false` is green. Not proven: all new `@fs` names, a real SW 1.0 run.

**Menu comparison (R2, user decision 14 Nov).** Menus, dialogs and flows of a Frontend must be equal or similar to the model's
(extensions later), not only the pictures. The step `menue-vergleich` in `frontend-round` (after `gates`, before `acceptance`;
red jumps to `build`) lets `scripts/menue-vergleich` (Python, no network, no AI) compare the program's menu tree and dialog
list (dump `fs-menu-dump/1`) with the `## Menü-Inventar` list / dialogs table of the dossier. Deviations (`fehlt`,
`zusaetzlich`, `anders-benannt`, `kuerzel`) are allowed only with a reason in the Frontend's exception list; without one the
result is red. Applies to every Frontend of wave 1. Format: `docs/menue-vergleich.md`, self test
`scripts/test_menue_vergleich.sh`.

**Style gate of the theme bridge (user rules on ADR 0074, plan `/srv/FreeSynergy/.scratch/plan-theme-bruecke.md`).** In `frontend-umbau`, before `convert` (replacing painted copies by Blinc blocks), run `bruecke`, `alle-stile-erweitern`, `stil-vollstaendigkeit` (script: theme contract x all styles, a missing entry = red, no release) and `zwei-stile` (script: every block drawn in two styles, an unchanged picture = red, exceptions only for content; only blocks already reached through an adapter are drawn, own blocks still carry fixed values before `convert`). Order: contract, completeness, two styles. Every style styles every block with all parts and states; a new block, part or state = a new contract version and the fixed step `alle-stile-erweitern`. Red rounds are counted, from the third one the flow ends with the person. In `frontend-round`, `stil-sichern` saves the style of the rebuilt model (English name, reference and licence named, nothing copied), then `stil-vollstaendigkeit`. The scripts are not built yet.

**Block assignment (user decision 14 Nov, all Frontends).** Fixed order: (1) block analysis - every area of the model is assigned to a
block family and block id ("what is this?": table, tree, list, card, bar, dialog ...), existing blocks first, a new one only with the
reason why no existing block (with a module) fits; (2) look - style analysis, tokens and style rules per block (ADR 0074); (3)
composition - the arrangement in the Frontend. Example: the file table of fs-files is the same table as later in fs-data, only with
hidden headers and no calculation module (formulas = a module, ADR 0084 pattern). The objective check `baustein-zuordnung` (in
`dossier`, before publish; red jumps to `research`) lets `scripts/baustein-zuordnung` check the `## Baustein-Zuordnung` table of
`bausteine.md`, counts the red rounds itself (state file in the dossier folder, rc 3 from the third one) and reports the share per source
(family, Blinc widget, new; source order family, Blinc, new with a reason). Format: `docs/baustein-zuordnung.md`, self test
`scripts/test_baustein_zuordnung.sh`.

**New program (`programm-neu`, user request 14 Nov).** One flow from the test question to acceptance, 17 steps in seven phases:
P0 (1 "whose data?", ADR 0076, outcome Programm / Facette / Modul / Suite; 2 hub and catalog, ADR 0068), P1 (3 standards and their test
collections, ADR 0014; 4 core crate, wrapper not fork; 5 fix the formats (times UTC plus zone id, persons as references, ADR 0085 draft) and the way of moving house), P2 (6 data
model on the standards, saving is not exporting, ADR 0046; 7 topic interface, ADR 0045/0066; 8 manifest, ADR 0068), P3 (9 engine with
contract tests; 9a importers/exporters as border adapters per ADR 0085 (draft); 10 admin Frontend from the type description), P4 (11 model dossier; 12 `frontend-round` with block assignment, look,
composition, `menue-vergleich`; 13 language MF2 de+en and accessibility), P5 (14 hub registration and the `fs-server` adapter per ADR
0082; 15 security, ADR 0054/0056/0067; 16 packages, catalog ADR 0077, docs; then `release-flow`, then the server run), P6 (17 acceptance by the user, findings run
through `bugfix`). Every agent step (`objective`) is followed by an objective script check; red jumps back by `switch`/`then`, at most twice: the third red round (`runde` in the result of the check script, not reset by green inside a loop) ends in `eskalation` with the person. `frontend-round` runs here with `release: false` (one single release, only after language, security and packages). Every step names its
ADRs in `freesynergy.net:adr` and its phase in `freesynergy.net:phase`. The outcomes Facette, Modul and Suite leave the flow at a stub
(`facette-abzweig`, `modul-abzweig`, `suite-abzweig`); **`facette-anlegen` and a module flow do not exist yet** and are kept as a TODO,
not invented. No agent works on a real server: step 14 runs only locally against test instances, the real run is the person's
(`server-lauf`). Self test: `crates/fs-release/tests/workflows_programm_neu.rs` (counter-tests: a step without `kind`, without an ADR
reference, without a check step, a jump to nothing; structure rules: no dead step, jumps back land only on an agent or a person, escalation after three red rounds, order security, packages, release, server run, complete inputs of the sub-flows).
