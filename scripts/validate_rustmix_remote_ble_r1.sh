#!/usr/bin/env bash
set -euo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$ROOT"

required=(
  "src/rustmix_remote/mod.rs"
  "src/rustmix_remote/rrbp.rs"
  "src/rustmix_remote/queue.rs"
  "src/rustmix_remote/bridge.rs"
  "src/rustmix_remote/ble_gatt.rs"
  "docs/rustmix-remote/ble-gatt-r1.md"
  "sdkconfig.defaults.rustmix-remote-ble"
)

for f in "${required[@]}"; do
  if [ ! -f "$f" ]; then
    echo "Missing required file: $f" >&2
    exit 1
  fi
done

grep -q 'rustmix-remote-ble' Cargo.toml
grep -q 'pub mod rustmix_remote;' src/lib.rs
grep -q 'RustmixRemoteBleGattService' src/main.rs
grep -q 'rustmix_remote_queue.pop' src/main.rs
grep -q 'RUSTMIX_REMOTE_SERVICE_UUID' src/rustmix_remote/ble_gatt.rs

if command -v cargo >/dev/null 2>&1; then
  echo "Running host tests for rustmix_remote module..."
  cargo test rustmix_remote --lib
else
  echo "cargo not found; static validation only"
fi

echo "Rustmix Remote BLE GATT r1 validation: OK"
