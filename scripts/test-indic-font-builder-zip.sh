#!/usr/bin/env bash
set -euo pipefail
ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
TMP="$(mktemp -d)"
trap 'rm -rf "$TMP"' EXIT
mkdir -p "$TMP/sd"

node --check "$ROOT/tools/font-builder/zip_store.js"
node --check "$ROOT/tools/font-builder/app.js"
node - "$ROOT/tools/font-builder/zip_store.js" "$TMP/rustmix-indic-font-pack.zip" <<'JS'
const fs = require('fs');
const zip = require(process.argv[2]);
const out = process.argv[3];
const entries = [
  { name: 'FONTS.TXT', bytes: 'DEVANAGARI|SMALL|NSD16.RWF\nDEVANAGARI|XLARGE|NSD28.RWF\nGUJARATI|SMALL|NSG16.RWF\n' },
  { name: 'NSD16.RWF', bytes: Buffer.from('RWF1-dev-small') },
  { name: 'NSD28.RWF', bytes: Buffer.from('RWF1-dev-xlarge') },
  { name: 'NSG16.RWF', bytes: Buffer.from('RWF1-guj-small') },
  { name: 'README.TXT', bytes: 'extract and install\n' },
];
fs.writeFileSync(out, zip.storedZip(entries));
JS
python3 - "$TMP/rustmix-indic-font-pack.zip" <<'PY'
from pathlib import Path
from zipfile import ZipFile
import sys
archive = Path(sys.argv[1])
with ZipFile(archive) as z:
    names = set(z.namelist())
    expected = {'FONTS.TXT', 'NSD16.RWF', 'NSD28.RWF', 'NSG16.RWF', 'README.TXT'}
    assert names == expected, (names, expected)
    assert z.testzip() is None
PY
"$ROOT/scripts/install-indic-font-pack.sh" "$TMP/rustmix-indic-font-pack.zip" "$TMP/sd"
"$ROOT/scripts/verify-indic-font-pack.sh" "$TMP/sd"
[[ -f "$TMP/sd/RUSTMIX/FONTS/NSD16.RWF" ]]
[[ -f "$TMP/sd/RUSTMIX/FONTS/NSD28.RWF" ]]
[[ -f "$TMP/sd/RUSTMIX/FONTS/NSG16.RWF" ]]
echo 'indic-font-builder-single-zip-selftest=ok'
