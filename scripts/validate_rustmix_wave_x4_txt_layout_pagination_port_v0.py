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
docs = read(ROOT / "docs/rustmix-wave/x4-txt-layout-pagination-port-v0.md")
readme = read(ROOT / "README.md")

required_hal = [
    "pub struct TxtLayoutPagination",
    "pub fn total_pages(&self) -> usize",
    "pub fn page_range(&self, page_index: usize)",
    "build_wrapped_txt_lines_v0",
    "push_wrapped_line_v0",
    "push_wrapped_word_v0",
    "is_gutenberg_start_marker_v0",
    "is_gutenberg_end_marker_v0",
    "read_reader_book_bytes_v0",
    "build_txt_layout_pagination_v0",
    "render_reader_layout_page_with_title_v0",
    "READER_LAYOUT_BODY_MAX_CHARS",
    "READER_LAYOUT_LINES_PER_PAGE",
    "RAW-RUSTMIX-WAVE-X4-TXT-LAYOUT-PAGINATION-V0-START",
    "RAW-RUSTMIX-WAVE-X4-TXT-LAYOUT-PAGINATION-READ-OK",
    "RAW-RUSTMIX-WAVE-X4-TXT-LAYOUT-PAGINATION-WRAP-OK",
    "RAW-RUSTMIX-WAVE-X4-TXT-LAYOUT-PAGINATION-V0-OK",
    "RAW-RUSTMIX-WAVE-X4-TXT-LAYOUT-PAGE-RENDER-START",
    "RAW-RUSTMIX-WAVE-X4-TXT-LAYOUT-PAGE-RENDER-OK",
]

for item in required_hal:
    if item not in hal:
        errors.append(f"HAL missing X4 TXT layout pagination item: {item}")

required_target = [
    "TxtLayoutPagination",
    "let mut txt_layout = None::<TxtLayoutPagination>",
    "build_txt_layout_pagination_v0(&mut sd_storage, 0)",
    "layout.total_pages()",
    "ReaderScreenState::new_with_total_pages(0, 0, total_pages)",
    "render_reader_layout_page_with_title_v0",
    "RAW-RUSTMIX-WAVE-X4-TXT-LAYOUT-PAGINATION-TARGET-OK",
    "sd_state.next_page()",
    "sd_state.previous_page()",
]

for item in required_target:
    if item not in target:
        errors.append(f"target missing X4 TXT layout pagination item: {item}")

target_no_comments = "\n".join(line.split("//", 1)[0] for line in target.splitlines())

for forbidden in [
    "epub",
    "InputRouter",
    "spawn_input_task",
    "rot_sw",
    "gpio3.degrade_input",
]:
    if forbidden in target_no_comments:
        errors.append(f"target contains forbidden feature in TXT layout pagination v0: {forbidden}")

combined = docs + "\n" + readme

required_docs = [
    "X4 TXT Layout Pagination Port v0",
    "line/page pagination",
    "word wrapping",
    "Project Gutenberg",
    "real total pages",
    "No EPUB",
    "No bookmark persistence",
    "No progress persistence",
]

for item in required_docs:
    if item not in combined:
        errors.append(f"docs/README missing TXT layout pagination text: {item}")

if errors:
    for error in errors:
        print("ERROR:", error)
    raise SystemExit(1)

print("rustmix-wave-x4-txt-layout-pagination-port-v0=ok")
