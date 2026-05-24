#!/usr/bin/env python3
"""Emergency helper: rebuild INDEX.TXT from existing DATA/*.JSN files.

Prefer rebuilding from dictionary.json with build_dictionary_sd_pack.py. This tool
only makes INDEX.TXT match files currently present in DATA, so words from missing
shards remain unavailable.
"""

from __future__ import annotations

import argparse
import json
import re
from pathlib import Path

NAME_RE = re.compile(r"^[A-Z0-9_]{1,8}$")
SHARD_LIMIT = 16 * 1024


def main() -> int:
    parser = argparse.ArgumentParser()
    parser.add_argument("dict_dir", help="Path to DICT folder")
    parser.add_argument("--write", action="store_true", help="Actually rewrite INDEX.TXT. Without this, only prints proposed rows.")
    args = parser.parse_args()

    dict_dir = Path(args.dict_dir)
    data_dir = dict_dir / "DATA"
    if not data_dir.is_dir():
        raise SystemExit(f"missing DATA folder: {data_dir}")

    rows: list[str] = []
    skipped: list[str] = []
    for path in sorted(data_dir.glob("*.JSN")):
        name = path.stem.upper()
        if not NAME_RE.fullmatch(name):
            skipped.append(f"unsafe name: {path.name}")
            continue
        size = path.stat().st_size
        if size > SHARD_LIMIT:
            skipped.append(f"too large: {path.name} = {size}")
            continue
        try:
            payload = json.loads(path.read_text(encoding="utf-8"))
        except Exception as exc:  # noqa: BLE001
            skipped.append(f"invalid JSON: {path.name}: {exc}")
            continue
        if not isinstance(payload, dict):
            skipped.append(f"not object JSON: {path.name}")
            continue
        rows.append(f"{name}|DATA/{name}.JSN")

    text = "\n".join(rows) + ("\n" if rows else "")
    if args.write:
        (dict_dir / "INDEX.TXT").write_text(text, encoding="utf-8")
        print(f"rewrote {dict_dir / 'INDEX.TXT'} with {len(rows)} rows")
    else:
        print(text, end="")
        print(f"# dry-run: {len(rows)} rows; add --write to replace INDEX.TXT")

    for item in skipped[:40]:
        print(f"warning: skipped {item}")
    if len(skipped) > 40:
        print(f"warning: skipped {len(skipped) - 40} more files")
    return 0 if rows else 1


if __name__ == "__main__":
    raise SystemExit(main())
