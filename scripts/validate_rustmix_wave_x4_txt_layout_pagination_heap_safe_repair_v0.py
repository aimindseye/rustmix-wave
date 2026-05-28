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

hal = read(ROOT / "hal-waveshare-epd397/src/lib.rs")
docs = read(ROOT / "docs/rustmix-wave/x4-txt-layout-pagination-heap-safe-repair-v0.md")
readme = read(ROOT / "README.md")

required_hal = [
    "pub const READER_LAYOUT_MAX_BOOK_BYTES: usize = 16384",
    "pub const READER_LAYOUT_MAX_LINES: usize = 192",
    "RAW-RUSTMIX-WAVE-X4-TXT-LAYOUT-PAGINATION-UTF8-OK",
    "RAW-RUSTMIX-WAVE-X4-TXT-LAYOUT-PAGINATION-WRAP-START",
    "RAW-RUSTMIX-WAVE-X4-TXT-LAYOUT-PAGINATION-WRAP-LIMIT-OK",
    "RAW-RUSTMIX-WAVE-X4-TXT-LAYOUT-PAGINATION-WRAP-DONE",
]

for item in required_hal:
    if item not in hal:
        errors.append(f"HAL missing heap-safe repair item: {item}")

if "pub const READER_LAYOUT_MAX_BOOK_BYTES: usize = 65536" in hal:
    errors.append("HAL still uses 64 KiB temporary book window")

combined = docs + "\n" + readme

required_docs = [
    "Heap-Safe Repair v0",
    "READER_LAYOUT_MAX_BOOK_BYTES = 16384",
    "READER_LAYOUT_MAX_LINES = 192",
    "streaming TXT page index/cache",
]

for item in required_docs:
    if item not in combined:
        errors.append(f"docs/README missing heap-safe text: {item}")

if errors:
    for error in errors:
        print("ERROR:", error)
    raise SystemExit(1)

print("rustmix-wave-x4-txt-layout-pagination-heap-safe-repair-v0=ok")
