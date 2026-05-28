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
target = read(ROOT / "target-waveshare-epd397/src/main.rs")
docs = read(ROOT / "docs/rustmix-wave/real-txt-pagination-v0.md")
readme = read(ROOT / "README.md")

required_hal = [
    "pub const READER_TXT_PAGE_BYTES: usize = 720",
    "pub total_pages: usize",
    "pub page_byte_stride: usize",
    "pub fn new_with_total_pages",
    "pub fn for_txt_len",
    "pub fn next_page(&mut self)",
    "pub fn previous_page(&mut self)",
    "pub fn page_number(&self) -> usize",
    "pub fn page_offset(&self) -> usize",
    "let offset = state.page_offset()",
    "let mut buf = [0u8; READER_TXT_PAGE_BYTES]",
    "fn draw_number",
    "draw_page_label(display, state)",
    "SD TXT PAGE",
]

for item in required_hal:
    if item not in hal:
        errors.append(f"HAL missing real TXT pagination item: {item}")

for forbidden_hal in [
    "total_pages_placeholder",
    "state.page_index.saturating_mul(300)",
    "PAGE 1 / 3",
    "MOCK FIRST PAGE",
    "MOCK NEXT PAGE",
    "MOCK PREV PAGE",
]:
    if forbidden_hal in hal:
        errors.append(f"HAL still contains placeholder pagination item: {forbidden_hal}")

required_target = [
    "RAW-RUSTMIX-WAVE-TXT-PAGINATION-V0-START",
    "RAW-RUSTMIX-WAVE-TXT-PAGINATION-LEN-OK",
    "RAW-RUSTMIX-WAVE-TXT-PAGINATION-V0-OK",
    "fs::metadata(RUSTMIX_WAVE_SD_BOOK_PATH)",
    "ReaderScreenState::for_txt_len(0, txt_len)",
    "sd_state.next_page()",
    "sd_state.previous_page()",
]

for item in required_target:
    if item not in target:
        errors.append(f"target missing real TXT pagination item: {item}")

target_no_comments = "\n".join(line.split("//", 1)[0] for line in target.splitlines())

for forbidden in [
    "epub",
    "InputRouter",
    "spawn_input_task",
    "rot_sw",
    "gpio3.degrade_input",
]:
    if forbidden in target_no_comments:
        errors.append(f"target contains forbidden feature in pagination v0: {forbidden}")

combined = docs + "\n" + readme

required_docs = [
    "Real TXT Pagination v0",
    "real TXT",
    "file length",
    "real page offsets",
    "next and previous clamping",
    "real page number",
    "No EPUB",
    "No bookmark persistence",
    "No progress persistence",
]

for item in required_docs:
    if item not in combined:
        errors.append(f"docs/README missing pagination text: {item}")

if errors:
    for error in errors:
        print("ERROR:", error)
    raise SystemExit(1)

print("rustmix-wave-real-txt-pagination-v0=ok")
