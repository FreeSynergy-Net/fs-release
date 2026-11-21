# Pattern: none (check script).
"""The run rule set and the station rules of the workflows (standard `workflows/STATES.md`).

usage as a script: `python3 -I check_run_rules.py <rules.yaml> <state> [reason]` checks one state of a run against the rule set
(`wartet` needs a reason of its list, every other state has none); exit 0 ok, 1 red.

`check_rules(rules)` checks the ONE rule set `workflows/run/rules.yaml`. The LIST of states is data: the structure rules are derived from the
file (every state has a rule, every target is a state, `final` states, the end state without way out, `reasons`, `reopen`, `archive-from`).
Only the roles are names here: ROLE below (start, run, pause, abort, fail, wait). That the default list is the one of the standard is
checked by the tests against `STATES.md` (the table), not by this file. The states belong to the run, once; a workflow file owns only rules
for its own stations: `check_station_rules` (who may, what happens on entry) and `check_status_command` (a step that runs a status command
needs a `timeout`, a local origin, no shell and no interpreter).
"""
import os
import re
import sys

ROLE = {"start": "offen", "run": "läuft", "pause": "pausiert", "abort": "abgebrochen", "fail": "fehlgeschlagen", "wait": "wartet"}
RULES_FILE = "run/rules.yaml"
RULE_SET_NAME = "freesynergy"
META_RULES, META_STATUS, META_RUN_RULES = "freesynergy.net:station-rules", "freesynergy.net:status-command", "freesynergy.net:run-rules"
META_EXEMPT = "freesynergy.net:timebox-exempt"
META_ALLOW, META_ORIGIN = "freesynergy.net:allow", "freesynergy.net:origin"
ON_ENTER = ("assign-role", "run-status-command", "create-decision", "set-wait-reason")
RULE_KEYS = {"allow", "on-enter", "from", "confirm"}
SHELLS = {"sh", "bash", "zsh", "dash", "fish", "ksh", "csh", "tcsh", "cmd", "powershell", "pwsh", "env"}
# Programs that run any code they are given (so they are no status command, also on the allow list). The list can never be complete: every
# `allow` entry stays a decision of trust (STATES.md). Reasons: script languages (perl, ruby, node, deno, bun, php, lua, tclsh, expect,
# python*, pypy*), text tools that execute (awk, find -exec, xargs, sed `e`, dc `!`), runners of other commands (sudo, nohup, timeout, make,
# git alias/hooks, busybox).
INTERPRETERS = {"perl", "ruby", "node", "nodejs", "deno", "bun", "awk", "gawk", "mawk", "find", "xargs", "sed", "dc", "php", "lua", "tclsh",
                "osascript", "sudo", "nohup", "timeout", "make", "git", "busybox", "expect"}
INTERPRETER_RE = re.compile(r"python[0-9.]*|pypy[0-9.]*|perl[0-9.]*|ruby[0-9.]*")
SHELL_CHARS = re.compile(r"[;&|`$<>(){}\n\r\\]")
MAX_TIMEOUT = 3600
MAX_TIMEBOX = 60
TIMEBOX_ACTIONS = ("stop", "report", "workflow-pflege")


def is_str_list(x):
    return isinstance(x, list) and all(isinstance(i, str) and i for i in x)


def program_name(p):
    return p.rsplit("/", 1)[-1].lower()


def is_shell(p):
    return program_name(p) in SHELLS


def is_interpreter(p):
    n = program_name(p)
    return n in INTERPRETERS or bool(INTERPRETER_RE.fullmatch(n))


