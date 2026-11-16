#!/usr/bin/env bash
# Pattern: none (shared functions of site-bauen.sh / site-freigabe-pruefen.sh / site-signieren.sh). Source it, do not run it.
# tree_list DIR   -> "<sha256>  <path>" per file, sorted bytewise, paths relative to DIR (LC_ALL=C so the order never depends on the locale)
# tree_hash DIR   -> sha256 of that list: ONE hash that names exactly this folder state (the approval is bound to it)
tree_list() {
  (cd "$1" && find . -type f -printf '%P\n' | LC_ALL=C sort | while IFS= read -r f; do sha256sum -- "$f"; done)
}
tree_hash() { tree_list "$1" | sha256sum | cut -d' ' -f1; }
# tar_tree_hash TARBALL -> tree_hash of the tarball's content, computed from the tarball itself (never from a manifest next to it)
tar_tree_hash() {
  local d; d=$(mktemp -d) || return 1
  if tar -xzf "$1" -C "$d" --no-same-owner --no-same-permissions 2>/dev/null; then tree_hash "$d"; else rm -rf "$d"; return 1; fi
  rm -rf "$d"
}
