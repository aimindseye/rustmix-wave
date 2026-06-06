#!/usr/bin/env bash
set -euo pipefail

VOLUME="${1:-}"
if [[ -z "$VOLUME" || ! -d "$VOLUME" ]]; then
  echo "usage: scripts/verify-dictionary-x4-pack.sh /Volumes/YOUR_SD_CARD" >&2
  exit 1
fi

DICT="$VOLUME/RUSTMIX/APPS/DICT"
INDEX="$DICT/INDEX.TXT"
if [[ ! -f "$INDEX" ]]; then
  echo "dictionary-x4-pack-verification=failed missing=$INDEX" >&2
  exit 1
fi

ROWS="$(grep -Ev '^[[:space:]]*(#|$)' "$INDEX" | wc -l | tr -d ' ')"
SHARDS=0
[[ -d "$DICT/DATA" ]] && SHARDS="$(find "$DICT/DATA" -type f -name '*.JSN' | wc -l | tr -d ' ')"

for prefix in CAB BARN CALE; do
  if ! grep -Fqx "$prefix|DATA/$prefix.JSN" "$INDEX"; then
    echo "dictionary-x4-pack-verification=failed missing-index-row=$prefix rows=$ROWS shards=$SHARDS" >&2
    exit 1
  fi
  if [[ ! -f "$DICT/DATA/$prefix.JSN" ]]; then
    echo "dictionary-x4-pack-verification=failed missing-shard=$prefix rows=$ROWS shards=$SHARDS" >&2
    exit 1
  fi
done

if ! grep -Fq '"CALENDAR"' "$DICT/DATA/CALE.JSN"; then
  echo "dictionary-x4-pack-verification=failed missing-word=CALENDAR shard=CALE.JSN" >&2
  exit 1
fi
if ! grep -Fq '"CAB"' "$DICT/DATA/CAB.JSN"; then
  echo "dictionary-x4-pack-verification=failed missing-word=CAB shard=CAB.JSN" >&2
  exit 1
fi
if ! grep -Fq '"BARN"' "$DICT/DATA/BARN.JSN"; then
  echo "dictionary-x4-pack-verification=failed missing-word=BARN shard=BARN.JSN" >&2
  exit 1
fi

echo "dictionary-x4-pack-verification=ok rows=$ROWS shards=$SHARDS probes=CALENDAR,CAB,BARN"
