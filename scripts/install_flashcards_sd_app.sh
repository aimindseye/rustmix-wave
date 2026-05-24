#!/usr/bin/env bash
set -euo pipefail
if [[ $# -ne 1 ]]; then
  printf 'usage: %s <mounted-sd-root>\n' "$0" >&2
  exit 2
fi
SD_ROOT="$1"
if [[ ! -d "$SD_ROOT" ]]; then
  printf 'SD root does not exist: %s\n' "$SD_ROOT" >&2
  exit 1
fi
SRC="examples/sd-card/RUSTMIX/APPS/FLASHCRD"
DST="$SD_ROOT/RUSTMIX/APPS/FLASHCRD"
if [[ ! -d "$SRC" ]]; then
  printf 'Flashcards source not found. Run apply script from repo root first.\n' >&2
  exit 1
fi
mkdir -p "$(dirname "$DST")"
rm -rf "$DST"
cp -R "$SRC" "$DST"
find "$DST" -name '.DS_Store' -delete
printf 'installed Flashcards SD app to %s\n' "$DST"
