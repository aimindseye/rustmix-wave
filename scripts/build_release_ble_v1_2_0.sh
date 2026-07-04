#!/usr/bin/env bash
set -euo pipefail

cd "$(dirname "$0")/.."

mkdir -p dist/releases

export PATH="$HOME/.cargo/bin:$PATH"
export RUSTFLAGS="${RUSTFLAGS:-} --cfg esp_idf_version_least_5_5_0"

SDK_BACKUP="$(mktemp)"
cp sdkconfig.defaults "$SDK_BACKUP"

restore_sdkconfig() {
  cp "$SDK_BACKUP" sdkconfig.defaults
  rm -f "$SDK_BACKUP"
}
trap restore_sdkconfig EXIT

if [ -f sdkconfig.defaults.before-rustmix-remote-ble-r1 ]; then
  cp sdkconfig.defaults.before-rustmix-remote-ble-r1 sdkconfig.defaults
fi

if [ -f sdkconfig.defaults.rustmix-remote-ble ]; then
  cat sdkconfig.defaults.rustmix-remote-ble >> sdkconfig.defaults
fi

rm -rf target/xtensa-esp32s3-espidf

cargo +esp build \
  --release \
  --target xtensa-esp32s3-espidf \
  --features rustmix-remote-ble

ELF="target/xtensa-esp32s3-espidf/release/waveshare-epd397-rust-app"
OUT="dist/releases/rustmix-wave-v1.2.0-ble.elf"

test -f "$ELF"
cp "$ELF" "$OUT"

sha256sum "$OUT" > "$OUT.sha256"

echo "Created $OUT"
cat "$OUT.sha256"
