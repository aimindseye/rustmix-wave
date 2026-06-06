#!/usr/bin/env bash
set -euo pipefail
ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"; FORCE=0
if [[ "${1:-}" == "--force" ]]; then FORCE=1; shift; fi
VOLUME="${1:-}"; if [[ -z "$VOLUME" ]]; then echo "usage: $0 [--force] /Volumes/YOUR_SD_CARD" >&2; exit 1; fi
SRC="$ROOT/examples/sd-card/RUSTMIX/APPS/SOKOBAN"; DEST="$VOLUME/RUSTMIX/APPS/SOKOBAN"; mkdir -p "$DEST"
copy_example(){ local src="$1" dest="$2"; if [[ -e "$dest" && "$FORCE" -ne 1 ]]; then echo "preserved-existing=$dest"; return; fi; cp "$src" "$dest"; echo "installed=$dest"; }
copy_example "$SRC/APP.TOM" "$DEST/APP.TOM"; copy_example "$SRC/MAIN.LUA" "$DEST/MAIN.LUA"
echo "rustmix-wave-sokoban-tilt-sd-sample-ready=$DEST"
