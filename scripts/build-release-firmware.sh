#!/usr/bin/env bash
set -euo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$ROOT"

SKIP_VALIDATE=0
if [[ "${1:-}" == "--skip-validate" ]]; then
  SKIP_VALIDATE=1
  shift
fi
if [[ "$#" -ne 0 ]]; then
  echo "usage: $0 [--skip-validate]" >&2
  exit 1
fi

if [[ "$SKIP_VALIDATE" -eq 0 ]]; then
  ./scripts/validate.sh
else
  echo 'release-firmware-validation=skipped'
fi

VERSION="$(sed -n 's/^version = "\([^"]*\)"/\1/p' Cargo.toml | head -n1)"
if [[ -z "$VERSION" ]]; then
  echo 'release-firmware-build=failed error=unable-to-determine-version' >&2
  exit 1
fi

ELF_SOURCE="$(./scripts/resolve-built-elf.sh)"
echo "release-firmware-source-elf=$ELF_SOURCE"

mkdir -p dist
PREFIX="dist/waveshare-epd397-rust-app-v${VERSION}"
ELF_OUT="${PREFIX}.elf"
FLASH_HELPER_OUT="${PREFIX}-flash-release.sh"
CHECKSUM_OUT="${PREFIX}-firmware-release.sha256"
FLASHING_OUT="${PREFIX}-FLASHING.txt"
ZIP_OUT="${PREFIX}-firmware-release.zip"

# Remove unsupported legacy raw-address artifacts from earlier release attempts.
rm -f dist/waveshare-epd397-rust-app-v*-flash.bin

cp "$ELF_SOURCE" "$ELF_OUT"
cp scripts/flash-release.sh "$FLASH_HELPER_OUT"
chmod +x "$FLASH_HELPER_OUT"

cat > "$FLASHING_OUT" <<TXT
Rustmix Wave firmware release v${VERSION}

Supported release flashing path (ELF-aware):

  ./$(basename "$FLASH_HELPER_OUT") $(basename "$ELF_OUT")

Equivalent direct command:

  espflash flash --chip esp32s3 --monitor $(basename "$ELF_OUT")

Development flashing with monitor remains available from the source tree:

  ./scripts/flash.sh monitor

SAFETY WARNING
--------------
Do not flash this release with espflash write-bin.
The write-bin command is a raw-address operation. This release bundle intentionally
ships no *-flash.bin artifact and does not define a supported raw address layout.

A merged factory-image workflow may be added later only after the bootloader,
partition-table, and application offsets have been validated on physical hardware.
TXT

sha256_file() {
  if command -v shasum >/dev/null 2>&1; then
    shasum -a 256 "$@"
  else
    sha256sum "$@"
  fi
}

sha256_file "$ELF_OUT" "$FLASH_HELPER_OUT" "$FLASHING_OUT" > "$CHECKSUM_OUT"
rm -f "$ZIP_OUT"
zip -jq "$ZIP_OUT" "$ELF_OUT" "$FLASH_HELPER_OUT" "$CHECKSUM_OUT" "$FLASHING_OUT"

echo "release-firmware-elf=$ELF_OUT"
echo "release-firmware-flash-helper=$FLASH_HELPER_OUT"
echo "release-firmware-checksums=$CHECKSUM_OUT"
echo "release-firmware-flashing=$FLASHING_OUT"
echo "release-firmware-zip=$ZIP_OUT"
echo 'release-firmware-format=elf-only'
echo 'release-firmware-build=ok'
