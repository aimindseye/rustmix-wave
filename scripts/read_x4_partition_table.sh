#!/usr/bin/env bash
set -euo pipefail

PORT="${1:-${ESPFLASH_PORT:-}}"
OUT="${2:-/tmp/x4-partition-table.bin}"
if [ -z "$PORT" ]; then
  cat >&2 <<'EOF'
usage: scripts/read_x4_partition_table.sh /dev/cu.usbmodemXXXX [/tmp/x4-partition-table.bin]
EOF
  exit 2
fi

espflash read-flash --chip esp32c3 --port "$PORT" 0x8000 0x1000 "$OUT"
echo "read partition table to $OUT"
echo "Expected Rustmix X4/CrossPoint-compatible table is partitions/xteink_x4_standard.bin (0xC00 bytes plus erased padding)."
