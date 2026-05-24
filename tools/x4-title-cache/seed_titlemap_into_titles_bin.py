#!/usr/bin/env python3
from __future__ import annotations

import argparse
from pathlib import Path


BAD_PHRASES = (
    "most other parts",
    "world at no cost",
    "project gutenberg",
    "produced by",
    "transcribed by",
    "start of the project gutenberg",
)


def load_existing_titles(path: Path) -> tuple[list[str], set[str]]:
    lines: list[str] = []
    keys: set[str] = set()

    if not path.exists():
        return lines, keys

    for raw in path.read_text(encoding="utf-8", errors="ignore").splitlines():
        if not raw.strip() or "\t" not in raw:
            continue
        key, title = raw.split("\t", 1)
        key = key.strip()
        title = title.strip()
        if not key or not title:
            continue
        joined = f"{key}\t{title}"
        if any(phrase in joined.lower() for phrase in BAD_PHRASES):
            continue
        lines.append(joined)
        keys.add(key.upper())

    return lines, keys


def main() -> int:
    parser = argparse.ArgumentParser(description="Seed TXT/MD mappings from _X4/TITLEMAP.TSV into _X4/TITLES.BIN.")
    parser.add_argument("--sd", required=True, help="Mounted SD root")
    parser.add_argument("--titlemap-rel", default="_X4/TITLEMAP.TSV")
    parser.add_argument("--titles-rel", default="_X4/TITLES.BIN")
    args = parser.parse_args()

    sd = Path(args.sd)
    titlemap = sd / args.titlemap_rel
    titles = sd / args.titles_rel

    if not sd.is_dir():
        raise SystemExit(f"SD mount not found: {sd}")
    if not titlemap.is_file():
        raise SystemExit(f"TITLEMAP missing: {titlemap}")

    titles.parent.mkdir(parents=True, exist_ok=True)

    existing, keys = load_existing_titles(titles)

    added = 0
    for raw in titlemap.read_text(encoding="utf-8", errors="ignore").splitlines():
        if not raw.strip() or "\t" not in raw:
            continue

        key, title = raw.split("\t", 1)
        key = key.strip()
        title = title.strip()
        if not key or not title:
            continue

        upper_key = key.upper()
        if not (upper_key.endswith(".TXT") or upper_key.endswith(".MD")):
            continue
        if any(phrase in title.lower() for phrase in BAD_PHRASES):
            continue
        if upper_key in keys:
            continue

        existing.append(f"{key}\t{title}")
        keys.add(upper_key)
        added += 1

    titles.write_text("\n".join(existing) + "\n", encoding="utf-8")

    print("# X4 TITLES.BIN Seeded")
    print("status=ACCEPTED")
    print(f"sd={sd}")
    print(f"titlemap={titlemap}")
    print(f"titles={titles}")
    print(f"added={added}")
    print(f"total={len(existing)}")
    print("marker=x4-title-cache-host-tools-ok")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
