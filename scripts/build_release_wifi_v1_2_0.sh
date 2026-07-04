#!/usr/bin/env bash
set -euo pipefail

cd "$(dirname "$0")/.."

mkdir -p dist/releases

export PATH="$HOME/.cargo/bin:$PATH"

if [ -f sdkconfig.defaults.before-rustmix-remote-ble-r1 ]; then
  cp sdkconfig.defaults.before-rustmix-remote-ble-r1 sdkconfig.defaults
fi

rm -rf target/xtensa-esp32s3-espidf

cargo +esp build \
  --release \
  --target xtensa-esp32s3-espidf

ELF="target/xtensa-esp32s3-espidf/release/waveshare-epd397-rust-app"
OUT="dist/releases/rustmix-wave-v1.2.0-wifi.elf"

test -f "$ELF"
cp "$ELF" "$OUT"

sha256sum "$OUT" > "$OUT.sha256"

echo "Created $OUT"
cat "$OUT.sha256"
