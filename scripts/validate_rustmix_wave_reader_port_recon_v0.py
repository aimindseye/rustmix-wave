#!/usr/bin/env python3
from pathlib import Path
import subprocess
import sys

ROOT = Path(sys.argv[1]).resolve() if len(sys.argv) > 1 else Path.cwd().resolve()

errors = []


def read(path: Path) -> str:
    try:
        return path.read_text()
    except FileNotFoundError:
        errors.append(f"missing file: {path.relative_to(ROOT)}")
        return ""


doc_path = ROOT / "docs" / "rustmix-wave" / "reader-port-recon-v0.md"
audit_path = ROOT / "scripts" / "audit_rustmix_wave_reader_port_recon_v0.py"
target_main_path = ROOT / "target-waveshare-epd397" / "src" / "main.rs"
hal_path = ROOT / "hal-waveshare-epd397" / "src" / "lib.rs"

for path in [doc_path, audit_path, target_main_path, hal_path]:
    if not path.exists():
        errors.append(f"missing required file: {path.relative_to(ROOT)}")

doc = read(doc_path)
audit = read(audit_path)
target_main = read(target_main_path)
hal = read(hal_path)

required_doc_text = [
    "Rustmix-Wave Reader Port Recon v0",
    "does **not** port the reader",
    "Reusable reader/core module categories",
    "X4-only assumptions to isolate",
    "ReaderDisplaySurface",
    "ShellDisplayBridge",
    "DisplayBackendAdapter",
    "ReaderStorage",
    "Proposed SD/storage compatibility path",
    "Concrete port plan",
    "First code migration recommendation",
    "No reader code is ported in this recon slice",
    "GPIO3 is `EPD_BUSY`",
]

for item in required_doc_text:
    if item not in doc:
        errors.append(f"reader recon doc missing required text: {item}")

required_audit_text = [
    "READER_PATTERNS",
    "STORAGE_PATTERNS",
    "HARDWARE_PATTERNS",
    "reader-port-recon-v0.md",
    "collect_candidates",
]

for item in required_audit_text:
    if item not in audit:
        errors.append(f"audit script missing required text: {item}")

# The Waveshare target must remain UI/display only in this recon slice.
target_no_comments = "\n".join(line.split("//", 1)[0] for line in target_main.splitlines())

for forbidden in [
    "ReaderDisplaySurface",
    "ReaderStorage",
    "open_reader",
    "reader_port",
    "epub",
    "bookmark",
    "progress",
    "spawn_input_task",
    "InputRouter",
    "rot_sw",
    "gpio3.degrade_input",
]:
    if forbidden in target_no_comments:
        errors.append(f"target main must not include reader/input migration in recon slice: {forbidden}")

if "GPIO3 is EPD_BUSY" not in hal:
    errors.append("HAL must continue documenting GPIO3 as EPD_BUSY")

if "pub struct ShellDisplayBridge" not in hal:
    errors.append("HAL must preserve ShellDisplayBridge display path")

if "pub struct DisplayBackendAdapter" not in hal:
    errors.append("HAL must preserve DisplayBackendAdapter display path")

# Re-run audit to ensure the recon doc can be regenerated.
if audit_path.exists():
    subprocess.check_call(["python3", str(audit_path), str(ROOT)])

if errors:
    for error in errors:
        print("ERROR:", error)
    raise SystemExit(1)

print("rustmix-wave-reader-port-recon-v0=ok")
