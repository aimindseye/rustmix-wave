#!/usr/bin/env python3
from pathlib import Path
import sys

ROOT = Path(sys.argv[1]).resolve() if len(sys.argv) > 1 else Path.cwd().resolve()

hal = ROOT / "hal-waveshare-epd397" / "Cargo.toml"
target = ROOT / "target-waveshare-epd397" / "Cargo.toml"
workspace = ROOT / "Cargo.toml"
validator = ROOT / "scripts" / "validate_rustmix_wave_display_backend_import_v0.py"

for path in [hal, target, workspace, validator]:
    if not path.exists():
        raise SystemExit(f"missing {path}")

hal.write_text("""[package]
name = "hal-waveshare-epd397"
version = "0.1.0"
edition = "2021"
description = "Waveshare ESP32-S3 e-Paper 3.97 hardware support for Rustmix-Wave"

[lib]
name = "hal_waveshare_epd397"
path = "src/lib.rs"

[dependencies]
anyhow = { version = "1.0", default-features = false, features = ["std"] }
embedded-graphics = "0.8"
esp-idf-hal = "0.46"
esp-idf-sys = { version = "0.37", features = ["binstart"] }
""")

target.write_text("""[package]
name = "target-waveshare-epd397"
version = "0.1.0"
edition = "2021"
resolver = "2"
build = "build.rs"
description = "Rustmix-Wave Waveshare ESP32-S3 e-Paper 3.97 target"

[[bin]]
name = "target-waveshare-epd397"
path = "src/main.rs"
harness = false

[profile.release]
opt-level = "s"
lto = true

[profile.dev]
debug = true
opt-level = "z"

[dependencies]
anyhow = { version = "1.0", default-features = false, features = ["std"] }
embedded-graphics = "0.8"
embedded-hal = "1.0"
embedded-svc = "0.29"
esp-idf-hal = "0.46"
esp-idf-svc = { version = "0.52.1", features = ["binstart", "critical-section"] }
esp-idf-sys = { version = "0.37", features = ["binstart"] }
hal-waveshare-epd397 = { path = "../hal-waveshare-epd397" }
log = { version = "0.4", default-features = false }

[build-dependencies]
embuild = "0.33"

[features]
default = []
""")

# Keep root workspace resolver aligned.
w = workspace.read_text()
if "resolver = \"2\"" not in w:
    if "[workspace]" in w:
        lines = w.splitlines()
        out = []
        inserted = False
        for line in lines:
            out.append(line)
            if line.strip() == "[workspace]":
                out.append('resolver = "2"')
                inserted = True
        if inserted:
            w = "\n".join(out) + "\n"
    else:
        w = w.rstrip() + '\n\n[workspace]\nresolver = "2"\n'
workspace.write_text(w)

v = validator.read_text()

# Remove overly-specific 0.37.2 checks if they were added.
v = v.replace("""
for item in [
    'esp-idf-sys = "0.37.2"',
]:
    if item not in hal_cargo:
        errors.append(f"HAL Cargo.toml missing aligned dependency: {item}")

for item in [
    'esp-idf-sys = { version = "0.37.2", features = ["binstart"] }',
]:
    if item not in target_cargo:
        errors.append(f"target Cargo.toml missing aligned dependency: {item}")
""", "")

if 'hal_cargo = read(ROOT / "hal-waveshare-epd397/Cargo.toml")' not in v:
    needle = 'target = read(ROOT / "target-waveshare-epd397/src/main.rs")\n'
    v = v.replace(
        needle,
        needle + 'target_cargo = read(ROOT / "target-waveshare-epd397/Cargo.toml")\nhal_cargo = read(ROOT / "hal-waveshare-epd397/Cargo.toml")\n',
    )

check = '''
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
'''

if "required_cargo_alignment" not in v:
    marker = 'for item in required_target:\n    if item not in target:\n        errors.append(f"target missing {item}")\n'
    v = v.replace(marker, marker + check)

validator.write_text(v)

print("rustmix-wave-focus-hub-cargo-alignment-v0=ok")
