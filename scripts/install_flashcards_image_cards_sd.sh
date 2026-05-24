#!/usr/bin/env bash
set -euo pipefail
if [ "$#" -ne 1 ]; then echo "usage: $0 /path/to/mounted/sd-card" >&2; exit 2; fi
SD="$1"
ROOT="$(cd "$(dirname "$0")/.." && pwd)"
SRC="$ROOT/sd-card/RUSTMIX/APPS/FLASHCRD"
DST="$SD/RUSTMIX/APPS/FLASHCRD"
mkdir -p "$SD/RUSTMIX/APPS"
rm -rf "$DST"
cp -R "$SRC" "$DST"
echo "installed image Flashcards app to $DST"
