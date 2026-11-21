#!/usr/bin/env python3
# Pattern: none (one-off migration helper, ADR 0105 E1 / decision 8).
"""Sets `actor-type: Service` and adds `freesynergy.net:bot-seat: <workflow>.<step>` to every top-level `actor: ai` step of workflows/*.yaml (text edit, formatting kept).

  python3 -I scripts/fill_botseat.py workflows
The seat value is a plain reference name `<workflow name>.<step name>` (ADR 0091 defines the seat as a mandate keyed by situation and number, ADR 0105 states no value format). Idempotent."""
import re
import sys
from pathlib import Path

total = 0
for f in sorted(Path(sys.argv[1]).glob("*.yaml")):
    lines = f.read_text().split("\n")
    m = next((re.match(r"\s+name:\s*(\S+)", l) for l in lines if re.match(r"\s+name:\s*\S+", l)), None)
    wf = m.group(1) if m else f.stem
    step, out, i = None, [], 0
    while i < len(lines):
        l = lines[i]
        s = re.match(r"^  - ([^\s:#][^:]*):\s*$", l)
        if s:
            step = s.group(1)
        out.append(l)
        a = re.match(r"^(\s+)freesynergy\.net:actor: ai\s*$", l)
        if a and step:
            ind = a.group(1)
            j = i + 1
            nxt = lines[j] if j < len(lines) else ""
            if re.match(r"\s+freesynergy\.net:actor-type:", nxt):
                out.append(f"{ind}freesynergy.net:actor-type: Service")
                i = j
            seat = f"{ind}freesynergy.net:bot-seat: {wf}.{step}"
            if seat not in lines[i : i + 12]:
                out.append(seat)
                total += 1
        i += 1
    f.write_text("\n".join(out))
print(total, "ai steps filled")
