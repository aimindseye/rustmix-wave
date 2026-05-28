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
docs = read(ROOT / "docs/rustmix-wave/shell-bridge-ui-import-v0.md")
readme = read(ROOT / "README.md")

required_hal = [
    "pub struct DisplayBackendAdapter",
    "pub struct ShellDisplayBridge",
    "pub fn stroke_rect",
    "ShellDisplayBridge::stroke_rect(self, x, y, w, h, stroke, color)",
    "pub mod ui",
    "pub trait RustmixWaveHomeDisplaySurface",
    "pub struct RustmixWaveHomeItem",
    "pub struct RustmixWaveHomeState",
    "pub selected_index: usize",
    "pub footer_hint: &'static str",
    "pub voice_status: &'static str",
    "render_rustmix_wave_home_v0",
    "render_rustmix_wave_home_navigation_smoke",
    "RAW-RUSTMIX-WAVE-SHELL-UI-V0-START",
    "RAW-RUSTMIX-WAVE-UI-SELECT-READER",
    "RAW-RUSTMIX-WAVE-UI-SELECT-NETWORK",
    "RAW-RUSTMIX-WAVE-UI-SELECT-PRODUCT",
    "RAW-RUSTMIX-WAVE-UI-SELECT-VOICE",
    "RAW-RUSTMIX-WAVE-UI-SELECT-TOOLS",
    "RAW-RUSTMIX-WAVE-UI-SELECT-SYSTEM",
    "RAW-RUSTMIX-WAVE-SHELL-UI-V0-OK",
    "RUSTMIX WAVE",
    "ROTARY HOME",
    "VOICE IDLE",
]

for item in required_hal:
    if item not in hal:
        errors.append(f"HAL missing {item}")

required_target = [
    "DisplayBackendAdapter",
    "ShellDisplayBridge",
    "render_rustmix_wave_home_navigation_smoke",
    "RAW-RUSTMIX-WAVE-SHELL-UI-SMOKE-START",
    "RAW-RUSTMIX-WAVE-SHELL-UI-INIT-OK",
    "RAW-RUSTMIX-WAVE-SHELL-UI-SMOKE-OK",
    "pins.gpio3",
]

for item in required_target:
    if item not in target:
        errors.append(f"target missing {item}")

target_no_comments = "\n".join(line.split("//", 1)[0] for line in target.splitlines())

for forbidden in [
    "InputRouter",
    "spawn_input_task",
    "rot_sw",
    "gpio3.degrade_input",
    "reader port complete",
]:
    if forbidden in target_no_comments:
        errors.append(f"target appears to include forbidden input/reader functionality: {forbidden}")

for item in [
    "Does not enable real rotary input",
    "Does not use GPIO3 for input",
    "Does not port the reader",
    "DisplayBackendAdapter",
    "ShellDisplayBridge",
]:
    combined = docs + "\n" + readme
    if item.lower() not in combined.lower():
        errors.append(f"docs/README missing {item}")

if "GPIO3 is EPD_BUSY" not in hal:
    errors.append("HAL must preserve GPIO3 as EPD_BUSY")

# Ensure stroke_rect is not accidentally implemented on DisplayBackendAdapter.
adapter_impl_start = hal.find("impl<'d> DisplayBackendAdapter<'d>")
shell_struct_start = hal.find("pub struct ShellDisplayBridge")
if adapter_impl_start >= 0 and shell_struct_start >= 0:
    adapter_impl_text = hal[adapter_impl_start:shell_struct_start]
    if "pub fn stroke_rect(" in adapter_impl_text:
        errors.append("DisplayBackendAdapter must not own stroke_rect; it belongs on ShellDisplayBridge")

if errors:
    for error in errors:
        print("ERROR:", error)
    raise SystemExit(1)

print("rustmix-wave-shell-bridge-ui-import-v0=ok")
