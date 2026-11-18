#!/usr/bin/env python3
# Pattern: none (script of the pin cascade tests).
"""Shape gate for the self tests `scripts/test_kaskade_*.sh` (Opus N1/N3, relapse of K2): in `chk "name" cmd...` only `cmd...` is checked. Anything chained to a `chk` call on the same
logical line by `&&`, `||` or `;` that is not itself a `chk` call (or a closing keyword `done`, `fi`, `then`, `do`, `else`) is NOT checked, so it is forbidden.

  python3 -I scripts/check_chk_shape.py FILE...      rc 0 = clean, 1 = findings (`file:line: text` on stdout), 2 = bad input

Covers: the pattern INDENTED (in a loop or function), `;`, `||`, `&&`, and continuation lines (`\\` at the end of a line joins the next one first); `&&`, `||` and `;` inside quotes or `$( )` do not count.
Here-document bodies are data and are skipped (an unterminated one is a finding); `eval` is a finding; a trailing `&&`, `||` or `|` joins the next line. A `chk` call also counts after `do`, `then`, `else`, `{`. Two chk calls on one line (`chk "a" [ x ] && chk "b" [ y ]`) are fine: every part is checked.
"""
import re
import sys

KEYWORDS = ("done", "fi", "then", "do", "else", "}")


def strip_heredocs(text):
    """The bodies of here-documents are data, not commands: blanked (the line numbers stay). Only a `<<WORD` OUTSIDE quotes counts (not `<<<`, not `"<<WORD"`, not `$(( 1 << x ))`);
    a here-document without its end marker is a FINDING (returned in `bad`), never a reason to swallow the rest of the file."""
    out, end, bad, lines = [], None, [], text.split("\n")
    for n, raw in enumerate(lines, 1):
        if end is not None:
            if raw.strip() == end[0]:
                end = None
            out.append("")
            continue
        out.append(raw)
        m = heredoc_marker(raw)
        if m:
            if any(x.strip() == m for x in lines[n:]):
                end = (m,)
            else:
                bad.append((n, raw.strip()))
    return "\n".join(out), bad


def heredoc_marker(line):
    """The end marker of a `<<WORD` / `<<-WORD` / `<<'WORD'` outside quotes and `$(( ))`, or None."""
    st, i = [], 0
    while i < len(line):
        c, top = line[i], (st[-1] if st else None)
        if top == "s":
            if c == "'":
                st.pop()
        elif top == "d":
            if c == "\\":
                i += 1
            elif c == '"':
                st.pop()
        elif c == "'":
            st.append("s")
        elif c == '"':
            st.append("d")
        elif line.startswith("$((", i):
            st.append("a")
            i += 2
        elif top == "a":
            if line.startswith("))", i):
                st.pop()
                i += 1
        elif line.startswith("<<<", i):
            i += 2
        elif line.startswith("<<", i):
            m = re.match(r"<<-?\s*(?:'([A-Za-z_]+)'|\"([A-Za-z_]+)\"|([A-Za-z_][A-Za-z_0-9]*))", line[i:])
            if m:
                return next(g for g in m.groups() if g)
        i += 1
    return None


def logical_lines(text):
    """[(first line number, joined text)]: a line ending in a backslash is joined with the next one."""
    out, buf, start = [], "", 0
    for n, raw in enumerate(text.split("\n"), 1):
        if not buf:
            start = n
        if raw.endswith("\\"):
            buf += raw[:-1] + " "
            continue
        if re.search(r"(&&|\|\||\|)\s*$", raw) and not raw.strip().startswith("#"):
            buf += raw + " "  # a trailing && || | continues on the next line, no backslash needed
            continue
        out.append((start, buf + raw))
        buf = ""
    if buf:
        out.append((start, buf))
    return out


def split_top(s):
    """Split at top-level `&&`, `||` and `;` (not inside '...', "...", $( ), backticks); returns the parts."""
    st, parts, cur, i = [], [], 0, 0
    while i < len(s):
        c, top = s[i], (st[-1] if st else None)
        if top == "s":
            if c == "'":
                st.pop()
        elif top == "d":
            if c == "\\":
                i += 1
            elif c == '"':
                st.pop()
            elif s.startswith("$(", i):
                st.append("p")
                i += 1
        elif top == "b":
            if c == "`":
                st.pop()
        else:
            if c == "\\":
                i += 1
            elif c == "'":
                st.append("s")
            elif c == '"':
                st.append("d")
            elif c == "`":
                st.append("b")
            elif s.startswith("$(", i):
                st.append("p")
                i += 1
            elif c == ")" and top == "p":
                st.pop()
            elif top is None and c == "#" and (i == 0 or s[i - 1] in " \t"):
                parts.append(s[cur:i])
                return parts
            elif top is None and (s.startswith("&&", i) or s.startswith("||", i)):
                parts.append(s[cur:i])
                cur = i + 2
                i += 1
            elif top is None and c == ";":
                parts.append(s[cur:i])
                cur = i + 1
        i += 1
    parts.append(s[cur:])
    return parts


def is_chk(part):
    p = re.sub(r"^(?:(?:do|then|else|\{)\s+)+", "", part.strip())
    return re.match(r"^chk\s", p) is not None


def findings(text):
    body, out = strip_heredocs(text)
    for n, line in logical_lines(body):
        bare = re.sub(r"'[^']*'|\"(?:[^\"\\]|\\.)*\"", "''", line)  # words inside quotes are data
        if re.search(r"(^|[\s;&|(])eval\s", bare) and not line.lstrip().startswith("#"):
            out.append((n, line.strip()))
            continue
        parts = [p.strip() for p in split_top(line)]
        for k, p in enumerate(parts[:-1]):
            if not is_chk(p):
                continue
            nxt = parts[k + 1]
            if nxt and not is_chk(nxt) and nxt not in KEYWORDS:
                out.append((n, line.strip()))
                break
    return out


def main(argv):
    if not argv:
        print("usage: check_chk_shape.py FILE...", file=sys.stderr)
        return 2
    bad = 0
    for f in argv:
        try:
            text = open(f, encoding="utf-8").read()
        except OSError as e:
            print(f"cannot read {f}: {e}", file=sys.stderr)
            return 2
        for n, line in findings(text):
            print(f"{f}:{n}: {line[:200]}")
            bad += 1
    return 1 if bad else 0


if __name__ == "__main__":
    sys.exit(main(sys.argv[1:]))
