#!/usr/bin/env bash
# Helpers of scripts/gates.sh for the Fehlerliste ([GATES-SAMMELLISTE]); sourced, own file so the rc rule has a test (scripts/test_gates_sammelliste.sh).

# gates_read_rc <file>: the rc a step wrote into <file>. A file that is missing, empty or no number (disk full, killed step) is RED (1), never green.
gates_read_rc() {
  local v=""
  [ -r "$1" ] && v="$(cat "$1")"
  if [[ "$v" =~ ^[0-9]+$ ]]; then echo "$v"; else echo 1; fi
}
