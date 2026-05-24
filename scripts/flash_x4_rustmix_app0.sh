#!/usr/bin/env bash
set -euo pipefail

PORT="${1:-${ESPFLASH_PORT:-}}"
BAUD="${ESPFLASH_BAUD:-115200}"
TARGET="target/riscv32imc-unknown-none-elf/release/target-xteink-x4"

if [ -z "$PORT" ]; then
  cat >&2 <<'EOF'
usage: scripts/flash_x4_rustmix_app0.sh /dev/cu.usbmodemXXXX

Build and flash Rustmix to the Xteink X4 app0 slot, then force the
ESP-IDF OTA selector back to app0 by erasing otadata before reset/monitor.

Use this script for normal cable flashing. It preserves the accepted
X4/CrossPoint partition table and avoids booting a stale app1 image left by
other firmware or OTA tests.
EOF
  exit 2
fi

./scripts/validate_x4_standard_partition_table_compatibility.sh
if [ -x ./scripts/validate_x4_flash_ota_slot_policy.sh ]; then
  ./scripts/validate_x4_flash_ota_slot_policy.sh
fi

cargo build -p target-xteink-x4 --release --target riscv32imc-unknown-none-elf

cat <<EOF

Flashing Rustmix to X4 app0 on $PORT.

Policy:
  app0 offset:    0x10000
  app1 offset:    0x650000
  otadata offset: 0xe000
  otadata size:   0x2000

The otadata erase is intentional. ESP-IDF OTA boot selection is stored in the
otadata partition. If another firmware selected app1, a normal espflash flash
can update app0 while the bootloader still boots app1. Erasing otadata makes
app0 the safe default for this cable-flash workflow.
EOF

espflash erase-region \
  --chip esp32c3 \
  --port "$PORT" \
  0xe000 0x2000

espflash flash \
  --chip esp32c3 \
  --baud "$BAUD" \
  --monitor \
  --before default-reset \
  --port "$PORT" \
  "$TARGET"
