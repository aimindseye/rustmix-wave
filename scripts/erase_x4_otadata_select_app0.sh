#!/usr/bin/env bash
set -euo pipefail

PORT="${1:-${ESPFLASH_PORT:-}}"
if [ -z "$PORT" ]; then
  cat >&2 <<'EOF'
usage: scripts/erase_x4_otadata_select_app0.sh /dev/cu.usbmodemXXXX

Erase only the X4 otadata partition at 0xe000..0x10000 so the ESP-IDF
bootloader stops selecting a stale OTA app1 image and falls back to app0.
EOF
  exit 2
fi

./scripts/validate_x4_standard_partition_table_compatibility.sh

espflash erase-region \
  --chip esp32c3 \
  --port "$PORT" \
  0xe000 0x2000

echo "x4-otadata-erased-app0-selected"
