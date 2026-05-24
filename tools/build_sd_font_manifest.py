#!/usr/bin/env python3
"""Build /RUSTMIX/FONTS/MANIFEST.TXT records for existing .VFN files.

This helper does not convert TTF/OTF into VFN. It creates a manifest for
prebuilt VFN files that are ready to copy to the X4 SD card.
"""
from __future__ import annotations

import argparse
import binascii
from pathlib import Path


def is_safe_vfnt(path: Path) -> bool:
    if path.suffix.upper() != ".VFN":
        return False
    base = path.stem
    return 1 <= len(base) <= 8 and all(c.isupper() or c.isdigit() or c == "_" for c in base)


def main() -> int:
    parser = argparse.ArgumentParser()
    parser.add_argument("font_dir", type=Path)
    parser.add_argument("--display-prefix", default="SD Font")
    parser.add_argument("--script", default="Latin", choices=["Latin", "Devanagari", "Gujarati", "Symbols"])
    parser.add_argument("--style", default="Regular", choices=["Regular", "Bold", "Italic", "BoldItalic"])
    parser.add_argument("--pixel-size", type=int, default=14)
    parser.add_argument("--glyph-count", type=int, default=512)
    args = parser.parse_args()

    if args.pixel_size < 8 or args.pixel_size > 36:
        raise SystemExit("pixel size must be 8..36")
    if args.glyph_count < 1 or args.glyph_count > 2048:
        raise SystemExit("glyph count must be 1..2048")

    fonts = sorted(p for p in args.font_dir.glob("*.VFN") if is_safe_vfnt(p))
    if not fonts:
        raise SystemExit("no 8.3-safe .VFN files found")

    lines = [
        "# Rustmix SD font manifest",
        "# FONT|FONT_ID|Display Name|Script|Style|PixelSize|FILE.VFN|SizeBytes|CRC32HEX|GlyphCount",
    ]
    for font in fonts:
        data = font.read_bytes()
        crc = f"{binascii.crc32(data) & 0xffffffff:08X}"
        font_id = font.stem.upper()
        display = f"{args.display_prefix} {font_id}"
        lines.append(
            f"FONT|{font_id}|{display}|{args.script}|{args.style}|{args.pixel_size}|{font.name}|{len(data)}|{crc}|{args.glyph_count}"
        )

    manifest = args.font_dir / "MANIFEST.TXT"
    manifest.write_text("\n".join(lines) + "\n", encoding="utf-8")
    print(f"wrote {manifest} ({len(fonts)} font records)")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
