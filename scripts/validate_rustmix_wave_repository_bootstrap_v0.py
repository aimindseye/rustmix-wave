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

def remote_url(name: str) -> str:
    try:
        return subprocess.check_output(
            ["git", "remote", "get-url", name],
            cwd=ROOT,
            text=True,
            stderr=subprocess.STDOUT,
        ).strip()
    except subprocess.CalledProcessError:
        errors.append(f"missing git remote: {name}")
        return ""

for folder in [
    "hal-waveshare-epd397",
    "hal-waveshare-epd397/src",
    "target-waveshare-epd397",
    "target-waveshare-epd397/src",
    "docs/rustmix-wave",
    "scripts",
]:
    if not (ROOT / folder).is_dir():
        errors.append(f"missing folder: {folder}")

for file in [
    "hal-waveshare-epd397/README.md",
    "hal-waveshare-epd397/src/lib.rs",
    "target-waveshare-epd397/README.md",
    "target-waveshare-epd397/src/main.rs",
    "docs/rustmix-wave/bootstrap-v0.md",
    "docs/rustmix-wave/architecture.md",
    "docs/rustmix-wave/ui-direction.md",
    "docs/rustmix-wave/voice-layer.md",
    "docs/rustmix-wave/migration-plan.md",
    "README.md",
]:
    if not (ROOT / file).is_file():
        errors.append(f"missing file: {file}")

origin = remote_url("origin")
upstream = remote_url("rustmix-x4-upstream")

if "rustmix-wave" not in origin:
    errors.append(f"origin does not point to rustmix-wave: {origin}")

if "rustmix-x4-firmware" not in upstream:
    errors.append(f"rustmix-x4-upstream does not point to rustmix-x4-firmware: {upstream}")

readme = read(ROOT / "README.md")
docs = "\n".join(
    read(path)
    for path in [
        ROOT / "docs/rustmix-wave/bootstrap-v0.md",
        ROOT / "docs/rustmix-wave/architecture.md",
        ROOT / "docs/rustmix-wave/ui-direction.md",
        ROOT / "docs/rustmix-wave/voice-layer.md",
        ROOT / "docs/rustmix-wave/migration-plan.md",
    ]
)

combined = readme + "\n" + docs

required_text = [
    "Waveshare ESP32-S3 e-Paper 3.97",
    "Rustmix product model",
    "Focus Hub",
    "rotary-first",
    "voice",
    "hal-waveshare-epd397",
    "target-waveshare-epd397",
]

for text in required_text:
    if text.lower() not in combined.lower():
        errors.append(f"README/docs missing required text: {text}")

hal_src = read(ROOT / "hal-waveshare-epd397/src/lib.rs")
if "GPIO3 is EPD_BUSY" not in hal_src:
    errors.append("HAL skeleton must document GPIO3 as EPD_BUSY")

if errors:
    for error in errors:
        print("ERROR:", error)
    raise SystemExit(1)

print("rustmix-wave-repository-bootstrap-v0=ok")
