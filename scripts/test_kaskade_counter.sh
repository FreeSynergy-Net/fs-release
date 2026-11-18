#!/usr/bin/env bash
# Pattern: none (test support, sourced by scripts/test_kaskade_{plan,pins,gates}.sh).
# Self test of the FAILURE COUNTER of a test script (Opus N4/N5): `bad()` also writes into $tmp/.fails and the end line (`wait; nfail=...`) reads that file, so a failing chk cannot be lost in a
# subshell, pipe, background job, if, case, !, $( ), function, env prefix, eval, while, ||, ; or a trailing &&. `counter_selftest <script>` takes the helper lines (ok/bad/chk) and the end line
# out of THAT script (so a mutation of them in any of the three scripts turns THAT script red), runs a failing chk in every form, and mutates them six ways (M1..M6), each must turn it red.
# It also checks that the helper lines and the end line are byte-identical in the three scripts. Needs from the caller: here, tmp, chk, ok, bad.
counter_pick() { # <script> <what>: helpers | end
  case "$2" in
    helpers) grep -m1 -E '^ok\(\)' "$1"; grep -m1 -E '^bad\(\)' "$1"; grep -m1 -E '^chk\(\)' "$1" ;;  # the FIRST of each (a sample in a here-document must not count)
    end) grep -m1 -E '^wait; nfail=' "$1" ;;
  esac
}
# counted <helpers> <form>: lines in the counter file after the form (a FAILING chk inside it)
counter_counted() { bash -c 'tmp="$1"; fails=0; : >"$tmp/.fails"; eval "$2"; { eval "$3"; wait; } >/dev/null 2>&1; wc -l <"$tmp/.fails"' _ "$tmp/ctr" "$1" "$2" 2>/dev/null; }
counter_all() { # <helpers>: every form leaves at least one line in the counter file
  local form
  for form in '( chk "f" [ 1 -eq 2 ] )' 'chk "f" [ 1 -eq 2 ] | tee /dev/null' 'chk "f" [ 1 -eq 2 ] &' '{ sleep 0.2; chk "f" [ 1 -eq 2 ]; } &' 'if chk "f" [ 1 -eq 2 ]; then :; fi' \
    'case a in a) chk "f" [ 1 -eq 2 ] ;; esac' '! chk "f" [ 1 -eq 2 ]' 'x=$(chk "f" [ 1 -eq 2 ])' 'f() { chk "f" [ 1 -eq 2 ]; }; f' 'X=1 chk "f" [ 1 -eq 2 ]' \
    "eval 'chk \"f\" [ 1 -eq 2 ]'" 'chk "f" [ 1 -eq 2 ] &&
  [ 1 -eq 1 ]' 'while chk "f" [ 1 -eq 2 ]; do break; done' 'chk "f" [ 1 -eq 2 ] || true' 'chk "f" [ 1 -eq 2 ]; true'; do
    [ "$(counter_counted "$1" "$form")" -ge 1 ] || { echo "not counted: $form" >&2; return 1; }
  done
}
# counter_endrun <helpers> <endline> <form>: runs the form (a failing chk), then the END LINE of the script; returns its rc, output on stdout
counter_endrun() { bash -c 'tmp="$1"; fails=0; : >"$tmp/.fails"; eval "$2"; eval "$4"; eval "$3"' _ "$tmp/ctr" "$1" "$2" "$3"; }
counter_good() { # <helpers> <endline>: all checks that the real mechanism must pass
  local rc out
  counter_all "$1" || return 1
  [ "$(counter_counted "$1" 'chk "p" [ 1 -eq 1 ]')" = 0 ] || return 1
  rc=0; out="$(counter_endrun "$1" "$2" '( chk "f" [ 1 -eq 2 ] ) >/dev/null' 2>&1)" || rc=$?
  [ "$rc" -eq 1 ] && ! printf '%s' "$out" | grep -q 'all ok' || return 1
  rc=0; counter_endrun "$1" "$2" '{ sleep 0.2; chk "f" [ 1 -eq 2 ]; } >/dev/null &' >/dev/null 2>&1 || rc=$?
  [ "$rc" -eq 1 ] || return 1
  rc=0; counter_endrun "$1" "$2" 'chk "p" [ 1 -eq 1 ] >/dev/null' >/dev/null 2>&1 || rc=$?
  [ "$rc" -eq 0 ]
}
counter_selftest() { # <script>
  local s="$1" h e mh me d
  h="$(counter_pick "$s" helpers)"; e="$(counter_pick "$s" end)"
  mkdir -p "$tmp/ctr"
  chk "counter ($(basename "$s")): the helper lines and the end line are byte-identical in the three test scripts" \
    [ "$(for d in plan pins gates; do counter_pick "$here/test_kaskade_$d.sh" helpers; counter_pick "$here/test_kaskade_$d.sh" end; done | sort | uniq -c | awk '{print $1}' | sort -u | tr -d '\n')" = 3 ]
  chk "counter ($(basename "$s")): a failing chk is counted in EVERY form (subshell, pipe, background, brace background, if, case, !, \$( ), function, env prefix, eval, trailing &&, while, ||, ;), a passing one is not, the end line is red/green accordingly" counter_good "$h" "$e"
  # M1..M6: each mutation of the helpers / the end line must turn the self test red
  mh="$(printf '%s\n' "$h" | sed 's|; echo "\$1" >>"\$tmp/.fails"||')"
  chk "counter ($(basename "$s")) mutant M1 (bad() does not write the file) is caught" [ "$(counter_good "$mh" "$e" && echo green || echo red)" = red ]
  me="$(printf '%s\n' "$e" | sed 's/nfail="$(wc -l <"$tmp\/.fails")"/nfail=0/')"
  chk "counter ($(basename "$s")) mutant M2 (the end line ignores the file) is caught" [ "$(counter_good "$h" "$me" && echo green || echo red)" = red ]
  mh="$(printf '%s\n' "$h" | sed 's|>>"\$tmp/.fails"|>>"$tmp/.failz"|')"
  chk "counter ($(basename "$s")) mutant M3 (bad() writes another path) is caught" [ "$(counter_good "$mh" "$e" && echo green || echo red)" = red ]
  me="$(printf '%s\n' "$e" | sed 's/^wait; nfail="\(.*\)"; if /nfail="\1"; wait; if /')"
  chk "counter ($(basename "$s")) mutant M4 (the file is read before wait) is caught" [ "$me" != "$e" ] && chk "counter ($(basename "$s")) mutant M4 (the file is read before wait) is caught [part 2]" [ "$(counter_good "$h" "$me" && echo green || echo red)" = red ]
  me="$(printf '%s\n' "$e" | sed 's/^wait; //')"
  chk "counter ($(basename "$s")) mutant M5 (no wait before the file is read) is caught" [ "$me" != "$e" ] && chk "counter ($(basename "$s")) mutant M5 (no wait) is caught [part 2]" [ "$(counter_good "$h" "$me" && echo green || echo red)" = red ]
  me="$(printf '%s\n' "$e" | sed 's/\[ "\$nfail" -eq 0 \] && \[ "\$fails" -eq 0 \]/[ "$fails" -eq 0 ]/')"
  chk "counter ($(basename "$s")) mutant M6 (the end line uses only the variable) is caught" [ "$me" != "$e" ] && chk "counter ($(basename "$s")) mutant M6 (only the variable) is caught [part 2]" [ "$(counter_good "$h" "$me" && echo green || echo red)" = red ]
}
