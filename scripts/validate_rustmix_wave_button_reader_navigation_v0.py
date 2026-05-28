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
docs = read(ROOT / "docs/rustmix-wave/button-reader-navigation-v0.md")
readme = read(ROOT / "README.md")

required_target = [
    "PinDriver::input(pins.gpio4, Pull::Up)",
    "PinDriver::input(pins.gpio5, Pull::Up)",
    "PinDriver::input(pins.gpio6, Pull::Up)",
    "button_up.is_low()",
    "button_function.is_low()",
    "button_down.is_low()",
    "sd_state.next_page()",
    "sd_state.previous_page()",
    "render_reader_page_v0(&mut shell_display, &mut sd_storage, &sd_state)",
    "RAW-RUSTMIX-WAVE-BUTTON-NAV-V0-START",
    "RAW-RUSTMIX-WAVE-BUTTON-NAV-PINS-OK",
    "RAW-RUSTMIX-WAVE-BUTTON-NAV-READY",
    "RAW-RUSTMIX-WAVE-BUTTON-DOWN-NEXT",
    "RAW-RUSTMIX-WAVE-BUTTON-DOWN-NEXT-OK",
    "RAW-RUSTMIX-WAVE-BUTTON-UP-PREV",
    "RAW-RUSTMIX-WAVE-BUTTON-UP-PREV-OK",
    "RAW-RUSTMIX-WAVE-BUTTON-FUNCTION-REFRESH",
    "RAW-RUSTMIX-WAVE-BUTTON-FUNCTION-REFRESH-OK",
]

for item in required_target:
    if item not in target:
        errors.append(f"target missing button navigation item: {item}")

required_hal = [
    "pub const BUTTON_UP: i32 = 4",
    "pub const BUTTON_FUNCTION: i32 = 5",
    "pub const BUTTON_DOWN: i32 = 6",
    "pub const BUTTON_BOOT: i32 = 0",
    "GPIO3 is EPD_BUSY",
]

for item in required_hal:
    if item not in hal:
        errors.append(f"HAL missing button navigation item: {item}")

target_no_comments = "\n".join(line.split("//", 1)[0] for line in target.splitlines())

for forbidden in [
    "subscribe",
    "InterruptType",
    "ISR",
    "isr",
    "spawn_input_task",
    "InputRouter",
    "rot_sw",
    "gpio3.degrade_input",
    "pins.gpio0",
    "epub",
    "bookmark",
    "progress",
]:
    if forbidden in target_no_comments:
        errors.append(f"target contains forbidden feature in button nav v0: {forbidden}")

combined = docs + "\n" + readme

required_docs = [
    "Button Reader Navigation v0",
    "GPIO4 Button_Up",
    "GPIO5 Button_Function",
    "GPIO6 Button_Down",
    "active-low",
    "GPIO0 Boot is documented only",
    "GPIO3 remains reserved for EPD_BUSY",
    "No interrupts",
    "No EPUB",
    "No bookmark persistence",
    "No progress persistence",
]

for item in required_docs:
    if item not in combined:
        errors.append(f"docs/README missing button navigation text: {item}")

if errors:
    for error in errors:
        print("ERROR:", error)
    raise SystemExit(1)

print("rustmix-wave-button-reader-navigation-v0=ok")
