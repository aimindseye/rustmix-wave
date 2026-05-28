#!/usr/bin/env python3
from pathlib import Path
import sys

ROOT = Path(sys.argv[1]).resolve() if len(sys.argv) > 1 else Path.cwd().resolve()

errors = []

def read(path: Path) -> str:
    try:
        return path.read_text()
    except FileNotFoundError:
        errors.append(f"missing file: {path.relative_to(ROOT)}")
        return ""

def extract_read_book_fn(src: str) -> str:
    start = src.find("    fn read_reader_book_bytes_v0")
    if start < 0:
        start = src.find("fn read_reader_book_bytes_v0")

    if start < 0:
        return ""

    next_fn = src.find("    pub fn build_txt_layout_pagination_v0", start)
    if next_fn < 0:
        next_fn = src.find("pub fn build_txt_layout_pagination_v0", start)

    if next_fn < 0:
        return src[start:]

    return src[start:next_fn]

hal = read(ROOT / "hal-waveshare-epd397/src/lib.rs")
docs = read(ROOT / "docs/rustmix-wave/x4-txt-layout-pagination-bounded-read-repair-v0.md")
readme = read(ROOT / "README.md")

read_fn = extract_read_book_fn(hal)

required_hal = [
    "pub const READER_LAYOUT_MAX_BOOK_BYTES: usize = 65536",
    "RAW-RUSTMIX-WAVE-X4-TXT-LAYOUT-PAGINATION-BOUNDED-READ-START",
    "RAW-RUSTMIX-WAVE-X4-TXT-LAYOUT-PAGINATION-READ-LIMIT-OK",
    "RAW-RUSTMIX-WAVE-X4-TXT-LAYOUT-PAGINATION-READ-EOF-OK",
    "RAW-RUSTMIX-WAVE-X4-TXT-LAYOUT-PAGINATION-BOUNDED-READ-OK",
    "let remaining = READER_LAYOUT_MAX_BOOK_BYTES.saturating_sub(data.len())",
    "let take = core::cmp::min(n, remaining)",
    "data.extend_from_slice(&buf[..take])",
]

for item in required_hal:
    if item not in hal:
        errors.append(f"HAL missing bounded-read repair item: {item}")

if not read_fn:
    errors.append("could not extract read_reader_book_bytes_v0")

for stale in [
    "data.extend_from_slice(&buf[..n]);",
]:
    if stale in read_fn:
        errors.append(f"read_reader_book_bytes_v0 still contains unbounded read pattern: {stale}")

combined = docs + "\n" + readme

required_docs = [
    "Bounded Read Repair v0",
    "READER_LAYOUT_MAX_BOOK_BYTES = 65536",
    "panic_abort",
    "streaming page index/cache",
]

for item in required_docs:
    if item not in combined:
        errors.append(f"docs/README missing bounded-read text: {item}")

if errors:
    for error in errors:
        print("ERROR:", error)
    raise SystemExit(1)

print("rustmix-wave-x4-txt-layout-pagination-bounded-read-repair-v0=ok")
