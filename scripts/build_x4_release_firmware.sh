#!/usr/bin/env bash
set -euo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$ROOT"

TARGET="${TARGET:-riscv32imc-unknown-none-elf}"
PACKAGE="${PACKAGE:-target-xteink-x4}"
CHIP="${CHIP:-esp32c3}"
DIST="${DIST:-dist/x4}"
ELF="target/${TARGET}/release/${PACKAGE}"

mkdir -p "$DIST"

cargo build -p "$PACKAGE" --release --target "$TARGET"

if [ ! -f "$ELF" ]; then
  echo "expected firmware ELF not found: $ELF" >&2
  echo "available release files:" >&2
  find "target/${TARGET}/release" -maxdepth 1 -type f -print >&2 || true
  exit 1
fi

cp "$ELF" "$DIST/rustmix-os-x4.elf"

# Prefer a full merged image for release/new-device installation when supported.
# Fall back to an app image, but keep the file name explicit.
if espflash save-image --help 2>/dev/null | grep -q -- '--merge'; then
  espflash save-image --chip "$CHIP" --merge "$ELF" "$DIST/firmware.bin"
  printf '%s\n' 'type=merged-full-flash-image' > "$DIST/firmware.meta"
else
  espflash save-image --chip "$CHIP" "$ELF" "$DIST/rustmix-os-x4-app.bin"
  cp "$DIST/rustmix-os-x4-app.bin" "$DIST/firmware.bin"
  printf '%s\n' 'type=application-image-fallback' > "$DIST/firmware.meta"
  printf '%s\n' 'warning=espflash does not expose --merge; firmware.bin is an app image fallback' >> "$DIST/firmware.meta"
fi

{
  echo "Rustmix X4 firmware artifact"
  echo "package=${PACKAGE}"
  echo "target=${TARGET}"
  echo "chip=${CHIP}"
  echo "built_utc=$(date -u +%Y-%m-%dT%H:%M:%SZ)"
  cat "$DIST/firmware.meta"
} > "$DIST/README-FIRMWARE.txt"

if command -v shasum >/dev/null 2>&1; then
  (cd "$DIST" && shasum -a 256 firmware.bin rustmix-os-x4.elf > SHA256SUMS)
elif command -v sha256sum >/dev/null 2>&1; then
  (cd "$DIST" && sha256sum firmware.bin rustmix-os-x4.elf > SHA256SUMS)
fi

ls -lh "$DIST"
echo 'marker=x4-release-firmware-bin-built'
