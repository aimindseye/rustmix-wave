#!/usr/bin/env bash
set -euo pipefail

BIN="${1:-dist/x4/firmware.bin}"
CHIP="${CHIP:-esp32c3}"
PORT_ARG=()

if [ "${PORT:-}" != "" ]; then
  PORT_ARG=(--port "$PORT")
fi

if [ ! -f "$BIN" ]; then
  echo "firmware binary not found: $BIN" >&2
  exit 1
fi

# firmware.bin is generated as a merged full-flash image when the local espflash supports --merge.
# It is written at offset 0x0.
espflash write-bin --chip "$CHIP" "${PORT_ARG[@]}" 0x0 "$BIN"
echo 'marker=x4-release-firmware-bin-flashed'
