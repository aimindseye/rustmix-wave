#!/usr/bin/env bash
set -euo pipefail

if [[ "$#" -ne 2 ]]; then
  echo "usage: $0 GENERATED_FONT_PACK_DIRECTORY_OR_ZIP SD_CARD_VOLUME" >&2
  exit 2
fi
SOURCE_INPUT="$1"
VOLUME="$2"
TEMP_ROOT=""
cleanup() {
  if [[ -n "$TEMP_ROOT" ]]; then rm -rf "$TEMP_ROOT"; fi
}
trap cleanup EXIT

if [[ -d "$SOURCE_INPUT" ]]; then
  SOURCE="$SOURCE_INPUT"
elif [[ -f "$SOURCE_INPUT" && "$SOURCE_INPUT" == *.zip ]]; then
  command -v unzip >/dev/null 2>&1 || { echo "unzip is required to install $SOURCE_INPUT" >&2; exit 1; }
  TEMP_ROOT="$(mktemp -d)"
  unzip -q -o "$SOURCE_INPUT" -d "$TEMP_ROOT"
  SOURCE="$TEMP_ROOT"
else
  echo "missing generated font-pack directory or ZIP: $SOURCE_INPUT" >&2
  exit 1
fi

TARGET="$VOLUME/RUSTMIX/FONTS"
[[ -f "$SOURCE/FONTS.TXT" ]] || { echo "missing $SOURCE/FONTS.TXT" >&2; exit 1; }
mkdir -p "$TARGET"
cp "$SOURCE/FONTS.TXT" "$TARGET/FONTS.TXT"
find "$TARGET" -maxdepth 1 -type f -name '*.RWF' -delete
find "$SOURCE" -maxdepth 1 -type f -name '*.RWF' -exec cp {} "$TARGET/" \;
echo "indic-font-pack-install=ok target=$TARGET"
