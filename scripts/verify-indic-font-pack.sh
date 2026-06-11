#!/usr/bin/env bash
set -euo pipefail
if [[ "$#" -ne 1 ]]; then
  echo "usage: $0 SD_CARD_VOLUME" >&2
  exit 2
fi
ROOT="$1/RUSTMIX/FONTS"
[[ -f "$ROOT/FONTS.TXT" ]] || { echo "indic-font-pack-verification=failed reason=missing-manifest" >&2; exit 1; }
while IFS='|' read -r script size file; do
  [[ -z "${script// }" || "$script" == \#* ]] && continue
  [[ -f "$ROOT/$file" ]] || { echo "indic-font-pack-verification=failed reason=missing-pack file=$file" >&2; exit 1; }
done < "$ROOT/FONTS.TXT"
echo "indic-font-pack-verification=ok root=$ROOT"
