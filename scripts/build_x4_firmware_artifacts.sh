#!/usr/bin/env bash
set -euo pipefail

OUT_DIR="${1:-dist/x4}"
TARGET="riscv32imc-unknown-none-elf"
PACKAGE="target-xteink-x4"
ELF="target/${TARGET}/release/${PACKAGE}"
FULL_BIN="${OUT_DIR}/rustmix-os-x4-full.bin"
ELF_OUT="${OUT_DIR}/rustmix-os-x4.elf"

mkdir -p "$OUT_DIR"

./scripts/validate_x4_standard_partition_table_compatibility.sh
cargo build -p "$PACKAGE" --release --target "$TARGET"

cp -f "$ELF" "$ELF_OUT"
cp -f partitions/xteink_x4_standard.csv "${OUT_DIR}/xteink_x4_standard.csv"
cp -f partitions/xteink_x4_standard.bin "${OUT_DIR}/xteink_x4_standard.bin"
cp -f espflash.toml "${OUT_DIR}/espflash.toml"

if command -v espflash >/dev/null 2>&1; then
  espflash save-image --chip esp32c3 "$ELF" "$FULL_BIN"
else
  echo "warning: espflash not found; skipping full flash image generation" >&2
  echo "install with: cargo install espflash --locked" >&2
fi

cat > "${OUT_DIR}/FLASHING.txt" <<'TXT'
Rustmix Xteink X4 firmware artifact
======================================

Recommended source-tree flashing:
  scripts/flash_x4_rustmix_app0.sh /dev/cu.usbmodemXXXX

Artifact flashing from this directory when rustmix-os-x4-full.bin is present:
  espflash write-bin --chip esp32c3 --port /dev/cu.usbmodemXXXX 0 rustmix-os-x4-full.bin

Alternative from source with espflash and the repo espflash.toml:
  espflash flash --monitor --chip esp32c3 --port /dev/cu.usbmodemXXXX target/riscv32imc-unknown-none-elf/release/target-xteink-x4

The artifact uses the Xteink X4 / CrossPoint-compatible 16MB partition table:
  app0: 0x10000 size 0x640000
  app1: 0x650000 size 0x640000
TXT

if command -v sha256sum >/dev/null 2>&1; then
  (cd "$OUT_DIR" && sha256sum * > SHA256SUMS.txt)
elif command -v shasum >/dev/null 2>&1; then
  (cd "$OUT_DIR" && shasum -a 256 * > SHA256SUMS.txt)
else
  echo "warning: no SHA256 tool found" >&2
fi

echo "firmware_artifacts_ready=${OUT_DIR}"
