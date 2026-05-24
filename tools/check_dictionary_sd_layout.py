#!/usr/bin/env python3
"""Validate an X4 Rustmix dictionary SD pack.

This intentionally fails when INDEX.TXT references DATA/*.JSN files that are not
present. A missing shard means words in that shard cannot be searched on device.
"""

from __future__ import annotations

import argparse
import json
import re
import sys
from pathlib import Path

SHARD_LIMIT = 16 * 1024
NAME_RE = re.compile(r"^[A-Z0-9_]{1,8}$")


def parse_rows(index_path: Path) -> list[tuple[int, str, str]]:
    rows: list[tuple[int, str, str]] = []
    for line_no, raw in enumerate(index_path.read_text(encoding="utf-8").splitlines(), start=1):
        line = raw.strip()
        if not line or line.startswith("#"):
            continue
        if "|" not in line:
            raise ValueError(f"bad INDEX.TXT line {line_no}: {line!r}")
        name, rel = [p.strip() for p in line.split("|", 1)]
        rows.append((line_no, name, rel))
    return rows


def main() -> int:
    parser = argparse.ArgumentParser()
    parser.add_argument("dict_dir", help="Path to DICT folder, usually examples/sd-card/RUSTMIX/APPS/DICT")
    args = parser.parse_args()

    dict_dir = Path(args.dict_dir)
    index_path = dict_dir / "INDEX.TXT"
    data_dir = dict_dir / "DATA"

    errors: list[str] = []
    warnings: list[str] = []

    if not dict_dir.is_dir():
        raise SystemExit(f"missing DICT folder: {dict_dir}")
    if not index_path.exists():
        raise SystemExit(f"missing INDEX.TXT: {index_path}")
    if not data_dir.is_dir():
        raise SystemExit(f"missing DATA folder: {data_dir}")

    try:
        rows = parse_rows(index_path)
    except ValueError as exc:
        raise SystemExit(str(exc)) from exc

    if not rows:
        raise SystemExit("INDEX.TXT has no shard rows")

    seen: set[str] = set()
    referenced_files: set[Path] = set()
    valid_shards = 0

    for line_no, name, rel in rows:
        if not NAME_RE.fullmatch(name):
            errors.append(f"line {line_no}: unsafe shard name {name!r}")
        if name in seen:
            errors.append(f"line {line_no}: duplicate shard row {name}")
        seen.add(name)
        if not rel.startswith("DATA/") or not rel.endswith(".JSN"):
            errors.append(f"line {line_no}: bad shard rel path {rel!r}")
            continue
        path = dict_dir / rel
        referenced_files.add(path.resolve())
        if not path.exists():
            errors.append(f"missing shard for {name}: {path}")
            continue
        size = path.stat().st_size
        if size > SHARD_LIMIT:
            errors.append(f"shard too large for X4: {path} = {size} bytes")
        try:
            payload = json.loads(path.read_text(encoding="utf-8"))
        except Exception as exc:  # noqa: BLE001 - command-line validator should report any JSON issue.
            errors.append(f"invalid JSON in {path}: {exc}")
            continue
        if not isinstance(payload, dict):
            errors.append(f"shard is not a JSON object: {path}")
            continue
        valid_shards += 1

    for path in sorted(data_dir.glob("*.JSN")):
        if path.resolve() not in referenced_files:
            warnings.append(f"unreferenced DATA shard: {path}")

    if errors:
        print("dictionary SD layout FAILED", file=sys.stderr)
        print(f"checked rows: {len(rows)}, valid shards found: {valid_shards}", file=sys.stderr)
        for err in errors[:80]:
            print(f"- {err}", file=sys.stderr)
        if len(errors) > 80:
            print(f"- ... {len(errors) - 80} more errors omitted", file=sys.stderr)
        print(file=sys.stderr)
        print("Fix: rebuild the DICT folder from dictionary.json and replace the whole folder; do not merge-copy INDEX.TXT over old DATA.", file=sys.stderr)
        return 1

    for warning in warnings[:40]:
        print(f"warning: {warning}", file=sys.stderr)
    if len(warnings) > 40:
        print(f"warning: ... {len(warnings) - 40} more unreferenced shards omitted", file=sys.stderr)

    print(f"dictionary SD layout ok: {len(rows)} index rows, {valid_shards} readable shards")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
