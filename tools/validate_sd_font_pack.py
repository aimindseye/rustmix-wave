#!/usr/bin/env python3
"""Validate a Rustmix SD font pack under /RUSTMIX/FONTS.

Manifest line format:
FONT|FONT_ID|Display Name|Script|Style|PixelSize|FILE.VFN|SizeBytes|CRC32HEX|GlyphCount
"""
from __future__ import annotations

import argparse
import binascii
from pathlib import Path
import sys

MAX_FONT_BYTES = 512 * 1024
MAX_GLYPHS = 2048
ALLOWED_SCRIPTS = {"Latin", "Devanagari", "Gujarati", "Symbols"}
ALLOWED_STYLES = {"Regular", "Bold", "Italic", "BoldItalic"}


def is_83_font_name(name: str) -> bool:
    if "." not in name:
        return False
    base, ext = name.rsplit(".", 1)
    if ext.upper() != "VFN":
        return False
    if not 1 <= len(base) <= 8:
        return False
    return all(ch.isupper() or ch.isdigit() or ch == "_" for ch in base)


def validate_manifest(font_dir: Path, require_files: bool = True, allow_empty: bool = False) -> int:
    manifest = font_dir / "MANIFEST.TXT"
    if not manifest.exists():
        print(f"missing manifest: {manifest}")
        return 1

    errors: list[str] = []
    seen_ids: set[str] = set()
    seen_files: set[str] = set()
    records = 0

    for lineno, raw in enumerate(manifest.read_text(encoding="utf-8").splitlines(), start=1):
        line = raw.strip()
        if not line or line.startswith("#"):
            continue
        parts = line.split("|")
        if len(parts) != 10:
            errors.append(f"line {lineno}: expected 10 fields, got {len(parts)}")
            continue
        kind, font_id, display, script, style, px, file_name, size_text, crc_text, glyph_text = parts
        if kind != "FONT":
            errors.append(f"line {lineno}: record kind must be FONT")
        if not (1 <= len(font_id) <= 8) or not all(c.isupper() or c.isdigit() or c == "_" for c in font_id):
            errors.append(f"line {lineno}: unsafe font id {font_id!r}")
        if font_id in seen_ids:
            errors.append(f"line {lineno}: duplicate font id {font_id}")
        seen_ids.add(font_id)
        if not display or len(display) > 32:
            errors.append(f"line {lineno}: display name must be 1..32 chars")
        if script not in ALLOWED_SCRIPTS:
            errors.append(f"line {lineno}: unsupported script {script!r}")
        if style not in ALLOWED_STYLES:
            errors.append(f"line {lineno}: unsupported style {style!r}")
        try:
            pixel_size = int(px)
            if pixel_size < 8 or pixel_size > 36:
                raise ValueError
        except ValueError:
            errors.append(f"line {lineno}: pixel size must be 8..36")
        if not is_83_font_name(file_name):
            errors.append(f"line {lineno}: unsafe VFN file name {file_name!r}")
        if file_name in seen_files:
            errors.append(f"line {lineno}: duplicate VFN file {file_name}")
        seen_files.add(file_name)
        try:
            expected_size = int(size_text)
            if expected_size <= 0 or expected_size > MAX_FONT_BYTES:
                raise ValueError
        except ValueError:
            errors.append(f"line {lineno}: size must be 1..{MAX_FONT_BYTES}")
            expected_size = -1
        if len(crc_text) != 8 or any(ch not in "0123456789abcdefABCDEF" for ch in crc_text):
            errors.append(f"line {lineno}: crc32 must be 8 hex chars")
        try:
            glyph_count = int(glyph_text)
            if glyph_count <= 0 or glyph_count > MAX_GLYPHS:
                raise ValueError
        except ValueError:
            errors.append(f"line {lineno}: glyph count must be 1..{MAX_GLYPHS}")
        font_path = font_dir / file_name
        if require_files:
            if not font_path.exists():
                errors.append(f"line {lineno}: missing font file {font_path}")
            elif expected_size > 0:
                data = font_path.read_bytes()
                if len(data) != expected_size:
                    errors.append(
                        f"line {lineno}: size mismatch for {file_name}: manifest {expected_size}, actual {len(data)}"
                    )
                actual_crc = f"{binascii.crc32(data) & 0xffffffff:08X}"
                if len(crc_text) == 8 and actual_crc.upper() != crc_text.upper():
                    errors.append(
                        f"line {lineno}: crc mismatch for {file_name}: manifest {crc_text.upper()}, actual {actual_crc}"
                    )
        records += 1

    if records == 0 and not allow_empty:
        errors.append("manifest has no FONT records")

    if errors:
        for error in errors:
            print(error)
        return 1

    mode = "with files" if require_files else "manifest only"
    print(f"SD font pack ok: {records} font record(s), {mode}")
    return 0


def main() -> int:
    parser = argparse.ArgumentParser()
    parser.add_argument("font_dir", type=Path, help="Path to /RUSTMIX/FONTS")
    parser.add_argument(
        "--manifest-only",
        action="store_true",
        help="Validate manifest syntax without requiring VFN files to exist",
    )
    parser.add_argument(
        "--allow-empty",
        action="store_true",
        help="Permit a contract/sample manifest with no FONT records",
    )
    args = parser.parse_args()
    return validate_manifest(args.font_dir, require_files=not args.manifest_only, allow_empty=args.allow_empty)


if __name__ == "__main__":
    raise SystemExit(main())
