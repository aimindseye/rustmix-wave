#!/usr/bin/env bash
set -euo pipefail
ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
VOLUME="${1:-}"
if [[ -z "$VOLUME" ]]; then
  echo "usage: $0 /Volumes/YOUR_SD_CARD" >&2
  exit 1
fi
DEST="$VOLUME/RUSTMIX/SLEEP"
mkdir -p "$DEST"
cp "$ROOT/examples/sd-card/RUSTMIX/SLEEP/SLEEP.BMP" "$DEST/SLEEP.BMP"
cp "$ROOT/examples/sd-card/RUSTMIX/SLEEP/SLEEP01.BMP" "$DEST/SLEEP01.BMP"
echo "rustmix-wave-sleep-images-installed=$DEST"
