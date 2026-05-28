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

required_files = [
    "hal-waveshare-epd397/Cargo.toml",
    "hal-waveshare-epd397/src/lib.rs",
    "target-waveshare-epd397/Cargo.toml",
    "target-waveshare-epd397/build.rs",
    "target-waveshare-epd397/sdkconfig.defaults",
    "target-waveshare-epd397/src/main.rs",
    "docs/rustmix-wave/display-backend-import-v0.md",
    "README.md",
]

for file in required_files:
    if not (ROOT / file).is_file():
        errors.append(f"missing file: {file}")

cargo = read(ROOT / "Cargo.toml")
for member in ['"hal-waveshare-epd397"', '"target-waveshare-epd397"']:
    if member not in cargo:
        errors.append(f"workspace missing member: {member}")

hal = read(ROOT / "hal-waveshare-epd397/src/lib.rs")
target = read(ROOT / "target-waveshare-epd397/src/main.rs")
target_cargo = read(ROOT / "target-waveshare-epd397/Cargo.toml")
hal_cargo = read(ROOT / "hal-waveshare-epd397/Cargo.toml")
docs = read(ROOT / "docs/rustmix-wave/display-backend-import-v0.md")
readme = read(ROOT / "README.md")
cargo_config = read(ROOT / ".cargo/config.toml")
root_cargo = read(ROOT / "Cargo.toml")

required_hal = [
    "EPD_SCLK: i32 = 11",
    "EPD_MOSI: i32 = 12",
    "EPD_CS: i32 = 10",
    "EPD_DC: i32 = 9",
    "EPD_RST: i32 = 46",
    "EPD_BUSY: i32 = 3",
    "GPIO3 is EPD_BUSY",
    "pub fn init_display_free",
    "pub fn clear_display_free",
    "pub fn write_frame_free",
    "pub fn refresh_display_free",
    "pub fn sleep_display_free",
    "pub struct DisplayBackendAdapter",
    "pub struct ShellDisplayBridge",
    "SHELL_LOGICAL_WIDTH: usize = 480",
    "SHELL_LOGICAL_HEIGHT: usize = 800",
]

for item in required_hal:
    if item not in hal:
        errors.append(f"HAL missing {item}")

required_target = [
    "pins.gpio11",
    "pins.gpio12",
    "pins.gpio10",
    "pins.gpio9",
    "pins.gpio46",
    "pins.gpio3",
    "Pull::Floating",
    "DisplayBackendAdapter::new",
    "RAW-RUSTMIX-WAVE-DISPLAY-SMOKE-START",
    "RAW-RUSTMIX-WAVE-DISPLAY-SMOKE-BLACK-OK",
    "RAW-RUSTMIX-WAVE-DISPLAY-SMOKE-WHITE-OK",
    "RAW-RUSTMIX-WAVE-DISPLAY-SMOKE-OK",
]

for item in required_target:
    if item not in target:
        errors.append(f"target missing {item}")

required_cargo_alignment = [
    (hal_cargo, 'esp-idf-hal = "0.46"', "HAL"),
    (hal_cargo, 'esp-idf-sys = { version = "0.37", features = ["binstart"] }', "HAL"),
    (target_cargo, 'esp-idf-hal = "0.46"', "target"),
    (target_cargo, 'esp-idf-svc = { version = "0.52.1", features = ["binstart", "critical-section"] }', "target"),
    (target_cargo, 'esp-idf-sys = { version = "0.37", features = ["binstart"] }', "target"),
    (target_cargo, 'embedded-graphics = "0.8"', "target"),
    (target_cargo, 'embuild = "0.33"', "target"),
]

for cargo_src, required, label in required_cargo_alignment:
    if required not in cargo_src:
        errors.append(f"{label} Cargo.toml missing Focus Hub-aligned dependency: {required}")


required_xtensa_config = [
    'target = "xtensa-esp32s3-espidf"',
    'linker = "ldproxy"',
    'build-std = ["std", "panic_abort"]',
    'MCU = "esp32s3"',
]

for item in required_xtensa_config:
    if item not in cargo_config:
        errors.append(f".cargo/config.toml missing ESP-IDF Xtensa build config: {item}")

for item in [
    "[profile.release]",
    'opt-level = "s"',
]:
    if item not in root_cargo:
        errors.append(f"workspace Cargo.toml missing root profile setting: {item}")

combined_docs = docs + "\n" + readme
for item in [
    "Waveshare ESP32-S3 e-Paper 3.97",
    "DisplayBackendAdapter",
    "ShellDisplayBridge",
    "GPIO3",
    "Does not port the Rustmix reader",
]:
    if item.lower() not in combined_docs.lower():
        errors.append(f"docs/README missing {item}")

for forbidden in [
    "reader port",
    "Reader port complete",
    "InputRouter",
    "rotary input enabled",
]:
    if forbidden in target:
        errors.append(f"target should not include forbidden functionality: {forbidden}")

if errors:
    for error in errors:
        print("ERROR:", error)
    raise SystemExit(1)

print("rustmix-wave-display-backend-import-v0=ok")