def check_rules(rules):
    """Problems of the run rule set (a parsed `run/rules.yaml`)."""
    if not isinstance(rules, dict):
        return ["the rule set is not a mapping"]
    p = []
    if rules.get("name") != RULE_SET_NAME:
        p.append(f"`name` is {rules.get('name')!r}, the default rule set is called {RULE_SET_NAME!r}")
    states = rules.get("states")
    if not is_str_list(states) or not states:
        return p + ["`states` is missing (a list of state names)"]
    for s in dict.fromkeys(states):
        if states.count(s) > 1:
            p.append(f"state {s!r} is listed more than once in `states`")
    for role, name in ROLE.items():
        if name not in states:
            p.append(f"state {name!r} ({role}) is missing in `states`")
    if rules.get("start") != ROLE["start"]:
        p.append(f"`start` is {rules.get('start')!r}, it must be {ROLE['start']!r}")
    for key in ("final", "reopen", "archive-from"):
        v = rules.get(key)
        if not is_str_list(v) or any(s not in states for s in v):
            p.append(f"`{key}` is a list of states")
    final = rules["final"] if is_str_list(rules.get("final")) else []
    reasons = rules.get("reasons")
    if not isinstance(reasons, dict) or not all(is_str_list(v) and v and len(set(v)) == len(v) for v in reasons.values()):
        p.append("`reasons` maps a state to its non-empty list of reasons")
        reasons = {}
    if set(reasons) != {ROLE["wait"]}:
        p.append(f"only {ROLE['wait']!r} has reasons, and it must have them (a waiting run always says why)")
    trans = rules.get("transitions")
    if not isinstance(trans, dict):
        return p + ["`transitions` is missing (state -> targets)"]
    for src in states:
        if src not in trans:
            p.append(f"transition rule of {src!r} is missing")
    ends = []
    for src, targets in trans.items():
        if src not in states:
            p.append(f"transition from unknown state {src!r}")
        if not isinstance(targets, list) or not all(isinstance(t, str) and t in states for t in targets):
            p.append(f"transitions of {src!r}: a list of states is required")
            continue
        if len(set(targets)) != len(targets):
            p.append(f"transitions of {src!r}: a target is listed twice")
        if not targets:
            ends.append(src)
        if src not in final and ROLE["abort"] not in targets:
            p.append(f"transition {src!r} -> {ROLE['abort']!r} is missing (every live state, also {ROLE['fail']}, can be aborted)")
        if src not in final and src != ROLE["pause"] and ROLE["pause"] not in targets:
            p.append(f"transition {src!r} -> {ROLE['pause']!r} is missing (every live state, also {ROLE['fail']}, can be paused)")
    if len(ends) != 1 or ends[0] not in final:
        p.append(f"there must be exactly one end state (a final state without way out), found {ends}")
    else:
        end = ends[0]
        for src, targets in trans.items():
            if isinstance(targets, list) and end in targets and src not in (rules.get("archive-from") or []):
                p.append(f"transition {src!r} -> {end!r} is not allowed (only from {', '.join(rules.get('archive-from') or [])})")
        for src in rules.get("archive-from") or []:
            if isinstance(trans.get(src), list) and end not in trans[src]:
                p.append(f"transition {src!r} -> {end!r} is missing (archive-from)")
    for src in rules.get("reopen") or []:
        if isinstance(trans.get(src), list) and ROLE["start"] not in trans[src]:
            p.append(f"transition {src!r} -> {ROLE['start']!r} is missing (reopen)")
    for src, tgt, why in ((ROLE["fail"], ROLE["run"], "repeat after a failure"), (ROLE["run"], ROLE["fail"], "a station fails"),
                          (ROLE["pause"], ROLE["run"], "continue at the same station"), (ROLE["run"], ROLE["wait"], "a station waits")):
        if isinstance(trans.get(src), list) and tgt not in trans[src]:
            p.append(f"transition {src!r} -> {tgt!r} is missing ({why})")
    return p + check_timebox(rules.get("timebox"))


def check_timebox(tb):
    """The `timebox` block of the run rule set (CLAUDE.md 4a): positive integer minutes, at most MAX_TIMEBOX, and the actions on exceed."""
    if not isinstance(tb, dict):
        return ["`timebox` is missing (max-minutes, visible-every-minutes, on-exceed)"]
    p = []
    for key in ("max-minutes", "visible-every-minutes"):
        v = tb.get(key)
        if type(v) is not int or not 1 <= v <= MAX_TIMEBOX:
            p.append(f"`timebox` {key} is {v!r}, it must be a whole number of minutes 1..{MAX_TIMEBOX}")
    if tb.get("on-exceed") != list(TIMEBOX_ACTIONS):
        p.append(f"`timebox` on-exceed is {tb.get('on-exceed')!r}, it must be {list(TIMEBOX_ACTIONS)}")
    if set(tb) - {"max-minutes", "visible-every-minutes", "on-exceed"}:
        p.append("`timebox` has only max-minutes, visible-every-minutes, on-exceed")
    return p


def check_run_state(rules, state, reason=None):
    """One state of a run: `wartet` needs a reason out of its list, any other state has none."""
    states = rules.get("states", [])
    if state not in states:
        return [f"unknown state {state!r}"]
    allowed = (rules.get("reasons") or {}).get(state)
    if allowed is None:
        return [f"state {state!r} has no reason, but {reason!r} is given"] if reason is not None else []
    if not reason:
        return [f"state {state!r} needs a reason (one of {', '.join(allowed)})"]
    return [] if reason in allowed else [f"reason {reason!r} of state {state!r} is not one of {', '.join(allowed)}"]


def check_on_enter(station, enter, rules):
    """`on-enter` is a list of one-key mappings `- action: value` (a proposal: the form is open, see STATES.md)."""
    p = []
    reasons = (rules or {}).get("reasons", {}).get(ROLE["wait"], [])
    if not isinstance(enter, list):
        return [f"station rule of {station!r}: `on-enter` is a list of `- action: value` ({', '.join(ON_ENTER)})"]
    for item in enter:
        if not isinstance(item, dict) or len(item) != 1 or next(iter(item)) not in ON_ENTER:
            p.append(f"station rule of {station!r}: `on-enter` is a list of `- action: value` ({', '.join(ON_ENTER)})")
            continue
        ((action, value),) = item.items()
        if not isinstance(value, str) or not value.strip():
            p.append(f"station rule of {station!r}: the action {action!r} needs a value")
        elif action == "set-wait-reason" and value not in reasons:
            p.append(f"station rule of {station!r}: set-wait-reason {value!r} is not one of {', '.join(reasons)}")
    return p


