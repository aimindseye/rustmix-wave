#!/usr/bin/env python3
"""Build a tiny prepared TXT cache for an English and Devanagari smoke book."""

from __future__ import annotations

import argparse
from pathlib import Path
import struct


VFNT_MAGIC = b"VFNT"
VFNT_VERSION = 1
VFNT_HEADER_LEN = 44
VFNT_METRICS_LEN = 16
VFNT_BITMAP_LEN = 16
VFNT_ONE_BPP = 1

VRUN_MAGIC = b"VRUN"
VRUN_VERSION = 1
VRUN_HEADER_LEN = 20

FONT_LATIN = 1
FONT_DEVANAGARI = 2


def normalized_path_key(path: str) -> str:
    return path.replace("\\", "/").lower()


def book_folder_for_path(path: str) -> str:
    value = 0x811C9DC5
    for byte in normalized_path_key(path).encode("utf-8"):
        value ^= byte
        value = (value * 0x01000193) & 0xFFFFFFFF
    return f"{value:08X}"


def glyph_bitmap(glyph_id: int, width: int, height: int) -> bytes:
    row_stride = (width + 7) // 8
    rows = bytearray(row_stride * height)
    for y in range(height):
        for x in range(width):
            border = x == 0 or y == 0 or x + 1 == width or y + 1 == height
            body = ((glyph_id + x * 5 + y * 7) % 11) in (0, 3, 7)
            if border or body:
                rows[y * row_stride + x // 8] |= 1 << (7 - (x & 7))
    return bytes(rows)


def build_vfnt(script_code: int, pixel_size: int, glyph_ids: list[int]) -> bytes:
    width = 5 if script_code == 1 else 9
    height = 7 if script_code == 1 else 12
    advance = 6 if script_code == 1 else 11
    line_height = 12 if script_code == 1 else 18
    ascent = 9 if script_code == 1 else 14
    descent = -3 if script_code == 1 else -4

    glyph_ids = sorted(set(glyph_ids))
    metrics = bytearray()
    bitmap_index = bytearray()
    bitmap_data = bytearray()
    for glyph_id in glyph_ids:
        bitmap = glyph_bitmap(glyph_id, width, height)
        offset = len(bitmap_data)
        bitmap_data.extend(bitmap)
        metrics.extend(
            struct.pack(
                "<IhhhhHH",
                glyph_id,
                advance,
                0,
                0,
                ascent,
                width,
                height,
            )
        )
        bitmap_index.extend(
            struct.pack("<IIIHH", glyph_id, offset, len(bitmap), (width + 7) // 8, 0)
        )

    metrics_offset = VFNT_HEADER_LEN
    bitmap_index_offset = metrics_offset + len(metrics)
    bitmap_data_offset = bitmap_index_offset + len(bitmap_index)
    header = struct.pack(
        "<4sHHIHHhhIIIIIHH",
        VFNT_MAGIC,
        VFNT_VERSION,
        VFNT_HEADER_LEN,
        0,
        pixel_size,
        line_height,
        ascent,
        descent,
        len(glyph_ids),
        metrics_offset,
        bitmap_index_offset,
        bitmap_data_offset,
        len(bitmap_data),
        script_code,
        VFNT_ONE_BPP,
    )
    return header + metrics + bitmap_index + bitmap_data


def build_page(text: str) -> bytes:
    records = bytearray()
    x = 0
    y = 0
    for ch in text:
        if ch == "\r":
            continue
        if ch == "\n":
            x = 0
            y += 24
            continue
        codepoint = ord(ch)
        if ch == " ":
            x += 6
            continue
        if 0x0900 <= codepoint <= 0x097F:
            font_id = FONT_DEVANAGARI
            advance = 12
        else:
            font_id = FONT_LATIN
            advance = 7
        records.extend(
            struct.pack("<IIhhhhI", font_id, codepoint, x, y, advance, 0, len(records) // 20)
        )
        x += advance
    header = struct.pack(
        "<4sHHIHHI",
        VRUN_MAGIC,
        VRUN_VERSION,
        VRUN_HEADER_LEN,
        len(records) // 20,
        480,
        800,
        0,
    )
    return header + records


def write_text(path: Path, text: str) -> None:
    path.write_text(text, encoding="utf-8", newline="\n")


def generate(book: Path, out: Path, device_path: str) -> Path:
    text = book.read_text(encoding="utf-8")
    book_id = book_folder_for_path(device_path)
    cache_dir = out / book_id
    cache_dir.mkdir(parents=True, exist_ok=True)

    latin_ids = sorted({ord(ch) for ch in text if ch not in "\r\n " and not (0x0900 <= ord(ch) <= 0x097F)})
    deva_ids = sorted({ord(ch) for ch in text if 0x0900 <= ord(ch) <= 0x097F})
    if not latin_ids or not deva_ids:
        raise SystemExit("book must contain both Latin and Devanagari characters")

    (cache_dir / "LAT18.VFN").write_bytes(build_vfnt(1, 18, latin_ids))
    (cache_dir / "DEV22.VFN").write_bytes(build_vfnt(2, 22, deva_ids))
    (cache_dir / "P000.VRN").write_bytes(build_page(text))
    write_text(cache_dir / "FONTS.IDX", "Latin=LAT18.VFN\nDevanagari=DEV22.VFN\n")
    write_text(cache_dir / "PAGES.IDX", "P000.VRN\n")
    write_text(
        cache_dir / "META.TXT",
        "\n".join(
            [
                f"book_id={book_id}",
                f"source=/{device_path.strip('/')}",
                "title=Prepared TXT Smoke",
                "page_count=1",
                "latin_font=LAT18.VFN",
                "devanagari_font=DEV22.VFN",
                "pages=PAGES.IDX",
                "",
            ]
        ),
    )
    return cache_dir


def main() -> None:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--book", type=Path, default=Path(__file__).with_name("MIXED.TXT"))
    parser.add_argument("--out", type=Path, required=True, help="FCACHE output directory")
    parser.add_argument(
        "--device-path",
        default="MIXED.TXT",
        help="TXT path as the X4 Reader sees it, used for the cache id",
    )
    args = parser.parse_args()

    cache_dir = generate(args.book, args.out, args.device_path)
    print(f"prepared cache: {cache_dir}")
    print(f"copy this directory under /FCACHE/{cache_dir.name} on the X4 SD card")


if __name__ == "__main__":
    main()
