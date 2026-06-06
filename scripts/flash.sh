#!/usr/bin/env bash
set -euo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$ROOT"

PORT=""
case "$#" in
  0)
    ;;
  1)
    if [[ "$1" != "monitor" ]]; then
      PORT="$1"
    fi
    ;;
  2)
    if [[ "$1" != "--port" ]]; then
      echo "usage: $0 [monitor|PORT|--port PORT]" >&2
      exit 1
    fi
    PORT="$2"
    ;;
  *)
    echo "usage: $0 [monitor|PORT|--port PORT]" >&2
    exit 1
    ;;
esac

BIN="target/xtensa-esp32s3-espidf/release/waveshare-epd397-rust-app"
cargo +esp build --release

if [[ -n "$PORT" ]]; then
  exec espflash flash --chip esp32s3 --port "$PORT" --monitor "$BIN"
else
  exec espflash flash --chip esp32s3 --monitor "$BIN"
fi
