#!/usr/bin/env bash
set -euo pipefail

SD="${SD:-${1:-/media/mindseye73/SD_CARD}}"

if [ ! -d "$SD" ]; then
  echo "SD mount not found: $SD" >&2
  exit 2
fi

echo "# X4 Title Cache Inspection"
echo "sd=$SD"
echo

echo "## TITLEMAP"
if [ -f "$SD/_X4/TITLEMAP.TSV" ]; then
  echo "titlemap_present=yes"
  echo "titlemap_lines=$(wc -l < "$SD/_X4/TITLEMAP.TSV" | tr -d ' ')"
  sed -n '1,80p' "$SD/_X4/TITLEMAP.TSV"
else
  echo "titlemap_present=no"
fi

echo
echo "## TITLES.BIN"
if [ -f "$SD/_X4/TITLES.BIN" ]; then
  echo "titles_bin_present=yes"
  echo "txt_title_lines=$((strings -a "$SD/_X4/TITLES.BIN" | rg -n '\.TXT|\.MD|POIROT~|THEMUR~|THESIG~|Poirot Investigates|Roger Ackroyd|Sign of the Four' || true) | wc -l | tr -d ' ')"
  echo "epub_title_lines=$((strings -a "$SD/_X4/TITLES.BIN" | rg -n '\.EPU|\.EPUB|Alice|Dracula|Sherlock|Baskervilles' || true) | wc -l | tr -d ' ')"
  echo "bad_phrase_lines=$((strings -a "$SD/_X4/TITLES.BIN" | rg -n 'most other parts|world at no cost|Project Gutenberg|produced by|transcribed by|START OF THE PROJECT GUTENBERG' || true) | wc -l | tr -d ' ')"
  strings -a "$SD/_X4/TITLES.BIN" | sed -n '1,160p'
else
  echo "titles_bin_present=no"
fi

echo
echo "marker=x4-title-cache-host-tools-ok"
