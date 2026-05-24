#!/usr/bin/env python3
"""
Build Rustmix X4 SD font options from local font zip archives.

This tool intentionally does not ship font binaries. It consumes font archives
that the user already has locally and emits compact VFN files with VFNT headers plus manifests
for the SD card:

  /RUSTMIX/FONTS/MANIFEST.TXT
  /RUSTMIX/FONTS/UIFONTS.TXT
  /RUSTMIX/FONTS/*.VFN

Reader slot order in MANIFEST.TXT:
  SD 1 -> Charis
  SD 2 -> Bitter
  SD 3 -> Lexend Deca

UI options are staged in UIFONTS.TXT for the UI renderer integration slice.
"""

from __future__ import annotations

import argparse
import binascii
import os
import re
import shutil
import struct
import sys
import tempfile
import zipfile
from dataclasses import dataclass
from pathlib import Path
from typing import Iterable

VFNT_MAGIC = b"VFNT"
VFNT_VERSION = 1
VFNT_HEADER_LEN = 44
VFNT_GLYPH_METRICS_LEN = 16
VFNT_GLYPH_BITMAP_LEN = 16
SCRIPT_LATIN = 1
BITMAP_FORMAT_ONE_BPP = 1
ASCII_GLYPHS = list(range(32, 127))

try:
    from PIL import Image, ImageFont
except Exception as exc:  # pragma: no cover - user environment diagnostic
    Image = None
    ImageFont = None
    PIL_IMPORT_ERROR = exc
else:
    PIL_IMPORT_ERROR = None


@dataclass(frozen=True)
class FontBuildSpec:
    font_id: str
    display_name: str
    role: str
    archive: str
    source_candidates: tuple[str, ...]
    output_file: str
    pixel_size: int
    style: str = "Regular"
    script: str = "Latin"


READER_SPECS = (
    FontBuildSpec(
        font_id="CHARIS",
        display_name="Charis",
        role="reader",
        archive="charis",
        source_candidates=("Charis-Regular.ttf",),
        output_file="CHARIS18.VFN",
        pixel_size=18,
    ),
    FontBuildSpec(
        font_id="BITTER",
        display_name="Bitter",
        role="reader",
        archive="families",
        source_candidates=("Bitter/static/Bitter-Regular.ttf", "Bitter-Regular.ttf"),
        output_file="BITTER18.VFN",
        pixel_size=18,
    ),
    FontBuildSpec(
        font_id="LEXEND",
        display_name="Lexend Deca",
        role="reader",
        archive="families",
        source_candidates=("Lexend_Deca/static/LexendDeca-Regular.ttf", "LexendDeca-Regular.ttf"),
        output_file="LEXEND18.VFN",
        pixel_size=18,
    ),
)

UI_SPECS = (
    FontBuildSpec(
        font_id="INTERUI",
        display_name="Inter UI",
        role="ui",
        archive="families",
        source_candidates=("Inter/static/Inter_18pt-Regular.ttf", "Inter_18pt-Regular.ttf"),
        output_file="INTER14.VFN",
        pixel_size=14,
    ),
    FontBuildSpec(
        font_id="LEXUI",
        display_name="Lexend Deca UI",
        role="ui",
        archive="families",
        source_candidates=("Lexend_Deca/static/LexendDeca-Regular.ttf", "LexendDeca-Regular.ttf"),
        output_file="LEXUI14.VFN",
        pixel_size=14,
    ),
)


def safe_extract_zip(zip_path: Path, dest: Path) -> None:
    with zipfile.ZipFile(zip_path) as zf:
        for member in zf.infolist():
            name = member.filename
            if name.startswith("__MACOSX/") or "/__MACOSX/" in name:
                continue
            target = dest / name
            resolved = target.resolve()
            if not str(resolved).startswith(str(dest.resolve()) + os.sep):
                raise ValueError(f"unsafe zip member path: {name}")
            if member.is_dir():
                target.mkdir(parents=True, exist_ok=True)
            else:
                target.parent.mkdir(parents=True, exist_ok=True)
                with zf.open(member) as src, target.open("wb") as out:
                    shutil.copyfileobj(src, out)


