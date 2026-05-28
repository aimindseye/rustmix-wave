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

target = read(ROOT / "target-waveshare-epd397/src/main.rs")
hal = read(ROOT / "hal-waveshare-epd397/src/lib.rs")
docs = read(ROOT / "docs/rustmix-wave/txt-boot-flow-cleanup-v0.md")
readme = read(ROOT / "README.md")

required_target = [
    "RAW-RUSTMIX-WAVE-TXT-BOOT-FLOW-V0-START",
    "RAW-RUSTMIX-WAVE-TXT-BOOT-DISPLAY-READY-OK",
    "RAW-RUSTMIX-WAVE-TXT-BOOT-SD-MOUNT-OK",
    "RAW-RUSTMIX-WAVE-TXT-BOOT-FIRST-PAGE-OK",
    "RAW-RUSTMIX-WAVE-TXT-BOOT-FLOW-V0-OK",
    "ensure_sd_txt_sample_book_v0()",
    "SdTxtReaderStorage::new()",
    "ReaderScreenState::new(0, 0)",
    "render_reader_page_v0(&mut shell_display, &mut sd_storage, &sd_state)",
    "SdMmcHostDriver::new_4bits",
    "MountedFatfs::mount",
]

for item in required_target:
    if item not in target:
        errors.append(f"target missing TXT boot cleanup item: {item}")

target_no_comments = "\n".join(line.split("//", 1)[0] for line in target.splitlines())

for forbidden in [
    "render_rustmix_wave_home_navigation_smoke",
    "render_reader_foundation_flow_v0",
    "render_reader_display_surface_placeholder_v0",
    "RAW-RUSTMIX-WAVE-SHELL-UI-SMOKE-START",
    "RAW-RUSTMIX-WAVE-SHELL-UI-V0-START",
    "RAW-RUSTMIX-WAVE-READER-FOUNDATION-DEMO-START",
    "RAW-RUSTMIX-WAVE-READER-FOUNDATION-V0-START",
    "RAW-RUSTMIX-WAVE-DISPLAY-SMOKE-START",
    "InputRouter",
    "spawn_input_task",
    "rot_sw",
    "gpio3.degrade_input",
]:
    if forbidden in target_no_comments:
        errors.append(f"target still contains old verification or forbidden path: {forbidden}")

required_hal = [
    "pub struct DisplayBackendAdapter",
    "pub struct ShellDisplayBridge",
    "pub trait ReaderDisplaySurface",
    "pub trait ReaderStorage",
    "GPIO3 is EPD_BUSY",
]

for item in required_hal:
    if item not in hal:
        errors.append(f"HAL missing required preserved item: {item}")

combined = docs + "\n" + readme

required_docs = [
    "TXT Boot Flow Cleanup v0",
    "dashboard navigation smoke",
    "mock reader first/next/previous smoke",
    "old display verification",
    "SD TXT reader path",
    "GPIO3 remains",
    "No EPUB",
    "No bookmark persistence",
    "No progress persistence",
    "No real rotary input",
]

for item in required_docs:
    if item not in combined:
        errors.append(f"docs/README missing cleanup text: {item}")

if errors:
    for error in errors:
        print("ERROR:", error)
    raise SystemExit(1)

print("rustmix-wave-txt-boot-flow-cleanup-v0=ok")
