#!/usr/bin/env bash
set -euo pipefail

FORCE=0
X4_REPO="${RUSTMIX_X4_FIRMWARE:-}"

usage() {
  cat >&2 <<'USAGE'
usage: scripts/install-dictionary-x4-pack.sh [--force] [--x4-repo PATH] /Volumes/YOUR_SD_CARD

Copies the complete Rustmix X4 dictionary SD pack into:
  /RUSTMIX/APPS/DICT

Use --x4-repo to copy from an existing rustmix-x4-firmware checkout. Without
that option, the script downloads the public main-branch archive temporarily.

The repository smoke pack is intentionally small. Use --force when replacing
an existing smoke or partial pack with the complete X4 dictionary pack.
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
  TMP="$(mktemp -d "${TMPDIR:-/tmp}/rustmix-x4-dict.XXXXXX")"
  ARCHIVE="$TMP/rustmix-x4-firmware-main.zip"
  echo "dictionary-x4-pack-download=starting"
  curl --fail --location --silent --show-error \
    "https://github.com/aimindseye/rustmix-x4-firmware/archive/refs/heads/main.zip" \
    --output "$ARCHIVE"
  unzip -q "$ARCHIVE" -d "$TMP"
  X4_REPO="$TMP/rustmix-x4-firmware-main"
fi

SRC="$X4_REPO/examples/sd-card/RUSTMIX/APPS/DICT"
DEST="$VOLUME/RUSTMIX/APPS/DICT"
for required in APP.TOM INDEX.TXT MAIN.LUA DATA; do
  if [[ ! -e "$SRC/$required" ]]; then
    echo "dictionary-x4-pack=failed missing=$SRC/$required" >&2
    exit 1
  fi
done

verify_full_pack_source() {
  local source="$1"
  for prefix in CAB BARN CALE; do
    if [[ ! -f "$source/DATA/$prefix.JSN" ]]; then
      echo "dictionary-x4-pack=failed incomplete-source-missing=$source/DATA/$prefix.JSN" >&2
      exit 1
    fi
    if ! grep -Fqx "$prefix|DATA/$prefix.JSN" "$source/INDEX.TXT"; then
      echo "dictionary-x4-pack=failed incomplete-source-index-row=$prefix" >&2
      exit 1
    fi
  done
}
verify_full_pack_source "$SRC"

existing_counts() {
  local target="$1"
  local rows=0 shards=0
  [[ -f "$target/INDEX.TXT" ]] && rows="$(grep -Ev '^[[:space:]]*(#|$)' "$target/INDEX.TXT" | wc -l | tr -d ' ')"
  [[ -d "$target/DATA" ]] && shards="$(find "$target/DATA" -type f -name '*.JSN' | wc -l | tr -d ' ')"
  printf 'rows=%s shards=%s' "$rows" "$shards"
}

if [[ -e "$DEST" && "$FORCE" -ne 1 ]]; then
  COUNTS="$(existing_counts "$DEST")"
  echo "dictionary-x4-pack=preserved-existing path=$DEST $COUNTS"
  echo "dictionary-x4-pack-install-hint=rerun-with-force"
  echo "rerun-with-force=--force"
  exit 0
fi

mkdir -p "$VOLUME/RUSTMIX/APPS"
rm -rf "$DEST"
cp -R "$SRC" "$DEST"

SHARDS="$(find "$DEST/DATA" -type f -name '*.JSN' | wc -l | tr -d ' ')"
ROWS="$(grep -Ev '^[[:space:]]*(#|$)' "$DEST/INDEX.TXT" | wc -l | tr -d ' ')"
echo "rustmix-wave-dictionary-x4-pack-installed=$DEST rows=$ROWS shards=$SHARDS"
echo "dictionary-x4-pack-source=$X4_REPO"
"$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)/verify-dictionary-x4-pack.sh" "$VOLUME"