def find_font(root: Path, candidates: Iterable[str]) -> Path:
    normalized_files: dict[str, Path] = {}
    for path in root.rglob("*.ttf"):
        if "/__MACOSX/" in str(path):
            continue
        rel = path.relative_to(root).as_posix()
        normalized_files[rel.lower()] = path
        normalized_files[path.name.lower()] = path

    for candidate in candidates:
        key = candidate.lower()
        if key in normalized_files:
            return normalized_files[key]

    wanted = ", ".join(candidates)
    raise FileNotFoundError(f"could not find font source: {wanted}")


def pack_bits(mask_rows: list[list[int]], width: int, height: int) -> tuple[bytes, int]:
    if width <= 0 or height <= 0:
        return b"", 0
    row_stride = (width + 7) // 8
    out = bytearray()
    for y in range(height):
        row = mask_rows[y]
        for byte_idx in range(row_stride):
            value = 0
            for bit in range(8):
                x = byte_idx * 8 + bit
                if x < width and row[x]:
                    value |= 1 << (7 - bit)
            out.append(value)
    return bytes(out), row_stride


def render_glyph(font, glyph_id: int):
    char = chr(glyph_id)
    # Use a generous canvas so accents and bearings are not clipped.
    bbox = font.getbbox(char)
    if bbox is None:
        bbox = (0, 0, 0, 0)
    left, top, right, bottom = bbox
    width = max(0, int(right - left))
    height = max(0, int(bottom - top))
    advance = int(round(font.getlength(char)))

    if width == 0 or height == 0:
        return {
            "glyph_id": glyph_id,
            "advance_x": max(advance, 1 if char == " " else 0),
            "advance_y": 0,
            "bearing_x": int(left),
            "bearing_y": int(-top),
            "width": 0,
            "height": 0,
            "bitmap": b"",
            "row_stride": 0,
        }

    if Image is None:
        raise RuntimeError("Pillow is not available")
    image = Image.new("L", (width, height), 0)
    # Draw at negative bbox origin so the glyph lands at 0,0 in this bitmap.
    from PIL import ImageDraw

    draw = ImageDraw.Draw(image)
    draw.text((-left, -top), char, font=font, fill=255)
    pixels = image.load()
    rows: list[list[int]] = []
    for y in range(height):
        rows.append([1 if pixels[x, y] >= 96 else 0 for x in range(width)])
    bitmap, row_stride = pack_bits(rows, width, height)
    return {
        "glyph_id": glyph_id,
        "advance_x": max(advance, 0),
        "advance_y": 0,
        "bearing_x": int(left),
        "bearing_y": int(-top),
        "width": width,
        "height": height,
        "bitmap": bitmap,
        "row_stride": row_stride,
    }


