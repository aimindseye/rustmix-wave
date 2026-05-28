#!/usr/bin/env python3
from pathlib import Path
import sys
import subprocess

ROOT = Path(sys.argv[1]).resolve() if len(sys.argv) > 1 else Path.cwd().resolve()

errors = []

def read(path: Path) -> str:
    try:
        return path.read_text()
    except FileNotFoundError:
        errors.append(f"missing file: {path.relative_to(ROOT)}")
        return ""

doc = read(ROOT / "docs/rustmix-wave/x4-reader-reuse-map-v0.md")
readme = read(ROOT / "README.md")
hal = read(ROOT / "hal-waveshare-epd397/src/lib.rs")
target = read(ROOT / "target-waveshare-epd397/src/main.rs")

required_doc = [
    "Rustmix-Wave X4 Reader Reuse Map v0",
    "Reuse reader/UI/product logic. Do not reuse X4 hardware ownership.",
    "Reuse as-is",
    "Reuse with adapter",
    "Copy concepts only",
    "Do not reuse",
    "Reader layout",
    "TXT wrapping and pagination",
    "Font rendering",
    "Book browser",
    "Progress and bookmarks later",
    "Rustmix-Wave X4 Reader Layout Port v0",
    "Rustmix-Wave X4 TXT Layout Pagination Port v0",
    "Rustmix-Wave X4 Font Layer Port v0",
    "No runtime behavior change",
]

for item in required_doc:
    if item not in doc:
        errors.append(f"reuse map doc missing required text: {item}")

required_readme = [
    "Rustmix-Wave X4 Reader Reuse Map v0",
    "reuse as-is",
    "reuse with adapter",
    "copy concepts only",
    "do not reuse",
    "docs and validation only",
]

for item in required_readme:
    if item not in readme:
        errors.append(f"README missing reuse map text: {item}")

required_runtime_preserved = [
    "pub struct DisplayBackendAdapter",
    "pub struct ShellDisplayBridge",
    "pub trait ReaderDisplaySurface",
    "GPIO3 is EPD_BUSY",
]

for item in required_runtime_preserved:
    if item not in hal:
        errors.append(f"HAL no longer preserves accepted boundary: {item}")

required_target_preserved = [
    "RAW-RUSTMIX-WAVE-TXT-BROWSER-V0-START",
    "RAW-RUSTMIX-WAVE-BUTTON-NAV-READY",
    "render_reader_page_v0",
]

for item in required_target_preserved:
    if item not in target:
        errors.append(f"target no longer preserves accepted runtime item: {item}")

for forbidden in [
    "RAW-RUSTMIX-WAVE-X4-REUSE-MAP",
    "render_x4_reader",
    "x4_reader_reuse_runtime",
]:
    if forbidden in target or forbidden in hal:
        errors.append(f"reuse map must not add runtime behavior marker/code: {forbidden}")

try:
    changed = subprocess.check_output(
        ["git", "-C", str(ROOT), "diff", "--name-only"],
        text=True,
        stderr=subprocess.DEVNULL,
    ).splitlines()
except Exception:
    changed = []

allowed_prefixes = [
    "README.md",
    "docs/rustmix-wave/x4-reader-reuse-map-v0.md",
    "scripts/validate_rustmix_wave_x4_reader_reuse_map_v0.py",
    "local_patches/",
]

for path in changed:
    if not any(path == p or path.startswith(p) for p in allowed_prefixes):
        errors.append(f"runtime/source file changed in docs-only slice: {path}")

if errors:
    for error in errors:
        print("ERROR:", error)
    raise SystemExit(1)

print("rustmix-wave-x4-reader-reuse-map-v0=ok")
