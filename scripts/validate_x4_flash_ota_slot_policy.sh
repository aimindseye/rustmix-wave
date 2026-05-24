#!/usr/bin/env bash
set -euo pipefail

python3 - <<'PYVALIDATE'
from pathlib import Path
import csv
import re

root = Path.cwd()
partition_csv = root / "partitions" / "xteink_x4_standard.csv"
flash_script = root / "scripts" / "flash_x4_rustmix_app0.sh"
erase_script = root / "scripts" / "erase_x4_otadata_select_app0.sh"

if not partition_csv.exists():
    raise SystemExit("missing partitions/xteink_x4_standard.csv")

rows = []
with partition_csv.open(newline="") as handle:
    for row in csv.reader(line for line in handle if line.strip() and not line.lstrip().startswith("#")):
        rows.append([cell.strip() for cell in row])

by_name = {row[0]: row for row in rows if row}
required = {
    "otadata": ("0xe000", "0x2000"),
    "app0": ("0x10000", "0x640000"),
    "app1": ("0x650000", "0x640000"),
    "spiffs": ("0xc90000", "0x360000"),
    "coredump": ("0xff0000", "0x10000"),
}

for name, (offset, size) in required.items():
    row = by_name.get(name)
    if row is None:
        raise SystemExit(f"missing partition row: {name}")
    got_offset = row[3].lower()
    got_size = row[4].lower()
    if got_offset != offset or got_size != size:
        raise SystemExit(
            f"partition {name} changed: got offset={got_offset} size={got_size}, "
            f"expected offset={offset} size={size}"
        )

for script in [flash_script, erase_script]:
    if not script.exists():
        raise SystemExit(f"missing required script: {script}")
    text = script.read_text(encoding="utf-8")
    normalized = re.sub(r"\s+", " ", text.lower())
    if "erase-region" not in normalized:
        raise SystemExit(f"{script} must erase otadata with espflash erase-region")
    if "0xe000" not in normalized or "0x2000" not in normalized:
        raise SystemExit(f"{script} must erase the exact otadata range 0xe000 0x2000")

print("x4-flash-ota-slot-policy-ok")
PYVALIDATE
