# Pattern: none (check script).
"""The run rule set and the station rules of the workflows (standard `workflows/STATES.md`).

`check_rules(rules)` checks the ONE rule set `workflows/run/rules.yaml` (the states of a run, the wait reasons, the transitions). The states
belong to the run, once; a workflow file owns only rules for its own steps (stations): `check_station_rules` (who may, what happens on entry)
and `check_status_command` (a step that runs a status command needs a `timeout`, a local origin, no shell).
"""
import re

STATES = ("offen", "wartet-auf-entscheidung", "auf-anfrage", "in-prüfung", "pausiert", "fehlgeschlagen", "abgebrochen", "erledigt", "archiviert")
STATIONS = "stationen"  # not a state: the run is on one of the own steps of its workflow
WAIT = {"wartet-auf-entscheidung": "entscheidung", "auf-anfrage": "anfrage", "in-prüfung": "prüfung"}
FINAL = ("erledigt", "abgebrochen", "archiviert")  # no live state: nothing to pause or abort any more
ARCHIVE_FROM = ("erledigt", "abgebrochen", "fehlgeschlagen")
NOT_PAUSABLE = (*FINAL, "pausiert", "fehlgeschlagen")  # a failed run is not paused (open question, see STATES.md)
RULES_FILE = "run/rules.yaml"
META_RULES, META_STATUS = "freesynergy.net:station-rules", "freesynergy.net:status-command"
META_ALLOW, META_ORIGIN = "freesynergy.net:allow", "freesynergy.net:origin"
ON_ENTER = ("assign-role", "run-status-command", "create-decision", "set-wait-reason")
SHELLS = {"sh", "bash", "zsh", "dash", "fish", "ksh", "csh", "tcsh", "cmd", "powershell", "pwsh", "env"}
SHELL_CHARS = re.compile(r"[;&|`$<>(){}\n\\]")
MAX_TIMEOUT = 3600


def is_str_list(x):
    return isinstance(x, list) and all(isinstance(i, str) for i in x)


def check_rules(rules):
    """Problems of the run rule set (a parsed `run/rules.yaml`)."""
    if not isinstance(rules, dict):
        return ["the rule set is not a mapping"]
    p = []
    states = rules.get("states")
    if not is_str_list(states):
        return ["`states` is missing (a list of the nine state names)"]
    for s in STATES:
        if s not in states:
            p.append(f"state {s!r} is missing in `states`")
        elif states.count(s) > 1:
            p.append(f"state {s!r} is listed more than once in `states`")
    for s in states:
        if s not in STATES:
            p.append(f"unknown state {s!r} in `states` (allowed: {', '.join(STATES)}; {STATIONS!r} is no state)")
    if rules.get("start") != "offen":
        p.append(f"`start` is {rules.get('start')!r}, it must be 'offen'")
    if rules.get("wait-reasons") != WAIT:
        p.append("`wait-reasons` must map exactly " + ", ".join(f"{k} -> {v}" for k, v in WAIT.items()))
    trans = rules.get("transitions")
    if not isinstance(trans, dict):
        return p + ["`transitions` is missing (state -> targets)"]
    names = set(STATES) | {STATIONS}
    for src in (*STATES, STATIONS):
        if src not in trans:
            p.append(f"transition rule of {src!r} is missing")
    for src, targets in trans.items():
        if src not in names:
            p.append(f"transition from unknown state {src!r}")
        if not is_str_list(targets) or any(t not in names for t in targets):
            p.append(f"transitions of {src!r}: a list of states (or `{STATIONS}`) is required")
            continue
        if len(set(targets)) != len(targets):
            p.append(f"transitions of {src!r}: a target is listed twice")
        if "archiviert" in targets and src not in ARCHIVE_FROM:
            p.append(f"transition {src!r} -> 'archiviert' is not allowed (only from {', '.join(ARCHIVE_FROM)})")
        if src == "archiviert" and targets:
            p.append("'archiviert' has no way out")
        if src not in FINAL and "abgebrochen" not in targets:
            p.append(f"transition {src!r} -> 'abgebrochen' is missing (every live state, also fehlgeschlagen, can be aborted)")
        if src not in NOT_PAUSABLE and "pausiert" not in targets:
            p.append(f"transition {src!r} -> 'pausiert' is missing (every live state can be paused)")
    for src, tgt, why in (("fehlgeschlagen", STATIONS, "repeat after a failure"), (STATIONS, "fehlgeschlagen", "a station fails"),
                          ("erledigt", "offen", "reopen"), ("abgebrochen", "offen", "reopen")):
        if isinstance(trans.get(src), list) and tgt not in trans[src]:
            p.append(f"transition {src!r} -> {tgt!r} is missing ({why})")
    return p


