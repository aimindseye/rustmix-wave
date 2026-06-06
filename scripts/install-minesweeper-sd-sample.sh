#!/usr/bin/env bash
set -euo pipefail
ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
FORCE=0
if [[ "${1:-}" == "--force" ]]; then FORCE=1; shift; fi
VOLUME="${1:-}"
if [[ -z "$VOLUME" ]]; then echo "usage: $0 [--force] /Volumes/YOUR_SD_CARD" >&2; exit 1; fi
SRC="$ROOT/examples/sd-card/RUSTMIX/APPS/MINES"
DEST="$VOLUME/RUSTMIX/APPS/MINES"
mkdir -p "$DEST"
for name in APP.TOM MAIN.LUA; do
  if [[ -e "$DEST/$name" && "$FORCE" -ne 1 ]]; then
    echo "preserved-existing=$DEST/$name"
  else
    cp "$SRC/$name" "$DEST/$name"
    echo "installed=$DEST/$name"
  fi
done
echo "rustmix-wave-minesweeper-sd-sample-ready=$DEST"
