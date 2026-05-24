#!/usr/bin/env python3
from __future__ import annotations

import argparse
import re
from collections import defaultdict
from pathlib import Path


def sanitize_alias_part(text: str) -> str:
    out: list[str] = []
    for ch in text:
        c = ch.upper()
        if "A" <= c <= "Z" or "0" <= c <= "9":
            out.append(c)
    return "".join(out) or "FILE"


def friendly_title(path: Path) -> str:
    stem = path.stem
    text = re.sub(r"[_\-.]+", " ", stem)
    text = re.sub(r"\s+", " ", text).strip()
    if not text:
        text = path.stem

    small = {"a", "an", "and", "as", "at", "by", "for", "in", "of", "on", "or", "the", "to"}
    words: list[str] = []
    for i, word in enumerate(text.split(" ")):
        lower = word.lower()
        if i > 0 and lower in small:
            words.append(lower)
        else:
            words.append(lower[:1].upper() + lower[1:])
    return " ".join(words)


def build_title_map(sd: Path, include_md: bool, alias_max: int) -> list[str]:
    exts = {".txt"}
    if include_md:
        exts.add(".md")

    files = sorted(
        [p for p in sd.iterdir() if p.is_file() and p.suffix.lower() in exts],
        key=lambda p: p.name.lower(),
    )

    base_groups: dict[tuple[str, str], list[Path]] = defaultdict(list)
    for p in files:
        ext = sanitize_alias_part(p.suffix[1:])[:3]
        base6 = sanitize_alias_part(p.stem)[:6]
        base_groups[(base6, ext)].append(p)

    lines: list[str] = []
    seen: set[str] = set()

    for (base6, ext), group in sorted(base_groups.items()):
        for idx, p in enumerate(group, start=1):
            title = friendly_title(p)
            aliases = [
                p.name,
                p.name.upper(),
                f"{base6}~{idx}.{ext}",
                f"{base6.title()}~{idx}.{ext.lower()}",
            ]

            for n in range(1, alias_max + 1):
                aliases.append(f"{base6}~{n}.{ext}")

            for alias in aliases:
                key = alias.upper()
                if key in seen:
                    continue
                seen.add(key)
                lines.append(f"{alias}\t{title}\n")

    return lines


def main() -> int:
    parser = argparse.ArgumentParser(description="Generate _X4/TITLEMAP.TSV for X4 TXT/MD display names.")
    parser.add_argument("--sd", required=True, help="Mounted SD root, for example /media/mindseye73/SD_CARD")
    parser.add_argument("--out-rel", default="_X4/TITLEMAP.TSV", help="Output path relative to SD root")
    parser.add_argument("--include-md", action="store_true", default=True)
    parser.add_argument("--alias-max", type=int, default=9)
    args = parser.parse_args()

    sd = Path(args.sd)
    if not sd.is_dir():
        raise SystemExit(f"SD mount not found: {sd}")

    out_path = sd / args.out_rel
    out_path.parent.mkdir(parents=True, exist_ok=True)

    lines = build_title_map(sd, include_md=args.include_md, alias_max=args.alias_max)
    out_path.write_text("".join(lines), encoding="utf-8")

    print("# X4 TXT Title Map Generated")
    print("status=ACCEPTED")
    print(f"sd={sd}")
    print(f"out={out_path}")
    print(f"lines={len(lines)}")
    print("marker=x4-title-cache-host-tools-ok")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