def check_station_rules(meta, step_names):
    """`document.metadata` of a flow: no states of its own; optional rules only for its own steps."""
    p = []
    if not isinstance(meta, dict):
        return p
    for key in ("freesynergy.net:states", "freesynergy.net:transitions"):
        if key in meta:
            p.append(f"document.metadata {key} must not be there (the states belong to the run: workflows/run/rules.yaml)")
    rules = meta.get(META_RULES)
    if rules is None:
        return p
    if not isinstance(rules, dict):
        return p + [f"document.metadata {META_RULES} must be a mapping station -> rule"]
    for station, rule in rules.items():
        if station not in step_names:
            p.append(f"station rule for {station!r}: no step of this flow has this name")
        if not isinstance(rule, dict) or set(rule) - {"allow", "on-enter"}:
            p.append(f"station rule of {station!r}: only `allow` and `on-enter` are allowed")
            continue
        if "allow" in rule and (not is_str_list(rule["allow"]) or not rule["allow"]):
            p.append(f"station rule of {station!r}: `allow` is a non-empty list of roles")
        enter = rule.get("on-enter", [])
        if not is_str_list(enter) or any(e not in ON_ENTER for e in enter):
            p.append(f"station rule of {station!r}: `on-enter` is a list of {', '.join(ON_ENTER)}")
    return p


def seconds(t):
    """The limit of a Serverless Workflow `timeout` (`after`: an ISO 8601 duration or a {seconds, minutes, hours} mapping) in seconds."""
    after = t.get("after") if isinstance(t, dict) else None
    if isinstance(after, str):
        m = re.fullmatch(r"PT(?:(\d+)H)?(?:(\d+)M)?(?:(\d+)S)?", after)
        if m and any(m.groups()):
            h, mi, s = (int(g or 0) for g in m.groups())
            return h * 3600 + mi * 60 + s
    if isinstance(after, dict) and after and set(after) <= {"hours", "minutes", "seconds"} and all(isinstance(v, int) for v in after.values()):
        return after.get("hours", 0) * 3600 + after.get("minutes", 0) * 60 + after.get("seconds", 0)
    return None


def check_status_command(where, task, meta):
    """A step with `freesynergy.net:status-command`: `timeout` 1..3600 s, `freesynergy.net:origin: local`, no shell, a script under
    `scripts/` or a program of `freesynergy.net:allow`."""
    cmd = meta.get(META_STATUS)
    if cmd is None:
        return []
    p = []
    if not isinstance(cmd, str) or not cmd.strip():
        return [f"{where}: {META_STATUS} is a non-empty command string"]
    limit = seconds(task.get("timeout"))
    if limit is None:
        p.append(f"{where}: a status command needs a `timeout` (after: ISO 8601 duration or seconds/minutes/hours)")
    elif not 1 <= limit <= MAX_TIMEOUT:
        p.append(f"{where}: the `timeout` of a status command is {limit} s, allowed 1..{MAX_TIMEOUT}")
    if meta.get(META_ORIGIN) != "local":
        p.append(f"{where}: a status command runs only for runs of local origin ({META_ORIGIN}: local)")
    allow = meta.get(META_ALLOW, [])
    if not is_str_list(allow):
        p.append(f"{where}: {META_ALLOW} is a list of program names")
        allow = []
    program = cmd.split()[0]
    if SHELL_CHARS.search(cmd):
        p.append(f"{where}: the status command contains shell syntax")
    if program.rsplit("/", 1)[-1] in SHELLS or any(a.rsplit("/", 1)[-1] in SHELLS for a in allow):
        p.append(f"{where}: a shell is no status command")
    if not (program.startswith("scripts/") and ".." not in program) and program not in allow:
        p.append(f"{where}: the status command {program!r} is neither a script under scripts/ nor in {META_ALLOW}")
    return p
