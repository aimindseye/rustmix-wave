#!/usr/bin/env bash
set -euo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
TMP="$(mktemp -d)"
trap 'rm -rf "$TMP"' EXIT
mkdir -p "$TMP/generated" "$TMP/sd" "$TMP/sd-zip"

python3 - "$TMP/generated" "$TMP/sample.epub" <<'PY'
from pathlib import Path
from zipfile import ZipFile
import sys
root = Path(sys.argv[1])
epub = Path(sys.argv[2])
root.mkdir(parents=True, exist_ok=True)
(root / 'FONTS.TXT').write_text('DEVANAGARI|MEDIUM|NSD20.RWF\nGUJARATI|MEDIUM|NSG20.RWF\n')
(root / 'NSD20.RWF').write_bytes(b'RWF1-smoke-devanagari')
(root / 'NSG20.RWF').write_bytes(b'RWF1-smoke-gujarati')
with ZipFile(epub, 'w') as archive:
    archive.writestr('OEBPS/chapter.xhtml', '<html><body><p>श्रीकृष्ण धर्म</p><p>વાલ્મીકી કૃષ્ણ</p></body></html>')
PY

"$ROOT/scripts/install-indic-font-pack.sh" "$TMP/generated" "$TMP/sd"
"$ROOT/scripts/verify-indic-font-pack.sh" "$TMP/sd"
( cd "$TMP/generated" && zip -q "$TMP/generated.zip" FONTS.TXT NSD20.RWF NSG20.RWF )
"$ROOT/scripts/install-indic-font-pack.sh" "$TMP/generated.zip" "$TMP/sd-zip"
"$ROOT/scripts/verify-indic-font-pack.sh" "$TMP/sd-zip"
python3 "$ROOT/scripts/extract-epub-font-corpus.py" "$TMP/sample.epub" > "$TMP/corpus.txt"
grep -Fq 'श्रीकृष्ण' "$TMP/corpus.txt"
grep -Fq 'વાલ્મીકી' "$TMP/corpus.txt"
[[ -f "$TMP/sd/RUSTMIX/FONTS/FONTS.TXT" ]]
[[ -f "$TMP/sd/RUSTMIX/FONTS/NSD20.RWF" ]]
[[ -f "$TMP/sd/RUSTMIX/FONTS/NSG20.RWF" ]]
[[ -f "$TMP/sd-zip/RUSTMIX/FONTS/NSD20.RWF" ]]
[[ -f "$TMP/sd-zip/RUSTMIX/FONTS/NSG20.RWF" ]]
echo 'indic-font-pack-workflow-selftest=ok'
