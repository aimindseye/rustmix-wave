#!/usr/bin/env bash
set -euo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$ROOT"

TARGET="${TARGET:-riscv32imc-unknown-none-elf}"
PACKAGE="${PACKAGE:-target-xteink-x4}"
CHIP="${CHIP:-esp32c3}"
DIST="${DIST:-dist/rustmix-x4}"
ELF="target/${TARGET}/release/${PACKAGE}"
OUT_BIN="${DIST}/rustfirmware.bin"

mkdir -p "$DIST"

./scripts/validate_x4_standard_partition_table_compatibility.sh
cargo build -p "$PACKAGE" --release --target "$TARGET"

if [ ! -f "$ELF" ]; then
  echo "expected firmware ELF not found: $ELF" >&2
  exit 1
fi

cp -f "$ELF" "${DIST}/rustmix-x4.elf"
cp -f partitions/xteink_x4_standard.csv "${DIST}/xteink_x4_standard.csv"
cp -f partitions/xteink_x4_standard.bin "${DIST}/xteink_x4_standard.bin"
cp -f espflash.toml "${DIST}/espflash.toml"

# Default apps and fonts are embedded in the firmware image by the Rustmix
# first-boot SD provisioning table. The generated rustfirmware.bin therefore
# contains the firmware plus the starter app/font payloads that will be seeded
# to /RUSTMIX on first boot.
if espflash save-image --help 2>/dev/null | grep -q -- '--merge'; then
  espflash save-image --chip "$CHIP" --merge "$ELF" "$OUT_BIN"
  IMAGE_TYPE="merged-full-flash-image"
else
  espflash save-image --chip "$CHIP" "$ELF" "$OUT_BIN"
  IMAGE_TYPE="application-image-fallback"
fi

cat > "${DIST}/README-RUSTFIRMWARE.txt" <<TXT
Rustmix X4 rustfirmware.bin
===========================

Image: rustfirmware.bin
Type: ${IMAGE_TYPE}
Target: ${TARGET}
Package: ${PACKAGE}
Chip: ${CHIP}

This binary embeds Rustmix firmware plus the starter app/font payloads used by
first-boot SD provisioning. On first boot it seeds missing files under:

  /RUSTMIX
  /RUSTMIX/APPS
  /RUSTMIX/FONTS
  /RUSTMIX/SLEEP
  /RUSTMIX/CACHE
  /RUSTMIX/STATE

Use scripts/flash_x4_release_bin.sh ${OUT_BIN} /dev/ttyACM0 for first install.
Use scripts/flash_x4_rustmix_app0.sh /dev/ttyACM0 for normal app-only updates.
TXT

if command -v sha256sum >/dev/null 2>&1; then
  (cd "$DIST" && sha256sum rustfirmware.bin rustmix-x4.elf > SHA256SUMS.txt)
elif command -v shasum >/dev/null 2>&1; then
  (cd "$DIST" && shasum -a 256 rustfirmware.bin rustmix-x4.elf > SHA256SUMS.txt)
fi

ls -lh "$OUT_BIN"
echo "rustfirmware_bin=${OUT_BIN}"
