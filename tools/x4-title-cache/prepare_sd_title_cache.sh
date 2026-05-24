#!/usr/bin/env bash
set -euo pipefail

SD="${SD:-${1:-/media/mindseye73/SD_CARD}}"
SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"

if [ ! -d "$SD" ]; then
  echo "SD mount not found: $SD" >&2
  exit 2
fi

python3 "$SCRIPT_DIR/generate_title_map.py" --sd "$SD"
python3 "$SCRIPT_DIR/seed_titlemap_into_titles_bin.py" --sd "$SD"
SD="$SD" "$SCRIPT_DIR/inspect_title_cache.sh"

sync
echo "x4-title-cache-host-tools-ok"
