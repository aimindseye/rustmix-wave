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
docs = read(ROOT / "docs/rustmix-wave/reader-display-surface-boundary-v0.md")
readme = read(ROOT / "README.md")

required_hal = [
    "pub mod reader_display",
    "pub trait ReaderDisplaySurface",
    "fn logical_width(&self) -> u32",
    "fn logical_height(&self) -> u32",
    "fn clear(&mut self)",
    "fn fill_rect(&mut self, x: u32, y: u32, w: u32, h: u32, black: bool)",
    "fn draw_mono_bitmap(&mut self, x: u32, y: u32, w: u32, h: u32, data: &[u8])",
    "fn flush(&mut self) -> Result<()>",
    "impl<'d> ReaderDisplaySurface for ShellDisplayBridge<'d>",
    "ShellDisplayBridge::clear_fb(self, BinaryColor::Off)",
    "ShellDisplayBridge::fill_rect(self, x, y, w, h, color)",
    "ShellDisplayBridge::set_pixel(self, x + xx, y + yy, BinaryColor::On)",
    "ShellDisplayBridge::flush(self)",
    "render_reader_display_surface_placeholder_v0",
    "RAW-RUSTMIX-WAVE-READER-BOUNDARY-V0-START",
    "RAW-RUSTMIX-WAVE-READER-DISPLAY-PLACEHOLDER-OK",
    "RAW-RUSTMIX-WAVE-READER-BOUNDARY-V0-OK",
]

for item in required_hal:
    if item not in hal:
        errors.append(f"HAL missing reader display boundary item: {item}")

required_target = [
    "reader_display::render_reader_display_surface_placeholder_v0",
    "render_reader_display_surface_placeholder_v0(&mut shell_display)",
    "RAW-RUSTMIX-WAVE-READER-BOUNDARY-DEMO-START",
    "RAW-RUSTMIX-WAVE-READER-BOUNDARY-DEMO-OK",
]

for item in required_target:
    if item not in target:
        errors.append(f"target missing reader display boundary item: {item}")

target_no_comments = "\n".join(line.split("//", 1)[0] for line in target.splitlines())

for forbidden in [
    "ReaderStorage",
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
        errors.append(f"target must not include reader/storage/input port in boundary v0: {forbidden}")

combined = docs + "\n" + readme

required_docs = [
    "ReaderDisplaySurface",
    "ShellDisplayBridge",
    "DisplayBackendAdapter",
    "does **not** port the reader",
    "does **not** add `ReaderStorage`",
    "does **not** enable real rotary input",
    "GPIO3",
    "480x800",
]

for item in required_docs:
    if item not in combined:
        errors.append(f"docs/README missing reader boundary text: {item}")

if "GPIO3 is EPD_BUSY" not in hal:
    errors.append("HAL must preserve GPIO3 as EPD_BUSY")

if "pub struct DisplayBackendAdapter" not in hal:
    errors.append("HAL must preserve DisplayBackendAdapter")

if "pub struct ShellDisplayBridge" not in hal:
    errors.append("HAL must preserve ShellDisplayBridge")

if errors:
    for error in errors:
        print("ERROR:", error)
    raise SystemExit(1)

print("rustmix-wave-reader-display-surface-boundary-v0=ok")