def check_station_rules(meta, step_names, rules=None):
    """`document.metadata` of a flow: no states of its own; optional rules only for its own steps."""
    p = []
    if not isinstance(meta, dict):
        return p
    for key in ("freesynergy.net:states", "freesynergy.net:transitions"):
        if key in meta:
            p.append(f"document.metadata {key} must not be there (the states belong to the run: workflows/run/rules.yaml)")
    if META_RUN_RULES in meta and meta[META_RUN_RULES] != RULE_SET_NAME:
        p.append(f"document.metadata {META_RUN_RULES} is {meta[META_RUN_RULES]!r}: only the rule set {RULE_SET_NAME!r} exists (own rule sets follow with fs-flow)")
    rule_map = meta.get(META_RULES)
    if rule_map is None:
        return p
    if not isinstance(rule_map, dict):
        return p + [f"document.metadata {META_RULES} must be a mapping station -> rule"]
    for station, rule in rule_map.items():
        if station not in step_names:
            p.append(f"station rule for {station!r}: no step of this flow has this name")
        if not isinstance(rule, dict) or set(rule) - RULE_KEYS:
            p.append(f"station rule of {station!r}: only {', '.join(sorted(RULE_KEYS))} are allowed")
            continue
        if "allow" in rule and (not is_str_list(rule["allow"]) or not rule["allow"]):
            p.append(f"station rule of {station!r}: `allow` is a non-empty list of roles")
        if "from" in rule and (not is_str_list(rule["from"]) or any(s not in step_names for s in rule["from"])):
            p.append(f"station rule of {station!r}: `from` is a list of steps of this flow")
        if "confirm" in rule and not isinstance(rule["confirm"], bool):
            p.append(f"station rule of {station!r}: `confirm` is true or false")
        p += check_on_enter(station, rule.get("on-enter", []), rules)
    return p


def seconds(t):
    """The limit of a Serverless Workflow `timeout` (`after`: an ISO 8601 duration or a {seconds, minutes, hours} mapping) in seconds."""
    after = t.get("after") if isinstance(t, dict) else None
    if isinstance(after, str):
        m = re.fullmatch(r"PT(?:(\d+)H)?(?:(\d+)M)?(?:(\d+)S)?", after)
        if m and any(m.groups()):
            h, mi, s = (int(g or 0) for g in m.groups())
            return h * 3600 + mi * 60 + s
    if isinstance(after, dict) and after and set(after) <= {"hours", "minutes", "seconds"} and all(type(v) is int and v >= 0 for v in after.values()):
        return after.get("hours", 0) * 3600 + after.get("minutes", 0) * 60 + after.get("seconds", 0)
    return None


def timeouts(node):
    """Every Serverless Workflow `timeout` (a mapping with `after`) below a node: [(seconds or None)]."""
    found = []
    if isinstance(node, dict):
        for k, v in node.items():
            if k == "timeout" and isinstance(v, dict) and "after" in v:
                found.append(seconds(v))
            else:
                found += timeouts(v)
    elif isinstance(node, list):
        for v in node:
            found += timeouts(v)
    return found


def check_timebox_exempt(where, node, meta, rules):
    """A `timeout` longer than `timebox.max-minutes` of the rule set needs `freesynergy.net:timebox-exempt: <reason>`."""
    limit = ((rules or {}).get("timebox") or {}).get("max-minutes")
    if type(limit) is not int or META_EXEMPT in (meta or {}) and str(meta[META_EXEMPT]).strip():
        return []
    return [f"{where}: a `timeout` of {s} s is longer than the time box of the run ({limit} min, run/rules.yaml); add {META_EXEMPT}: <reason>"
            for s in timeouts({k: v for k, v in node.items() if k != "metadata"}) if s is not None and s > limit * 60]


def check_status_command(where, task, meta):
    """A step with `freesynergy.net:status-command`: `timeout` 1..3600 s, `freesynergy.net:origin: local`, no shell, no interpreter, a
    script under `scripts/` or a program of `freesynergy.net:allow`."""
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
    if is_shell(program) or any(is_shell(a) for a in allow):
        p.append(f"{where}: a shell is no status command")
    if is_interpreter(program) or any(is_interpreter(a) for a in allow):
        p.append(f"{where}: an interpreter is no status command (it runs any code)")
    if not (program.startswith("scripts/") and ".." not in program) and program not in allow:
        p.append(f"{where}: the status command {program!r} is neither a script under scripts/ nor in {META_ALLOW}")
    return p


def main(argv):
    import yaml
    with open(argv[1], encoding="utf-8") as f:
        problems = check_run_state(yaml.safe_load(f), argv[2], argv[3] if len(argv) > 3 else None)
    for p in problems:
        print(p)
    return 1 if problems else 0


if __name__ == "__main__":
    sys.path.insert(0, os.path.dirname(os.path.realpath(__file__)))
    sys.exit(main(sys.argv))
