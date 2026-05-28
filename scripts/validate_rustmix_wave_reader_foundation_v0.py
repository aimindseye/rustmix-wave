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
docs = read(ROOT / "docs/rustmix-wave/reader-foundation-v0.md")
readme = read(ROOT / "README.md")

required_hal = [
    "pub mod reader_foundation",
    "pub struct ReaderBook",
    "pub trait ReaderStorage",
    "fn list_books(&mut self) -> Result<&'static [ReaderBook]>",
    "fn read_file_chunk(",
    "fn read_state_file(",
    "fn write_state_file(",
    "pub struct MockReaderStorage",
    "impl ReaderStorage for MockReaderStorage",
    "pub struct ReaderScreenState",
    "pub selected_book_index: usize",
    "pub page_index: usize",
    "pub total_pages_placeholder: usize",
    "pub fn next_page(&mut self)",
    "pub fn previous_page(&mut self)",
    "render_reader_page_v0",
    "render_reader_foundation_flow_v0",
    "ReaderDisplaySurface",
    "RAW-RUSTMIX-WAVE-READER-FOUNDATION-V0-START",
    "RAW-RUSTMIX-WAVE-READER-MOCK-STORAGE-OK",
    "RAW-RUSTMIX-WAVE-READER-PAGE-RENDER-START",
    "RAW-RUSTMIX-WAVE-READER-PAGE-RENDER-OK",
    "RAW-RUSTMIX-WAVE-READER-MOCK-FIRST-PAGE-OK",
    "RAW-RUSTMIX-WAVE-READER-MOCK-NAV-NEXT-OK",
    "RAW-RUSTMIX-WAVE-READER-MOCK-NAV-PREV-OK",
    "RAW-RUSTMIX-WAVE-READER-FOUNDATION-V0-OK",
]

for item in required_hal:
    if item not in hal:
        errors.append(f"HAL missing reader foundation item: {item}")

required_target = [
    "reader_foundation::render_reader_foundation_flow_v0",
    "render_reader_foundation_flow_v0(&mut shell_display)",
    "RAW-RUSTMIX-WAVE-READER-FOUNDATION-DEMO-START",
    "RAW-RUSTMIX-WAVE-READER-FOUNDATION-DEMO-OK",
]

for item in required_target:
    if item not in target:
        errors.append(f"target missing reader foundation item: {item}")

target_no_comments = "\n".join(line.split("//", 1)[0] for line in target.splitlines())

for forbidden in [
    "open_reader",
    "epub",
    "bookmark",
    "progress",
    "InputRouter",
    "spawn_input_task",
    "rot_sw",
    "gpio3.degrade_input",
]:
    if forbidden in target_no_comments:
        errors.append(f"target must not include forbidden reader/input feature: {forbidden}")

combined = docs + "\n" + readme

required_docs = [
    "Reader Foundation v0",
    "ReaderStorage",
    "MockReaderStorage",
    "ReaderScreenState",
    "ReaderDisplaySurface",
    "ShellDisplayBridge",
    "DisplayBackendAdapter",
    "does not",
    "real SD",
    "EPUB",
    "GPIO3 remains reserved for EPD_BUSY",
]

for item in required_docs:
    if item not in combined:
        errors.append(f"docs/README missing reader foundation text: {item}")

if "GPIO3 is EPD_BUSY" not in hal:
    errors.append("HAL must preserve GPIO3 as EPD_BUSY")

if "pub struct DisplayBackendAdapter" not in hal:
    errors.append("HAL must preserve DisplayBackendAdapter")

if "pub struct ShellDisplayBridge" not in hal:
    errors.append("HAL must preserve ShellDisplayBridge")

if "pub trait ReaderDisplaySurface" not in hal:
    errors.append("HAL must preserve ReaderDisplaySurface")

if errors:
    for error in errors:
        print("ERROR:", error)
    raise SystemExit(1)

print("rustmix-wave-reader-foundation-v0=ok")