def build_vfnt(ttf_path: Path, pixel_size: int) -> bytes:
    if ImageFont is None:
        raise RuntimeError(
            f"Pillow is required to build VFNT files: {PIL_IMPORT_ERROR}. "
            "Install with: python3 -m pip install pillow"
        )

    font = ImageFont.truetype(str(ttf_path), pixel_size)
    ascent, descent = font.getmetrics()
    line_height = int(ascent + descent + max(2, pixel_size // 6))

    glyphs = [render_glyph(font, glyph_id) for glyph_id in ASCII_GLYPHS]
    metrics_offset = VFNT_HEADER_LEN
    bitmap_index_offset = metrics_offset + len(glyphs) * VFNT_GLYPH_METRICS_LEN
    bitmap_data_offset = bitmap_index_offset + len(glyphs) * VFNT_GLYPH_BITMAP_LEN

    bitmap_data = bytearray()
    metrics_table = bytearray()
    bitmap_index = bytearray()

    for glyph in glyphs:
        bitmap_offset = len(bitmap_data)
        bitmap = glyph["bitmap"]
        bitmap_data.extend(bitmap)
        metrics_table.extend(
            struct.pack(
                "<IhhhhHH",
                glyph["glyph_id"],
                clamp_i16(glyph["advance_x"]),
                clamp_i16(glyph["advance_y"]),
                clamp_i16(glyph["bearing_x"]),
                clamp_i16(glyph["bearing_y"]),
                clamp_u16(glyph["width"]),
                clamp_u16(glyph["height"]),
            )
        )
        bitmap_index.extend(
            struct.pack(
                "<IIIHH",
                glyph["glyph_id"],
                bitmap_offset,
                len(bitmap),
                clamp_u16(glyph["row_stride"]),
                0,
            )
        )

    header = struct.pack(
        "<4sHHIHHhhIIIIIHH",
        VFNT_MAGIC,
        VFNT_VERSION,
        VFNT_HEADER_LEN,
        0,
        pixel_size,
        line_height,
        clamp_i16(ascent),
        clamp_i16(-descent),
        len(glyphs),
        metrics_offset,
        bitmap_index_offset,
        bitmap_data_offset,
        len(bitmap_data),
        SCRIPT_LATIN,
        BITMAP_FORMAT_ONE_BPP,
    )
    assert len(header) == VFNT_HEADER_LEN
    return header + metrics_table + bitmap_index + bytes(bitmap_data)


def clamp_i16(value: int) -> int:
    return max(-32768, min(32767, int(value)))


def clamp_u16(value: int) -> int:
    return max(0, min(65535, int(value)))


def font_row(spec: FontBuildSpec, file_path: Path, vfnt_data: bytes) -> str:
    crc = f"{binascii.crc32(vfnt_data) & 0xFFFFFFFF:08X}"
    return "|".join(
        [
            "FONT",
            spec.font_id,
            spec.display_name,
            spec.script,
            spec.style,
            str(spec.pixel_size),
            file_path.name,
            str(len(vfnt_data)),
            crc,
            str(len(ASCII_GLYPHS)),
        ]
    )


def write_readme(out: Path) -> None:
    (out / "README.TXT").write_text(
        "Rustmix SD font pack generated from local font archives.\n"
        "Reader slots in MANIFEST.TXT: SD1=Charis, SD2=Bitter, SD3=Lexend Deca.\n"
        "UI options are staged in UIFONTS.TXT for the UI font runtime slice.\n",
        encoding="utf-8",
    )


def build_pack(charis_zip: Path, families_zip: Path, out: Path, clean: bool) -> None:
    if clean and out.exists():
        shutil.rmtree(out)
    out.mkdir(parents=True, exist_ok=True)

    with tempfile.TemporaryDirectory(prefix="rustmix-fonts-") as td:
        temp = Path(td)
        charis_root = temp / "charis"
        families_root = temp / "families"
        charis_root.mkdir()
        families_root.mkdir()
        safe_extract_zip(charis_zip, charis_root)
        safe_extract_zip(families_zip, families_root)
        roots = {"charis": charis_root, "families": families_root}

        reader_rows: list[str] = []
        ui_rows: list[str] = []
        for spec in (*READER_SPECS, *UI_SPECS):
            src = find_font(roots[spec.archive], spec.source_candidates)
            vfnt = build_vfnt(src, spec.pixel_size)
            target = out / spec.output_file
            target.write_bytes(vfnt)
            row = font_row(spec, target, vfnt)
            if spec.role == "reader":
                reader_rows.append(row)
            else:
                ui_rows.append(row)
            print(f"created {target} from {src.name} ({len(vfnt)} bytes)")

        (out / "MANIFEST.TXT").write_text("\n".join(reader_rows) + "\n", encoding="utf-8")
        (out / "UIFONTS.TXT").write_text("\n".join(ui_rows) + "\n", encoding="utf-8")
        write_readme(out)
        print(f"created {out / 'MANIFEST.TXT'} ({len(reader_rows)} reader rows)")
        print(f"created {out / 'UIFONTS.TXT'} ({len(ui_rows)} UI rows)")


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--charis-zip", required=True, type=Path)
    parser.add_argument("--families-zip", required=True, type=Path)
    parser.add_argument(
        "--out",
        type=Path,
        default=Path("examples/sd-card/RUSTMIX/FONTS"),
        help="Output /RUSTMIX/FONTS folder path",
    )
    parser.add_argument("--clean", action="store_true", help="Replace output folder first")
    args = parser.parse_args()

    for path in (args.charis_zip, args.families_zip):
        if not path.exists():
            parser.error(f"missing zip: {path}")

    build_pack(args.charis_zip, args.families_zip, args.out, args.clean)
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
