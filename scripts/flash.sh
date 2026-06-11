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

ELF="$(./scripts/resolve-built-elf.sh)"
echo "development-flash-elf=$ELF"

if [[ -n "$PORT" ]]; then
  exec espflash flash --chip esp32s3 --port "$PORT" --monitor "$ELF"
else
  exec espflash flash --chip esp32s3 --monitor "$ELF"
fi
