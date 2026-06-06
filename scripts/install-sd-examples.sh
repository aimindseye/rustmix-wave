#!/usr/bin/env bash
set -euo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
FORCE=0
if [[ "${1:-}" == "--force" ]]; then
  FORCE=1
  shift
fi
VOLUME="${1:-}"
if [[ -z "$VOLUME" ]]; then
  echo "usage: $0 [--force] /Volumes/YOUR_SD_CARD" >&2
  exit 1
fi

SRC="$ROOT/examples/sd-card/RUSTMIX"
DEST="$VOLUME/RUSTMIX"
mkdir -p "$DEST/SLEEP" "$DEST/BOOKS" "$DEST/READER/CACHE" "$DEST/APPS/HGRID" "$DEST/APPS/SUDOKU" "$DEST/APPS/MINES" "$DEST/APPS/TILTMAZE" "$DEST/APPS/M2048" "$DEST/APPS/SOKOBAN" "$DEST/APPS/DICT/DATA" "$DEST/APPS/CALENDAR"

copy_example() {
  local src="$1"
  local dest="$2"
  if [[ -e "$dest" && "$FORCE" -ne 1 ]]; then
    echo "preserved-existing=$dest"
    return
  fi
  cp "$src" "$dest"
  echo "installed=$dest"
}

copy_example "$SRC/WIFI.TXT.example" "$DEST/WIFI.TXT"
copy_example "$SRC/WEATHER.TXT.example" "$DEST/WEATHER.TXT"
copy_example "$SRC/ALARMS.TXT.example" "$DEST/ALARMS.TXT"
copy_example "$SRC/DISPLAY.TXT.example" "$DEST/DISPLAY.TXT"
copy_example "$SRC/SLEEP/SLEEP.BMP" "$DEST/SLEEP/SLEEP.BMP"
copy_example "$SRC/SLEEP/SLEEP01.BMP" "$DEST/SLEEP/SLEEP01.BMP"
copy_example "$SRC/BOOKS/README.TXT.example" "$DEST/BOOKS/README.TXT"
copy_example "$SRC/READER/PREFS.TXT.example" "$DEST/READER/PREFS.TXT"
copy_example "$SRC/APPS/HGRID/APP.TOM" "$DEST/APPS/HGRID/APP.TOM"
copy_example "$SRC/APPS/HGRID/MAIN.LUA" "$DEST/APPS/HGRID/MAIN.LUA"
copy_example "$SRC/APPS/SUDOKU/APP.TOM" "$DEST/APPS/SUDOKU/APP.TOM"
copy_example "$SRC/APPS/SUDOKU/MAIN.LUA" "$DEST/APPS/SUDOKU/MAIN.LUA"

copy_example "$SRC/APPS/MINES/APP.TOM" "$DEST/APPS/MINES/APP.TOM"
copy_example "$SRC/APPS/MINES/MAIN.LUA" "$DEST/APPS/MINES/MAIN.LUA"

echo "rustmix-wave-sd-examples-ready=$DEST"
echo "rustmix-wave-minesweeper-sd-sample-ready=$DEST/APPS/MINES"

copy_example "$SRC/APPS/TILTMAZE/APP.TOM" "$DEST/APPS/TILTMAZE/APP.TOM"
copy_example "$SRC/APPS/TILTMAZE/MAIN.LUA" "$DEST/APPS/TILTMAZE/MAIN.LUA"
echo "rustmix-wave-tilt-maze-sd-sample-ready=$DEST/APPS/TILTMAZE"

copy_example "$SRC/APPS/M2048/APP.TOM" "$DEST/APPS/M2048/APP.TOM"
copy_example "$SRC/APPS/M2048/MAIN.LUA" "$DEST/APPS/M2048/MAIN.LUA"
echo "rustmix-wave-motion-2048-sd-sample-ready=$DEST/APPS/M2048"
copy_example "$SRC/APPS/SOKOBAN/APP.TOM" "$DEST/APPS/SOKOBAN/APP.TOM"
copy_example "$SRC/APPS/SOKOBAN/MAIN.LUA" "$DEST/APPS/SOKOBAN/MAIN.LUA"
echo "rustmix-wave-sokoban-tilt-sd-sample-ready=$DEST/APPS/SOKOBAN"

DICT_DEST="$DEST/APPS/DICT"
if find "$DICT_DEST" -mindepth 1 -print -quit | grep -q .; then
  echo "preserved-existing-dictionary-pack=$DICT_DEST"
  echo "dictionary-smoke-pack-install=skipped-existing-use-dedicated-installer"
else
  copy_example "$SRC/APPS/DICT/APP.TOM" "$DICT_DEST/APP.TOM"
  copy_example "$SRC/APPS/DICT/MAIN.LUA" "$DICT_DEST/MAIN.LUA"
  copy_example "$SRC/APPS/DICT/INDEX.TXT" "$DICT_DEST/INDEX.TXT"
  copy_example "$SRC/APPS/DICT/README.TXT" "$DICT_DEST/README.TXT"
  copy_example "$SRC/APPS/DICT/DATA/AA.JSN" "$DICT_DEST/DATA/AA.JSN"
  copy_example "$SRC/APPS/DICT/DATA/AB.JSN" "$DICT_DEST/DATA/AB.JSN"
  copy_example "$SRC/APPS/DICT/DATA/CA.JSN" "$DICT_DEST/DATA/CA.JSN"
  echo "rustmix-wave-dictionary-sd-smoke-pack-ready=$DICT_DEST"
fi
echo "dictionary-complete-pack-helper=$ROOT/scripts/install-dictionary-x4-pack.sh"

CALENDAR_DEST="$DEST/APPS/CALENDAR"
if find "$CALENDAR_DEST" -mindepth 1 -print -quit | grep -q .; then
  echo "preserved-existing-calendar-pack=$CALENDAR_DEST"
  echo "calendar-smoke-pack-install=skipped-existing-use-dedicated-installer"
else
  copy_example "$SRC/APPS/CALENDAR/APP.TOM" "$CALENDAR_DEST/APP.TOM"
  copy_example "$SRC/APPS/CALENDAR/MAIN.LUA" "$CALENDAR_DEST/MAIN.LUA"
  copy_example "$SRC/APPS/CALENDAR/EVENTS.TXT" "$CALENDAR_DEST/EVENTS.TXT"
  copy_example "$SRC/APPS/CALENDAR/US2026.TXT" "$CALENDAR_DEST/US2026.TXT"
  copy_example "$SRC/APPS/CALENDAR/README.TXT" "$CALENDAR_DEST/README.TXT"
  rm -f "$CALENDAR_DEST/HINDU26.TXT"
  echo "rustmix-wave-calendar-sd-smoke-pack-ready=$CALENDAR_DEST hindu=excluded"
fi
echo "calendar-us-pack-helper=$ROOT/scripts/install-calendar-x4-pack.sh"

