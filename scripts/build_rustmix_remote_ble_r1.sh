#!/usr/bin/env bash
set -euo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$ROOT"

if ! grep -q 'CONFIG_BT_ENABLED=y' sdkconfig.defaults 2>/dev/null; then
  cat <<'MSG'
ERROR: sdkconfig.defaults does not include BLE defaults yet.

Apply them first with:

  cat sdkconfig.defaults.rustmix-remote-ble >> sdkconfig.defaults

or manually merge the BLE section.
MSG
  exit 1
fi

cargo +esp build --release --features rustmix-remote-ble
