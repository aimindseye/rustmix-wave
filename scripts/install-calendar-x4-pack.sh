#!/usr/bin/env bash
set -euo pipefail

FORCE=0
X4_REPO="${RUSTMIX_X4_FIRMWARE:-}"

usage() {
  cat >&2 <<'USAGE'
usage: scripts/install-calendar-x4-pack.sh [--force] [--x4-repo PATH] /Volumes/YOUR_SD_CARD

Copies the Rustmix X4 Calendar U.S. pack into:
  /RUSTMIX/APPS/CALENDAR

The native Rustmix-Wave Calendar reuses EVENTS.TXT and US2026.TXT. APP.TOM and
MAIN.LUA are copied for removable-card compatibility. HINDU26.TXT is explicitly
excluded. Use --force to replace an existing Calendar pack.
USAGE
}

while [[ $# -gt 0 ]]; do
  case "$1" in
    --force) FORCE=1; shift ;;
    --x4-repo) X4_REPO="${2:-}"; [[ -n "$X4_REPO" ]] || { usage; exit 1; }; shift 2 ;;
    --help|-h) usage; exit 0 ;;
    *) break ;;
  esac
done

VOLUME="${1:-}"
if [[ -z "$VOLUME" ]]; then usage; exit 1; fi
if [[ ! -d "$VOLUME" ]]; then echo "missing-volume=$VOLUME" >&2; exit 1; fi

TMP=""
cleanup() { [[ -z "$TMP" ]] || rm -rf "$TMP"; }
trap cleanup EXIT

if [[ -z "$X4_REPO" ]]; then
  TMP="$(mktemp -d "${TMPDIR:-/tmp}/rustmix-x4-calendar.XXXXXX")"
  ARCHIVE="$TMP/rustmix-x4-firmware-main.zip"
  echo "calendar-x4-pack-download=starting"
  curl --fail --location --silent --show-error \
    "https://github.com/aimindseye/rustmix-x4-firmware/archive/refs/heads/main.zip" \
    --output "$ARCHIVE"
  unzip -q "$ARCHIVE" -d "$TMP"
  X4_REPO="$TMP/rustmix-x4-firmware-main"
fi

SRC="$X4_REPO/examples/sd-card/RUSTMIX/APPS/CALENDAR"
DEST="$VOLUME/RUSTMIX/APPS/CALENDAR"
for required in APP.TOM EVENTS.TXT MAIN.LUA US2026.TXT; do
  if [[ ! -f "$SRC/$required" ]]; then
    echo "calendar-x4-pack=failed missing=$SRC/$required" >&2
    exit 1
  fi
done

if [[ -e "$DEST" && "$FORCE" -ne 1 ]]; then
  echo "calendar-x4-pack=preserved-existing path=$DEST"
  echo "calendar-x4-pack-install-hint=rerun-with-force"
  exit 0
fi

mkdir -p "$DEST"
rm -f "$DEST/APP.TOM" "$DEST/EVENTS.TXT" "$DEST/MAIN.LUA" "$DEST/US2026.TXT" "$DEST/HINDU26.TXT"
cp "$SRC/APP.TOM" "$DEST/APP.TOM"
cp "$SRC/EVENTS.TXT" "$DEST/EVENTS.TXT"
cp "$SRC/MAIN.LUA" "$DEST/MAIN.LUA"
cp "$SRC/US2026.TXT" "$DEST/US2026.TXT"

if [[ -e "$DEST/HINDU26.TXT" ]]; then
  echo "calendar-x4-pack=failed hindu-pack-not-excluded" >&2
  exit 1
fi
ROWS="$(grep -Ev '^[[:space:]]*(#|$)' "$DEST/US2026.TXT" | wc -l | tr -d ' ')"
PERSONAL="$(grep -Ev '^[[:space:]]*(#|$)' "$DEST/EVENTS.TXT" | wc -l | tr -d ' ')"
echo "rustmix-wave-calendar-x4-us-pack-installed=$DEST personal=$PERSONAL us=$ROWS hindu=excluded"
